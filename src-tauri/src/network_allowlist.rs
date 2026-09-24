//! Central network boundary for DenizSigner.
//!
//! Every outbound network request initiated by DenizSigner itself must pass
//! through [`require_allowed_url`]. Device-local communication (usbmuxd /
//! lockdown over USB, localhost dev server) never touches this module.
//!
//! ## Allowed (functional, documented in PRIVACY.md)
//! - Apple endpoints used internally by `isideload`
//!   (Apple-ID auth, developer portal, certificates, provisioning).
//! - User-configured Anisette server (default `ani.sidestore.io`).
//! - Explicit user-initiated IPA downloads from `github.com`
//!   (`SideStore/SideStore`, `LiveContainer/LiveContainer` releases).
//! - Apple help pages opened in the system browser
//!   (`iforgot.apple.com`, `apple.co`).
//!
//! ## Removed vs. the upstream reference project
//! - Automatic update plugin + manifest endpoint: deleted, so there are no
//!   automatic update checks against the former project's releases anymore.
//! - Download counter script (GitHub API + badge): not shipped.
//! - No analytics / telemetry / crash-reporting / remote-config hosts exist
//!   anywhere in this codebase (verified by `scripts/audit-network.ps1`).

use url::Url;

use crate::error::AppError;

/// Hosts DenizSigner itself may contact (CORE network boundary).
fn allowed_hosts() -> &'static [&'static str] {
    &[
        // Apple-ID / developer services (via isideload + help links).
        "iforgot.apple.com",
        "apple.co",
        // User-triggered IPA downloads (SideStore / LiveContainer releases).
        "github.com",
        "objects.githubusercontent.com",
        "release-assets.githubusercontent.com",
    ]
}

/// Validates a full URL against the allowlist.
///
/// `extra_hosts` covers the user-configurable Anisette server host, which is
/// validated separately by [`require_anisette_url`].
pub fn require_allowed_url(raw: &str, extra_hosts: &[&str]) -> Result<Url, AppError> {
    let url = Url::parse(raw).map_err(|e| AppError::Network(format!("Invalid URL: {e}")))?;
    if url.scheme() != "https" {
        return Err(AppError::Network(format!(
            "Blocked non-https URL: {}",
            redact_url(&url)
        )));
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let allowed = allowed_hosts()
        .iter()
        .any(|h| host == *h || host.ends_with(&format!(".{h}")))
        || extra_hosts
            .iter()
            .any(|h| host == h.to_ascii_lowercase());
    if !allowed {
        return Err(AppError::Network(format!(
            "Blocked URL host '{host}'. See PRIVACY.md for the network allowlist."
        )));
    }
    Ok(url)
}

/// Validates a user-configured Anisette server value.
///
/// Accepts `host` or `https://host[:port]`; anything else is rejected.
/// Returns the normalized base URL. Only `https` is allowed.
pub fn require_anisette_url(raw: &str) -> Result<String, AppError> {
    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("https://{raw}")
    };
    let url = Url::parse(&with_scheme)
        .map_err(|e| AppError::Network(format!("Invalid anisette server: {e}")))?;
    if url.scheme() != "https" {
        return Err(AppError::Network(
            "Anisette server must use https.".to_string(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| AppError::Network("Anisette server needs a host.".to_string()))?;
    if host.is_empty() {
        return Err(AppError::Network(
            "Anisette server needs a host.".to_string(),
        ));
    }
    Ok(format!(
        "https://{}{}",
        host,
        url.port().map(|p| format!(":{p}")).unwrap_or_default()
    ))
}

/// Redacts a URL for logging (never log query strings / tokens).
pub fn redact_url(url: &Url) -> String {
    format!(
        "{}://{}{}",
        url.scheme(),
        url.host_str().unwrap_or("?"),
        url.path()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_apple_help_and_github_releases() {
        assert!(require_allowed_url("https://github.com/SideStore/SideStore/releases/latest/download/SideStore.ipa", &[]).is_ok());
        assert!(require_allowed_url("https://iforgot.apple.com/", &[]).is_ok());
    }

    #[test]
    fn allows_configured_anisette_host_only() {
        assert!(require_allowed_url("https://ani.sidestore.io/v3/fetch", &["ani.sidestore.io"]).is_ok());
        assert!(require_allowed_url("https://ani.sidestore.io/v3/fetch", &[]).is_err());
    }

    #[test]
    fn blocks_tracking_and_plaintext() {
        for blocked in [
            "https://analytics.example.com/e",
            "https://github.com.evil.com/x",
            "http://github.com/SideStore/SideStore/releases/x.ipa",
            "https://updates.example.com/app",
        ] {
            assert!(require_allowed_url(blocked, &["ani.sidestore.io"]).is_err(), "{blocked}");
        }
    }

    #[test]
    fn validates_anisette_server() {
        assert_eq!(
            require_anisette_url("ani.sidestore.io").unwrap(),
            "https://ani.sidestore.io"
        );
        assert!(require_anisette_url("http://ani.sidestore.io").is_err());
        assert!(require_anisette_url("").is_err());
    }
}
