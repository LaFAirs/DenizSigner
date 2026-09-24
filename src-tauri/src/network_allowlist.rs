//! Central network boundary for DenizSigner.
//!
//! Every outbound internet request initiated by DenizSigner itself must pass
//! through [`require_allowed_url`] (downloads) or [`require_anisette_url`]
//! (user-configured Anisette server). Device-local communication (usbmuxd /
//! lockdown over USB, localhost dev server) never touches this module.
//!
//! ## Request inventory (all requests DenizSigner itself initiates)
//!
//! | # | Target host | HTTPS | Purpose | Code site | User action? |
//! |---|-------------|-------|---------|-----------|----------------|
//! | 1 | Apple endpoints (internal to `isideload`: GSA auth, developer portal) | yes | Apple-ID login, 2FA session, certificates, App IDs, provisioning, signing | `isideload` crate via `account.rs` / `sideload.rs` | yes (login / install click) |
//! | 2 | Configured Anisette host (default `ani.sidestore.io`) | yes, enforced | Anisette headers for Apple auth | `account.rs` (`RemoteV3AnisetteProvider::set_url`) | yes (login; server chosen in Settings) |
//! | 3 | `github.com`, `objects.githubusercontent.com`, `release-assets.githubusercontent.com` (incl. redirects between them) | yes, enforced + re-validated per redirect | SideStore / LiveContainer `.ipa` artifacts | `sideload.rs::download` | yes (explicit install click) |
//! | 4 | `iforgot.apple.com`, `apple.co` | yes | Help links, opened in the system browser (no background traffic) | locale strings via opener | yes (click) |
//!
//! There is deliberately nothing else: no analytics, no telemetry, no crash
//! reporting, no remote config, no update checks, no background polling.
//! `scripts/audit-network.ps1` (CI) verifies this; its limits are documented
//! in the script header (string search, not flow analysis).
//!
//! ## Removed vs. the upstream reference project
//! - Automatic update plugin + manifest endpoint: deleted, so there are no
//!   automatic update checks against the former project's releases anymore.
//! - Download counter script (GitHub API + badge): not shipped.

use url::Url;

use crate::error::AppError;

/// Hosts DenizSigner itself may contact (CORE network boundary).
/// Anisette hosts are NOT listed here — they are user-configured and matched
/// exactly (no suffix matching) via `extra_hosts`.
fn allowed_hosts() -> &'static [&'static str] {
    &[
        // Apple help pages (opened in the system browser).
        "iforgot.apple.com",
        "apple.co",
        // User-triggered IPA downloads (SideStore / LiveContainer releases,
        // including GitHub's CDN redirect targets).
        "github.com",
        "objects.githubusercontent.com",
        "release-assets.githubusercontent.com",
    ]
}

/// Hosts that must appear verbatim in PRIVACY.md (documentation sync).
/// CI-adjacent test `privacy_documents_all_hosts` enforces this so new hosts
/// cannot be added silently.
pub fn documented_hosts() -> &'static [&'static str] {
    &[
        "iforgot.apple.com",
        "apple.co",
        "github.com",
        "objects.githubusercontent.com",
        "release-assets.githubusercontent.com",
        "ani.sidestore.io",
    ]
}

