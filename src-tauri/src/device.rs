use std::sync::Mutex;

use idevice::{
    IdeviceService,
    lockdown::LockdownClient,
    provider::UsbmuxdProvider,
    usbmuxd::{Connection, UsbmuxdAddr, UsbmuxdConnection},
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tokio_util::sync::CancellationToken;

use crate::{error::AppError, pairing::pairing_file};

/// Public label used for provider/hostname (own branding).
pub const PROVIDER_LABEL: &str = "denizsigner";

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub name: String,
    pub id: u32,
    pub udid: String,
    pub connection_type: String,
    pub version: String,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfoWithPairing {
    pub info: DeviceInfo,
    pub pairing: Vec<u8>,
}

pub type DeviceInfoMutex = Mutex<Option<DeviceInfoWithPairing>>;
pub type PairingCancelToken = Mutex<Option<CancellationToken>>;

/// Lists locally attached devices via usbmuxd (USB/network, no internet).
/// Polled only on start / user refresh / device change — no background polling.
#[tauri::command]
pub async fn list_devices() -> Result<Vec<Result<DeviceInfo, AppError>>, AppError> {
    let mut usbmuxd = get_usbmuxd().await?;
    let devs = usbmuxd.get_devices().await.map_err(|e| {
        AppError::Usbmuxd("Failed to list devices from usbmuxd".into(), e.to_string())
    })?;
    if devs.is_empty() {
        return Ok(vec![]);
    }
    let addr = UsbmuxdAddr::from_env_var().map_err(|e| {
        AppError::Usbmuxd("Invalid usbmuxd address from environment".into(), e.to_string())
    })?;

    let futs: Vec<_> = devs
        .iter()
        .map(|d| {
            let addr = addr.clone();
            async move {
                let provider = d.to_provider(addr, PROVIDER_LABEL);
                let connection_type = match d.connection_type {
                    Connection::Usb => "USB",
                    Connection::Network(_) => "Network",
                    Connection::Unknown(_) => "Unknown",
                }
                .to_string();
                let mut lc = LockdownClient::connect(&provider).await.map_err(|e| {
                    AppError::DeviceComsWithMessage(
                        "Unable to connect to lockdown".into(),
                        e.to_string(),
                    )
                })?;
                let name = lc
                    .get_value(Some("DeviceName"), None)
                    .await
                    .map_err(|e| {
                        AppError::DeviceComsWithMessage(
                            "Failed to fetch DeviceName".into(),
                            e.to_string(),
                        )
                    })?
                    .as_string()
                    .ok_or_else(|| AppError::DeviceComs("DeviceName was not a string".into()))?
                    .to_string();
                let version = lc
                    .get_value(Some("ProductVersion"), None)
                    .await
                    .map_err(|e| {
                        AppError::DeviceComsWithMessage(
                            "Failed to fetch ProductVersion".into(),
                            e.to_string(),
                        )
                    })?
                    .as_string()
                    .ok_or_else(|| AppError::DeviceComs("Product version was not a string".into()))?
                    .to_string();
                Ok::<DeviceInfo, AppError>(DeviceInfo {
                    name,
                    id: d.device_id,
                    udid: d.udid.clone(),
                    connection_type,
                    version,
                })
            }
        })
        .collect();
    Ok(futures::future::join_all(futs).await)
}

#[tauri::command]
pub async fn set_selected_device(
    app: AppHandle,
    device_state: State<'_, DeviceInfoMutex>,
    cancel_state: State<'_, PairingCancelToken>,
    device: Option<DeviceInfo>,
) -> Result<(), AppError> {
    if device.is_none() {
        *device_state.lock().unwrap() = None;
        return Ok(());
    }
    let mut usbmuxd = get_usbmuxd().await?;
    let token = CancellationToken::new();
    if let Some(old) = cancel_state.lock().unwrap().replace(token.clone()) {
        old.cancel();
    }
    let pairing = pairing_file(&app, device.as_ref().unwrap(), &mut usbmuxd, token.clone()).await;
    if !token.is_cancelled() {
        *cancel_state.lock().unwrap() = None;
    }
    let pairing = pairing?;
    *device_state.lock().unwrap() = Some(DeviceInfoWithPairing { info: device.unwrap(), pairing });
    Ok(())
}

#[tauri::command]
pub async fn cancel_pairing(cancel_state: State<'_, PairingCancelToken>) -> Result<(), AppError> {
    if let Some(t) = cancel_state.lock().unwrap().take() {
        t.cancel();
    }
    Ok(())
}

pub async fn get_usbmuxd() -> Result<UsbmuxdConnection, AppError> {
    UsbmuxdConnection::default()
        .await
        .map_err(|e| AppError::Usbmuxd("Failed to connect to usbmuxd".into(), e.to_string()))
}

pub async fn get_provider(device_info: &DeviceInfo) -> Result<UsbmuxdProvider, AppError> {
    get_provider_from_connection(device_info, &mut get_usbmuxd().await?).await
}

pub async fn get_provider_from_connection(
    device_info: &DeviceInfo,
    connection: &mut UsbmuxdConnection,
) -> Result<UsbmuxdProvider, AppError> {
    let device = connection.get_device(&device_info.udid).await.map_err(|e| {
        AppError::DeviceComsWithMessage("Failed to get device".into(), e.to_string())
    })?;
    Ok(device.to_provider(UsbmuxdAddr::from_env_var().unwrap(), PROVIDER_LABEL))
}
