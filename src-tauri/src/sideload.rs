use std::{path::PathBuf, sync::Mutex};

use isideload::{
    sideload::{application::SpecialApp, sideloader::Sideloader},
    util::callbacks::MaxCertsCallbackBox,
};
use tauri::{AppHandle, Manager, State, Window};

use crate::{
    device::{DeviceInfoMutex, get_provider, get_provider_from_connection, get_usbmuxd},
    error::AppError,
    ipa::validate_ipa_path,
    network_allowlist::{redact_url, require_allowed_url},
    operation::Operation,
    pairing::{get_sidestore_info, place_file},
};

pub type SideloaderMutex = Mutex<Option<Sideloader<MaxCertsCallbackBox>>>;

pub struct SideloaderGuard<'a> {
    state: &'a SideloaderMutex,
    sideloader: Option<Sideloader<MaxCertsCallbackBox>>,
}

impl<'a> SideloaderGuard<'a> {
    pub fn take(state: &'a SideloaderMutex) -> Result<Self, AppError> {
        let mut guard = state.lock().unwrap();
        let sideloader = guard.take().ok_or(AppError::NotLoggedIn)?;
        Ok(Self { state, sideloader: Some(sideloader) })
    }

    pub fn get_mut(&mut self) -> &mut Sideloader<MaxCertsCallbackBox> {
        self.sideloader.as_mut().expect("Sideloader should be present")
    }
}

impl Drop for SideloaderGuard<'_> {
    fn drop(&mut self) {
        *self.state.lock().unwrap() = self.sideloader.take();
    }
}

pub async fn sideload(
    device_state: State<'_, DeviceInfoMutex>,
    sideloader_state: State<'_, SideloaderMutex>,
    app_path: String,
) -> Result<Option<SpecialApp>, AppError> {
    validate_ipa_path(&app_path)?;
    let device = device_state.lock().unwrap().clone().ok_or(AppError::NoDeviceSelected)?;
    let provider = get_provider(&device.info).await?;
    let mut sideloader = SideloaderGuard::take(&sideloader_state)?;
    let special = sideloader
        .get_mut()
        .install_app(&provider, app_path.into(), false, None::<fn(f32) -> std::future::Ready<()>>)
        .await?;
    Ok(special)
}

#[tauri::command]
pub async fn sideload_operation(
    window: Window,
    device_state: State<'_, DeviceInfoMutex>,
    sideloader_state: State<'_, SideloaderMutex>,
    app_path: String,
) -> Result<(), AppError> {
    let op = Operation::new("sideload".to_string(), &window);
    op.start("prepare")?;
    op.fail_if_err("prepare", validate_ipa_path(&app_path))?;
    op.move_on("prepare", "install")?;
    op.fail_if_err("install", sideload(device_state, sideloader_state, app_path).await)?;
    op.complete("install")?;
    Ok(())
}

/// User-initiated SideStore/LiveContainer download + install + pairing.
/// Download URLs are allowlist-checked (github.com release assets only).
#[tauri::command]
pub async fn install_sidestore_operation(
    handle: AppHandle,
    window: Window,
    device_state: State<'_, DeviceInfoMutex>,
    sideloader_state: State<'_, SideloaderMutex>,
    nightly: bool,
    live_container: bool,
) -> Result<(), AppError> {
    let op = Operation::new("install_sidestore".to_string(), &window);
    op.start("download")?;
    let (filename, url) = if live_container {
        if nightly {
            ("LiveContainerSideStore-Nightly.ipa", "https://github.com/LiveContainer/LiveContainer/releases/download/nightly/LiveContainer+SideStore.ipa")
        } else {
            ("LiveContainerSideStore.ipa", "https://github.com/LiveContainer/LiveContainer/releases/latest/download/LiveContainer+SideStore.ipa")
        }
    } else if nightly {
        ("SideStore-Nightly.ipa", "https://github.com/SideStore/SideStore/releases/download/nightly/SideStore.ipa")
    } else {
        ("SideStore.ipa", "https://github.com/SideStore/SideStore/releases/latest/download/SideStore.ipa")
    };
    let url = require_allowed_url(url, &[])?.to_string();
    let dest = handle
        .path()
        .temp_dir()
        .map_err(|e| AppError::Filesystem("Failed to get temp dir".into(), e.to_string()))?
        .join(filename);
    op.fail_if_err("download", download(&url, &dest).await)?;
    op.move_on("download", "install")?;
    let device = device_state.lock().unwrap().clone().ok_or(AppError::NoDeviceSelected)?;
    op.fail_if_err(
        "install",
        sideload(device_state, sideloader_state, dest.to_string_lossy().to_string()).await,
    )?;
    op.move_on("install", "pairing")?;
    let info = op.fail_if_err("pairing", get_sidestore_info(&device.info, live_container).await)?;
    if let Some(info) = info {
        let mut usbmuxd = op.fail_if_err("pairing", get_usbmuxd().await)?;
        let provider =
            op.fail_if_err("pairing", get_provider_from_connection(&device.info, &mut usbmuxd).await)?;
        op.fail_if_err("pairing", place_file(device.pairing, &provider, info.bundle_id, info.path).await)?;
    } else {
        return op.fail(
            "pairing",
            AppError::HouseArrest(
                "SideStore not found".into(),
                "The device did not report SideStore's bundle ID as installed".into(),
            ),
        );
    }
    op.complete("pairing")?;
    Ok(())
}

