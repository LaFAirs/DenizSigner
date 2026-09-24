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

pub async fn download(url: impl AsRef<str>, dest: &PathBuf) -> Result<(), AppError> {
    let url = require_allowed_url(url.as_ref(), &[])?;
    let response = reqwest::get(url.clone())
        .await
        .map_err(|e| AppError::Download(format!("{}: {e}", redact_url(&url))))?;
    if !response.status().is_success() {
        return Err(AppError::Download(format!("Failed to download file: HTTP {}", response.status())));
    }
    let bytes = response.bytes().await.map_err(|e| AppError::Download(e.to_string()))?;
    tokio::fs::write(dest, &bytes)
        .await
        .map_err(|e| AppError::Filesystem("Failed to write downloaded file".into(), e.to_string()))?;
    Ok(())
}