/// Rejects raw URL strings with smuggling characteristics before parsing:
/// backslashes, whitespace/control/CRLF characters, userinfo (`@`), and
/// anything that is not an explicit `https://` prefix. This blocks
/// `http:` / `javascript:` / `file:` / `data:` schemes, `https:\\` tricks,
/// `user:password@host` credentials, and header-injection attempts.
/// Non-ASCII input is rejected as well: IDN hosts must be entered in
/// `xn--` punycode form (explicit, no invisible homoglyphs).
fn reject_suspicious_raw(raw: &str) -> Result<(), AppError> {
    if !raw.to_ascii_lowercase().starts_with("https://") {
        return Err(AppError::Network(
            "Only https:// URLs are allowed.".to_string(),
        ));
    }
    if raw.contains('@') {
        return Err(AppError::Network(
            "URLs with userinfo/credentials are blocked.".to_string(),
        ));
    }
    if !raw.is_ascii() {
        return Err(AppError::Network(
            "Non-ASCII URLs are blocked (use punycode).".to_string(),
        ));
    }
    if raw.contains(['\\', ' ', '\t', '\n', '\r']) {
        return Err(AppError::Network(
            "URL contains backslash, whitespace, or CRLF.".to_string(),
        ));
    }
    if raw.bytes().any(|b| b.is_ascii_control()) {
        return Err(AppError::Network(
            "URL contains control characters.".to_string(),
        ));
    }
    Ok(())
}

/// Normalizes a parsed host: lowercase, no trailing dot, ASCII-only
/// (punycode `xn--` labels pass; raw Unicode is rejected).
fn normalize_host(url: &Url) -> Result<String, AppError> {
    let host = url
        .host_str()
        .ok_or_else(|| AppError::Network("URL needs a host.".to_string()))?;
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return Err(AppError::Network("URL needs a host.".to_string()));
    }
    if !host.is_ascii() {
        return Err(AppError::Network(
            "Non-ASCII (non-punycode) hosts are blocked.".to_string(),
        ));
    }
    if url.username() != "" || url.password().is_some() {
        return Err(AppError::Network(
            "URLs with userinfo/credentials are blocked.".to_string(),
        ));
    }
    Ok(host)
}

/// Validates a full download URL against the allowlist.
///
/// `extra_hosts` covers the user-configured Anisette server host (exact match
/// only), validated separately by [`require_anisette_url`].
/// Query strings are permitted (CDN redirect targets use signed URLs) but are
/// never logged (see [`redact_url`]); fragments and explicit ports are
/// rejected (no legitimate use for downloads).
pub fn require_allowed_url(raw: &str, extra_hosts: &[&str]) -> Result<Url, AppError> {
    reject_suspicious_raw(raw)?;
    let url = Url::parse(raw).map_err(|e| AppError::Network(format!("Invalid URL: {e}")))?;
    if url.scheme() != "https" {
        return Err(AppError::Network(format!(
            "Blocked non-https URL: {}",
            redact_url(&url)
        )));
    }
    let host = normalize_host(&url)?;
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
    if url.fragment().is_some() {
        return Err(AppError::Network(
            "URL fragments are not accepted.".to_string(),
        ));
    }
    if url.port().is_some() {
        return Err(AppError::Network(
            "Explicit ports are not accepted for downloads.".to_string(),
        ));
    }
    Ok(url)
}

