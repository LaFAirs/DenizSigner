//! Friendly error model: technical errors stay available, but the UI
//! shows a human-readable title + actionable hint first.

use idevice::IdeviceError;
use isideload::SideloadError;
use rootcause::Report;
use serde::ser::{SerializeStruct, Serializer};
use serde::Serialize;

#[derive(Debug, thiserror::Error, Clone, strum::AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum AppError {
    #[error("{0}")]
    MaxApps(String),
    #[error("{0}")]
    NotEnoughAppIds(String),
    #[error("{0}")]
    DeviceComs(String),
    #[error("{0}")]
    Underage(String),
    #[error("{0}")]
    AccountLocked(String),
    #[error("{0}")]
    Developer(String),
    #[error("{0}")]
    Auth(String),
    #[error("{0}")]
    Download(String),
    #[error("{0}: {1}")]
    HouseArrest(String, String),
    #[error("{0}")]
    RemotePairing(String),
    #[error("{0}: {1}")]
    LockdownPairing(String, String),
    #[error("{0} canceled")]
    Canceled(String),
    #[error("Failed to emit status to frontend: {0}")]
    OperationUpdate(String),
    #[error("{0}: {1}")]
    DeviceComsWithMessage(String, String),
    #[error("{0}: {1}")]
    Usbmuxd(String, String),
    #[error("Not logged in")]
    NotLoggedIn,
    #[error("No device selected")]
    NoDeviceSelected,
    #[error("{0}")]
    Anisette(String),
    #[error("{0}")]
    Keyring(String),
    #[error("Keyring error: {0} - {1}")]
    KeyringWithMessage(String, String),
    #[error("{0}: {1}")]
    Storage(String, String),
    #[error("{0}")]
    Misc(String),
    #[error("{0}: {1}")]
    Filesystem(String, String),
    #[error("{0}")]
    Network(String),
    #[error("{0}")]
    InvalidIpa(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("type", <Self as AsRef<str>>::as_ref(self))?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

/// Maps an error type to (title, hint) shown in the UI.
/// Technical message stays available under "View technical details".
pub fn friendly(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "max_apps" => (
            "Installation failed — app limit reached",
            "This Apple ID already has the maximum number of sideloaded apps. Remove an unused app from the device, then try again.",
        ),
        "not_enough_app_ids" => (
            "Installation failed — no free App ID",
            "Apple rejected the provisioning profile. Free up an App ID (max 10 per 7 days on free accounts) or wait.",
        ),
        "account_locked" => (
            "Apple account locked",
            "Apple locked this account for security. Unlock it at iforgot.apple.com, then sign in again.",
        ),
        "auth" => (
            "Sign-in failed",
            "Check your Apple ID, password and 2FA code. Use an app-specific password only if Apple asks for one.",
        ),
        "developer" | "underage" => (
            "Apple Developer request rejected",
            "Check your Apple Developer account status, device pairing and Developer Mode.",
        ),
        "anisette" => (
            "Apple verification service unreachable",
            "The configured Anisette server did not answer. Check Settings → Signing or try another server.",
        ),
        "device_coms" | "device_coms_with_message" | "usbmuxd" => (
            "Device communication failed",
            "Check the USB connection, trust prompt on the device, and that iTunes/Apple Devices (Windows) is installed.",
        ),
        "house_arrest" => (
            "Pairing file could not be placed",
            "Make sure the target app is installed and open it once, then retry placing the pairing file.",
        ),
        "remote_pairing" | "lockdown_pairing" => (
            "Pairing failed",
            "Tap Trust on the device when prompted, keep it unlocked, then retry.",
        ),
        "download" => (
            "Download failed",
            "Check your connection and retry. DenizSigner only downloads IPAs you explicitly request.",
        ),
        "network" => (
            "Blocked by network policy",
            "This request is outside the documented allowlist (see Settings → Privacy).",
        ),
        "invalid_ipa" => (
            "Invalid IPA file",
            "The file is not a valid .ipa (a ZIP containing Payload/*.app). Choose another file.",
        ),
        "not_logged_in" => (
            "Not signed in",
            "Sign in with your Apple ID under Signing first.",
        ),
        "no_device_selected" => (
            "No device selected",
            "Connect your iPhone/iPad via USB and select it.",
        ),
        "keyring" | "keyring_with_message" => (
            "Secure storage unavailable",
            "The OS credential store denied access. See Settings → Privacy for the insecure-storage warning.",
        ),
        _ => (
            "Something went wrong",
            "Check your Apple Developer account, device pairing and Developer Mode. Technical details below.",
        ),
    }
}

impl From<Report> for AppError {
    fn from(report: Report) -> Self {
        let report_str = report.to_string();

        for cause in report.iter_reports() {
            if cause.downcast_current_context::<keyring::Error>().is_some() {
                return AppError::Keyring(report_str);
            }
            if let Some(err) = cause.downcast_current_context::<SideloadError>() {
                match err {
                    &SideloadError::AuthWithMessage(code, _) => match code {
                        -20209 => return AppError::AccountLocked(report_str),
                        _ => return AppError::Auth(report_str),
                    },
                    &SideloadError::DeveloperError(code, _) => match code {
                        1102 => return AppError::Underage(report_str),
                        _ => return AppError::Developer(report_str),
                    },
                    SideloadError::IdeviceError(idev_err) => match idev_err {
                        IdeviceError::Socket(_) => return AppError::DeviceComs(report_str),
                        IdeviceError::ApplicationVerificationFailed(e) => {
                            if e.contains("maximum number of installed apps") {
                                return AppError::MaxApps(report_str);
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            let cause_str = cause.to_string();
            if cause_str.contains("Not enough available app IDs") {
                return AppError::NotEnoughAppIds(report_str);
            }
            if cause_str.contains("Failed to get anisette data for login")
                || cause_str.contains("Failed to get anisette client info")
                || cause_str.contains("Failed to get anisette headers")
            {
                return AppError::Anisette(report_str);
            }
        }

        AppError::Misc(report_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_codes_have_guidance() {
        for kind in ["max_apps", "auth", "anisette", "network", "invalid_ipa"] {
            let (title, hint) = friendly(kind);
            assert!(!title.is_empty() && !hint.is_empty(), "{kind}");
        }
    }

    #[test]
    fn unknown_code_falls_back() {
        let (title, _) = friendly("totally_unknown_xyz");
        assert_eq!(title, "Something went wrong");
    }
}