/// Hard cap for a single IPA download (512 MiB — far above real
/// SideStore/LiveContainer artifacts, far below abuse scale).
pub const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
/// Maximum redirect hops followed, each re-validated against the allowlist.
const MAX_REDIRECTS: u8 = 5;

/// Downloads an allowlisted URL to `dest` (a caller-chosen temp path with a
/// fixed filename — never derived from remote data, so no path traversal).
/// Redirects are followed manually (max [`MAX_REDIRECTS`]) and every hop is
/// re-validated, so a redirect to an arbitrary host is blocked while legit
/// GitHub → CDN redirects keep working. The body streams with a hard size
/// cap; query strings never reach logs ([`redact_url`]).
pub async fn download(url: impl AsRef<str>, dest: &PathBuf) -> Result<(), AppError> {
    use futures::StreamExt;

    let mut current = require_allowed_url(url.as_ref(), &[])?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Download(format!("HTTP client setup failed: {e}")))?;

    let mut hops: u8 = 0;
    let response = loop {
        let redacted = redact_url(&current);
        let resp = client
            .get(current.clone())
            .send()
            .await
            .map_err(|e| AppError::Download(format!("{redacted}: {e}")))?;
        let status = resp.status();
        if status.is_success() {
            break resp;
        }
        if status.is_redirection() {
            hops += 1;
            if hops > MAX_REDIRECTS {
                return Err(AppError::Download(format!(
                    "{redacted}: too many redirects"
                )));
            }
            let location = resp.headers().get(reqwest::header::LOCATION).ok_or_else(|| {
                AppError::Download(format!("{redacted}: redirect without location"))
            })?;
            let location = location.to_str().map_err(|_| {
                AppError::Download(format!("{redacted}: invalid redirect target"))
            })?;
            // Resolve relative targets against the current URL, then validate.
            let next = current.join(location).map_err(|e| {
                AppError::Download(format!("{redacted}: invalid redirect target: {e}"))
            })?;
            current = require_allowed_url(next.as_str(), &[]).map_err(|_| {
                AppError::Network(format!(
                    "Blocked redirect to '{}'. See PRIVACY.md for the network allowlist.",
                    next.host_str().unwrap_or("?")
                ))
            })?;
            continue;
        }
        return Err(AppError::Download(format!(
            "Failed to download file: HTTP {status} ({redacted})"
        )));
    };

    check_content_length(response.content_length())?;
    // Truncate (never append to) any stale temp file with the same fixed name.
    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| AppError::Filesystem("Failed to write downloaded file".into(), e.to_string()))?;
    let mut total: u64 = 0;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|e| AppError::Download(format!("Download interrupted: {e}")))?;
        total = add_checked(total, chunk.len() as u64)?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| AppError::Filesystem("Failed to write downloaded file".into(), e.to_string()))?;
    }
    Ok(())
}

/// Rejects declared bodies larger than [`MAX_DOWNLOAD_BYTES`].
fn check_content_length(len: Option<u64>) -> Result<(), AppError> {
    if let Some(len) = len {
        add_checked(0, len)?;
    }
    Ok(())
}

/// Adds bytes under the cap; errors before any overflow or oversize write.
fn add_checked(total: u64, add: u64) -> Result<u64, AppError> {
    let total = total
        .checked_add(add)
        .ok_or_else(|| AppError::Download("Download size overflow.".to_string()))?;
    if total > MAX_DOWNLOAD_BYTES {
        return Err(AppError::Download(format!(
            "Download exceeds the {} MiB safety limit.",
            MAX_DOWNLOAD_BYTES / (1024 * 1024)
        )));
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_cap_bounds() {
        assert!(add_checked(0, 1024).is_ok());
        assert!(add_checked(MAX_DOWNLOAD_BYTES - 1, 1).is_ok());
        assert!(add_checked(0, MAX_DOWNLOAD_BYTES + 1).is_err());
        assert!(add_checked(u64::MAX, 1).is_err());
        assert!(check_content_length(None).is_ok());
        assert!(check_content_length(Some(MAX_DOWNLOAD_BYTES)).is_ok());
        assert!(check_content_length(Some(MAX_DOWNLOAD_BYTES + 1)).is_err());
    }
}
