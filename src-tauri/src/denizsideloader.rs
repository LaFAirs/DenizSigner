//! DenizSideloader companion installer (Windows-assisted flow).
//!
//! DenizSideloader is DenizSigner's own minimal iOS companion app
//! (`DenizSideloader/` in this repo, MIT, clean-room implementation).
//! This module installs a **locally built** DenizSideloader IPA over USB
//! using the already proven transport (`isideload` signing +
//! `idevice` lockdown/AFC), then places the pairing file inside the
//! companion's Documents folder so the on-device refresh tracker can
//! find it.
//!
//! Design notes (honest scope):
//! - No download step: the IPA comes from the iOS CI artifact
//!   (`DenizSideloader-unsigned.ipa`) or a local Xcode build. That keeps
//!   the network boundary unchanged (no new hosts) and avoids shipping
//!   a possibly stale bundled binary.
//! - Free Apple IDs stay bound to Apple's limits (3 apps, 10 App IDs,
//!   ~7 day expiry). Nothing here extends or hides that.
//! - On-device re-signing without a PC is NOT claimed: the refresh
//!   tracking lives on-device, the actual re-sign runs here on Windows.
//!   See `DenizSideloader/README.md` and `docs/IPADOS27_COMPAT.md`.
//! - The legacy `install_sidestore_operation` (SideStore/LiveContainer)
//!   is kept in `sideload.rs` as a documented recovery path only; the UI
//!   no longer offers it. See `docs/RECOVERY_SIDESTORE.md`.

use idevice::installation_proxy::InstallationProxyClient;
use tauri::{State, Window};

use crate::{
    device::{DeviceInfo, DeviceInfoMutex, get_provider, get_provider_from_connection, get_usbmuxd},
    error::AppError,
    ipa::validate_ipa_path,
    operation::Operation,
    pairing::{PairingAppInfo, place_file},
    sideload::{SideloaderMutex, sideload},
};

/// Bundle ID of the DenizSideloader companion app.
/// Must match `PRODUCT_BUNDLE_IDENTIFIER` in the Xcode project.
pub const DENIZSIDELOADER_BUNDLE_ID: &str = "com.denizbudakli.denizsideloader";
/// Display name shown on the Home Screen (matched as a fallback).
pub const DENIZSIDELOADER_DISPLAY_NAME: &str = "DenizSideloader";
/// Pairing-file location inside the companion's Documents folder.
/// Same filename convention as other refresh-capable companions.
pub const DENIZSIDELOADER_PAIRING_PATH: &str = "ALTPairingFile.mobiledevicepairing";
/// Validity window of a free Apple-ID provisioning profile.
pub const FREE_ACCOUNT_VALIDITY_DAYS: i64 = 7;
/// Refresh is due while this many (or fewer) days remain.
pub const REFRESH_DUE_WITHIN_DAYS: i64 = 2;
/// Seconds per day (fixed; expiry math uses whole days).
pub const SECONDS_PER_DAY: i64 = 86_400;

/// Unix timestamp at which a profile installed at `installed_at_unix`
/// expires under a free Apple ID (`installed + 7 days`).
pub fn expiry_timestamp(installed_at_unix: i64) -> i64 {
    installed_at_unix.saturating_add(FREE_ACCOUNT_VALIDITY_DAYS.saturating_mul(SECONDS_PER_DAY))
}

/// Whole days remaining until `expiry_unix` (negative when expired).
/// Floor division, matching the Swift `ExpiryCalculator` exactly.
pub fn days_remaining(expiry_unix: i64, now_unix: i64) -> i64 {
    expiry_unix.saturating_sub(now_unix).div_euclid(SECONDS_PER_DAY)
}

/// True when the profile should be refreshed now
/// (expired or within the due window).
pub fn needs_refresh(expiry_unix: i64, now_unix: i64) -> bool {
    days_remaining(expiry_unix, now_unix) <= REFRESH_DUE_WITHIN_DAYS
}

/// True when the profile is already expired.
pub fn is_expired(expiry_unix: i64, now_unix: i64) -> bool {
    now_unix >= expiry_unix
}

/// Backoff delay before retrying a failed Windows-assisted refresh.
/// `attempt` is 1-based. Caps at one hour so retries stay user-visible
/// instead of silently stretching into days.
pub fn refresh_retry_delay_secs(attempt: u32) -> u64 {
    const CAP: u64 = 3_600;
    const BASE: u64 = 60;
    let shift = attempt.saturating_sub(1).min(6);
    BASE.saturating_mul(1_u64 << shift).min(CAP)
}