/// Validates a user-configured Anisette server value.
///
/// Accepts `host` or `https://host[:port]`; anything else is rejected.
/// Returns the normalized base URL (`https://host[:port]`, no path/query).
/// Rules: https only, no userinfo, no query, no fragment, no backslash or
/// control characters, ASCII host (punycode ok), explicit ports 1–65535 ok.
pub fn require_anisette_url(raw: &str) -> Result<String, AppError> {
    // Reject smuggling characters before any normalization or scheme prefix.
    if raw.contains(['\\', ' ', '\t', '\n', '\r'])
        || raw.bytes().any(|b| b.is_ascii_control())
        || raw.contains('@')
    {
        return Err(AppError::Network(
            "Anisette server contains rejected characters.".to_string(),
        ));
    }
    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("https://{raw}")
    };
    reject_suspicious_raw(&with_scheme)?;
    let url = Url::parse(&with_scheme)
        .map_err(|e| AppError::Network(format!("Invalid anisette server: {e}")))?;
    if url.scheme() != "https" {
        return Err(AppError::Network(
            "Anisette server must use https.".to_string(),
        ));
    }
    let host = normalize_host(&url)?;
    if url.query().is_some() || url.fragment().is_some() {
        return Err(AppError::Network(
            "Anisette server must be a bare host (no query/fragment).".to_string(),
        ));
    }
    if !url.path().is_empty() && url.path() != "/" {
        return Err(AppError::Network(
            "Anisette server must be a bare host (no path).".to_string(),
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
        assert!(require_allowed_url("https://objects.githubusercontent.com/u/12345", &[]).is_ok());
        assert!(require_allowed_url(
            "https://release-assets.githubusercontent.com/x/y?response-content-type=signed",
            &[]
        )
        .is_ok());
    }

    #[test]
    fn allows_configured_anisette_host_only() {
        assert!(require_allowed_url("https://ani.sidestore.io/v3/fetch", &["ani.sidestore.io"]).is_ok());
        assert!(require_allowed_url("https://ani.sidestore.io/v3/fetch", &[]).is_err());
        // Subdomain of the anisette host is NOT covered (exact match only).
        assert!(require_allowed_url("https://sub.ani.sidestore.io/x", &["ani.sidestore.io"]).is_err());
    }

    #[test]
    fn blocks_tracking_plaintext_and_lookalikes() {
        for blocked in [
            "https://analytics.example.com/e",
            "https://github.com.evil.com/x",
            "https://evilgithub.com/x",
            "https://trusted.example.evil.com/x",
            "https://trusted.example.com.evil.com/x",
            "http://github.com/SideStore/SideStore/releases/x.ipa",
            "https://updates.example.com/app",
            "https://github.com:8443/x",
            "https://github.com/x#frag",
        ] {
            assert!(require_allowed_url(blocked, &["ani.sidestore.io"]).is_err(), "{blocked}");
        }
    }

    #[test]
    fn blocks_smuggling_tricks() {
        for blocked in [
            "https://user:pass@github.com/x",
            "https:\\\\github.com\\x",
            "https://github.com/x\nSet-Cookie: a=b",
            "https://github.com/x\rcarriage",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/plain,hi",
            "http://github.com/x",
            "HTTPS://github.com.evil.com@github.com/x",
        ] {
            assert!(require_allowed_url(blocked, &[]).is_err(), "{blocked:?}");
        }
    }

    #[test]
    fn validates_anisette_server() {
        assert_eq!(
            require_anisette_url("ani.sidestore.io").unwrap(),
            "https://ani.sidestore.io"
        );
        assert_eq!(
            require_anisette_url("https://ANI.SIDESTORE.IO").unwrap(),
            "https://ani.sidestore.io"
        );
        assert_eq!(
            require_anisette_url("ani.example.com:8080").unwrap(),
            "https://ani.example.com:8080"
        );
        // Any host is acceptable as a *user-chosen* custom server (the UI
        // shows a trust warning); only smuggling tricks are rejected.
        assert_eq!(
            require_anisette_url("my-own.example.org").unwrap(),
            "https://my-own.example.org"
        );
        for bad in [
            "",
            "http://ani.sidestore.io",
            "https://user:pass@ani.sidestore.io",
            "https://ani.sidestore.io/v3?x=1",
            "https://ani.sidestore.io/v3#frag",
            "https://ani.sidestore.io/some/path",
            "https://münchen.example",
            "https:\\\\ani.sidestore.io",
            "javascript:alert(1)",
            "file:///x",
            "https://ani.sidestore.io/x\nHeader: 1",
        ] {
            assert!(require_anisette_url(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn redact_strips_query_secrets() {
        let url = Url::parse("https://release-assets.githubusercontent.com/a/b?token=SECRET&x=1").unwrap();
        let redacted = redact_url(&url);
        assert!(!redacted.contains("SECRET"));
        assert!(redacted.contains("release-assets.githubusercontent.com"));
    }

    #[test]
    fn privacy_documents_all_hosts() {
        let privacy = include_str!("../../PRIVACY.md");
        for host in documented_hosts() {
            assert!(
                privacy.contains(host),
                "PRIVACY.md must document host '{host}'"
            );
        }
    }
}