/// Resolves the installed DenizSideloader companion (if any) via the
/// on-device installation proxy. Matches by bundle ID first, then by
/// display name as a fallback for renamed/debug builds.
pub async fn get_denizsideloader_info(
    device: &DeviceInfo,
) -> Result<Option<PairingAppInfo>, AppError> {
    let provider = get_provider(device).await?;
    let mut proxy = InstallationProxyClient::connect(&provider).await.map_err(|e| {
        AppError::DeviceComsWithMessage("Failed to connect to installation proxy".into(), e.to_string())
    })?;
    let installed = proxy.get_apps(Some("User"), None).await.map_err(|e| {
        AppError::DeviceComsWithMessage("Failed to get installed apps".into(), e.to_string())
    })?;
    for (bundle_id, app) in &installed {
        let display = app
            .as_dictionary()
            .and_then(|x| x.get("CFBundleDisplayName").and_then(|x| x.as_string()))
            .unwrap_or("");
        if bundle_id == DENIZSIDELOADER_BUNDLE_ID || display == DENIZSIDELOADER_DISPLAY_NAME {
            return Ok(Some(PairingAppInfo {
                name: DENIZSIDELOADER_DISPLAY_NAME.to_string(),
                bundle_id: bundle_id.to_string(),
                path: DENIZSIDELOADER_PAIRING_PATH.to_string(),
            }));
        }
    }
    Ok(None)
}

/// Installs a local DenizSideloader IPA and places the pairing file.
/// `app_path` must point at a CI-built `DenizSideloader-unsigned.ipa`
/// (or any valid IPA carrying the companion bundle ID).
#[tauri::command]
pub async fn install_denizsideloader_operation(
    window: Window,
    device_state: State<'_, DeviceInfoMutex>,
    sideloader_state: State<'_, SideloaderMutex>,
    app_path: String,
) -> Result<(), AppError> {
    let op = Operation::new("install_denizsideloader".to_string(), &window);
    op.start("verify")?;
    op.fail_if_err("verify", validate_ipa_path(&app_path))?;
    op.move_on("verify", "install")?;
    // Clone the selected device first: `sideload` takes ownership of the
    // state handles (same pattern as the legacy SideStore flow).
    let device = device_state.lock().unwrap().clone().ok_or(AppError::NoDeviceSelected)?;
    op.fail_if_err(
        "install",
        sideload(device_state, sideloader_state, app_path).await,
    )?;
    op.move_on("install", "pairing")?;
    let info = op.fail_if_err("pairing", get_denizsideloader_info(&device.info).await)?;
    if let Some(info) = info {
        let mut usbmuxd = op.fail_if_err("pairing", get_usbmuxd().await)?;
        let provider = op.fail_if_err(
            "pairing",
            get_provider_from_connection(&device.info, &mut usbmuxd).await,
        )?;
        op.fail_if_err("pairing", place_file(device.pairing, &provider, info.bundle_id, info.path).await)?;
    } else {
        return op.fail(
            "pairing",
            AppError::HouseArrest(
                "DenizSideloader not found".into(),
                "The device did not report the DenizSideloader bundle ID as installed".into(),
            ),
        );
    }
    op.complete("pairing")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_identity_is_stable() {
        assert_eq!(DENIZSIDELOADER_BUNDLE_ID, "com.denizbudakli.denizsideloader");
        assert_eq!(DENIZSIDELOADER_PAIRING_PATH, "ALTPairingFile.mobiledevicepairing");
    }

    #[test]
    fn expiry_is_seven_days() {
        assert_eq!(expiry_timestamp(0), 7 * 86_400);
        assert_eq!(expiry_timestamp(1_000), 1_000 + 7 * 86_400);
    }

    #[test]
    fn remaining_and_refresh_windows() {
        let installed = 1_000_000_i64;
        let expiry = expiry_timestamp(installed);
        // Freshly installed: 7 days left, no refresh needed.
        assert_eq!(days_remaining(expiry, installed), 7);
        assert!(!needs_refresh(expiry, installed));
        assert!(!is_expired(expiry, installed));
        // Two days left: refresh is due.
        assert!(needs_refresh(expiry, expiry - 2 * 86_400));
        assert!(!is_expired(expiry, expiry - 2 * 86_400));
        // Three days left: not yet due.
        assert!(!needs_refresh(expiry, expiry - 3 * 86_400));
        // At and past expiry: refresh due + expired.
        assert!(needs_refresh(expiry, expiry));
        assert!(is_expired(expiry, expiry));
        assert!(needs_refresh(expiry, expiry + 100));
        assert!(is_expired(expiry, expiry + 100));
        assert!(days_remaining(expiry, expiry + 100) < 0);
    }

    #[test]
    fn retry_backoff_caps_at_one_hour() {
        assert_eq!(refresh_retry_delay_secs(1), 60);
        assert_eq!(refresh_retry_delay_secs(2), 120);
        assert_eq!(refresh_retry_delay_secs(3), 240);
        assert_eq!(refresh_retry_delay_secs(7), 3_600);
        assert_eq!(refresh_retry_delay_secs(100), 3_600);
    }

    #[test]
    fn pairing_failure_has_actionable_guidance() {
        // The pairing step must fail with a house_arrest error (which maps
        // to "open the app once, then retry") rather than a generic error.
        let (title, hint) = crate::error::friendly_error("house_arrest".to_string());
        assert!(!title.is_empty() && !hint.is_empty());
    }
}
