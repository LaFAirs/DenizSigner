//! Pairing-file management (lockdown + remote pairing), stored via the
//! OS keyring when available. Hostname label uses DenizSigner branding.

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use idevice::{
    IdeviceError, IdeviceService, RemoteXpcClient,
    core_device_proxy::CoreDeviceProxy,
    house_arrest::HouseArrestClient,
    installation_proxy::InstallationProxyClient,
    lockdown::LockdownClient,
    provider::{IdeviceProvider, UsbmuxdProvider},
    remote_pairing::{RemotePairingClient, RpPairingFile},
    rsd::RsdHandshake,
    usbmuxd::UsbmuxdConnection,
};
use isideload::util::storage::{InMemoryStorage, SideloadingStorage};
use plist_macro::{plist, plist_to_xml_bytes};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{
    device::{DeviceInfo, DeviceInfoMutex, DeviceInfoWithPairing, PROVIDER_LABEL, get_provider},
    error::AppError,
    secure_storage::{create_sideloading_storage, keyring_available},
};

struct PairingStorageEntry {
    keyring_enabled: bool,
    storage: Box<dyn SideloadingStorage>,
}

static PAIRING_STORAGE: OnceLock<Mutex<PairingStorageEntry>> = OnceLock::new();

const PAIRING_APPS: &[(&str, &str)] = &[
    ("SideStore", "ALTPairingFile.mobiledevicepairing"),
    ("LiveContainer", "SideStore/Documents/ALTPairingFile.mobiledevicepairing"),
    ("Feather", "pairingFile.plist"),
    ("StikDebug", "pairingFile.plist"),
    ("StikDebug (Sideloaded)", "rp_pairing_file.plist"),
    ("StikTest", "stiktest_pairing.plist"),
    ("Protokolle", "pairingFile.plist"),
    ("Antrag", "pairingFile.plist"),
    ("SparseBox", "pairingFile.plist"),
    ("StikStore", "pairingFile.plist"),
    ("ByeTunes", "pairing file/pairingFile.plist"),
    ("Reynard", "pairingFile.plist"),
    ("PanicAnalyzer", "pairingFile.plist"),
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingAppInfo {
    pub name: String,
    pub bundle_id: String,
    pub path: String,
}

async fn generate_lockdown_plist(
    device: &DeviceInfo,
    provider: &dyn IdeviceProvider,
    usbmuxd: &mut UsbmuxdConnection,
) -> Result<plist::Value, AppError> {
    let mut pairing_file = usbmuxd.get_pair_record(&device.udid).await.map_err(|e| {
        AppError::LockdownPairing("Failed to get pairing record for device".into(), e.to_string())
    })?;
    pairing_file.udid = Some(device.udid.clone());
    let mut lc = LockdownClient::connect(provider).await.map_err(|e| {
        AppError::DeviceComsWithMessage("Failed to connect to lockdown".into(), e.to_string())
    })?;
    lc.start_session(&pairing_file).await.map_err(|e| {
        AppError::DeviceComsWithMessage("Failed to start lockdown session".into(), e.to_string())
    })?;
    lc.set_value("EnableWifiDebugging", true.into(), Some("com.apple.mobile.wireless_lockdown"))
        .await
        .map_err(|e| AppError::LockdownPairing("Failed to enable wifi debugging".into(), e.to_string()))?;
    plist::Value::from_reader_xml(std::io::Cursor::new(pairing_file.serialize().map_err(|e| {
        AppError::LockdownPairing("Failed to serialize pairing file".into(), e.to_string())
    })?))
    .map_err(|e| AppError::LockdownPairing("Failed to parse pairing file as plist".into(), e.to_string()))
}

async fn generate_rppairing_plist(
    provider: &dyn IdeviceProvider,
) -> Result<(plist::Value, Vec<u8>), IdeviceError> {
    let bytes = generate_rppairing(provider, PROVIDER_LABEL).await?.to_bytes();
    let plist = plist::Value::from_reader_xml(std::io::Cursor::new(&bytes))
        .map_err(|e| IdeviceError::InternalError(format!("Invalid RPPairing plist: {e}")))?;
    Ok((plist, bytes))
}

async fn generate_rppairing(
    provider: &dyn IdeviceProvider,
    hostname: &str,
) -> Result<RpPairingFile, IdeviceError> {
    info!("Connecting to CoreDeviceProxy...");
    let proxy = CoreDeviceProxy::connect(provider).await?;
    let rsd_port = proxy.tunnel_info().server_rsd_port;
    let adapter = proxy.create_software_tunnel()?;
    let mut adapter = adapter.to_async_handle();
    let rsd_stream = adapter.connect(rsd_port).await?;
    let handshake = RsdHandshake::new(rsd_stream).await?;
    let tunnel_service = handshake
        .services
        .get("com.apple.internal.dt.coredevice.untrusted.tunnelservice")
        .ok_or_else(|| IdeviceError::InternalError("Untrusted tunnel service not found".into()))?;
    let tunnel_service_stream = adapter.connect(tunnel_service.port).await?;
    let mut remote_xpc = RemoteXpcClient::new(tunnel_service_stream).await?;
    remote_xpc.do_handshake().await?;
    let _ = remote_xpc.recv_root().await;
    info!("(You may need to tap Trust on the device)");
    let mut pairing_file = RpPairingFile::generate(hostname);
    let mut pairing_client = RemotePairingClient::new(remote_xpc, hostname);
    pairing_client.connect(&mut pairing_file, async || "000000".to_string()).await?;
    let tunnel_service_stream = adapter.connect(tunnel_service.port).await?;
    let mut remote_xpc = RemoteXpcClient::new(tunnel_service_stream).await?;
    remote_xpc.do_handshake().await?;
    let _ = remote_xpc.recv_root().await;
    let mut pairing_client = RemotePairingClient::new(remote_xpc, hostname);
    pairing_client.connect(&mut pairing_file, async || "000000".to_string()).await?;
    Ok(pairing_file)
}

pub async fn place_file(
    pairing: Vec<u8>,
    provider: &dyn IdeviceProvider,
    bundle_id: String,
    path: String,
) -> Result<(), AppError> {
    let house_arrest_client = HouseArrestClient::connect(provider)
        .await
        .map_err(|e| AppError::HouseArrest("Failed to connect to house arrest".into(), e.to_string()))?;
    let mut afc_client = house_arrest_client
        .vend_documents(bundle_id)
        .await
        .map_err(|e| AppError::HouseArrest("Failed to vend documents".into(), e.to_string()))?;
    afc_client
        .mk_dir(format!("/Documents/{}", path.rsplit_once('/').map(|x| x.0).unwrap_or("")))
        .await
        .map_err(|e| AppError::HouseArrest("Failed to create Documents directory".into(), e.to_string()))?;
    let mut file = afc_client
        .open(format!("/Documents/{path}"), idevice::afc::opcode::AfcFopenMode::Wr)
        .await
        .map_err(|e| AppError::HouseArrest("Failed to open file on device".into(), e.to_string()))?;
    file.write_entire(&pairing)
        .await
        .map_err(|e| AppError::HouseArrest("Failed to write pairing file".into(), e.to_string()))?;
    file.close()
        .await
        .map_err(|e| AppError::HouseArrest("Failed to close file".into(), e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn place_pairing_cmd(
    device_state: State<'_, DeviceInfoMutex>,
    bundle_id: String,
    path: String,
) -> Result<(), AppError> {
    let device = current_device(&device_state)?;
    let provider = get_provider(&device.info).await?;
    place_file(device.pairing, &provider, bundle_id, path).await
}

#[tauri::command]
pub async fn export_pairing_cmd(
    device_state: State<'_, DeviceInfoMutex>,
    app: AppHandle,
) -> Result<(), AppError> {
    let device = current_device(&device_state)?;
    let save_path = app
        .dialog()
        .file()
        .add_filter("Pairing File", &["plist", "mobiledevicepairing"])
        .set_file_name("pairingFile.plist")
        .set_title("Export Pairing File")
        .blocking_save_file();
    if let Some(p) = save_path
        && let Some(p) = p.as_path()
    {
        tokio::fs::write(p, &device.pairing)
            .await
            .map_err(|e| AppError::Filesystem("Failed to write pairing file".into(), e.to_string()))?;
        return Ok(());
    }
    Err(AppError::Canceled("Export".into()))
}

fn build_pairing_storage_entry(app: &AppHandle, keyring_enabled: bool) -> PairingStorageEntry {
    let storage = create_sideloading_storage(app).unwrap_or_else(|e| {
        error!("Pairing storage fallback to memory: {e}");
        Box::new(InMemoryStorage::new())
    });
    PairingStorageEntry { keyring_enabled, storage }
}

fn with_pairing_storage<T>(
    app: &AppHandle,
    f: impl FnOnce(&dyn SideloadingStorage) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let current = keyring_available();
    let cell = PAIRING_STORAGE
        .get_or_init(|| Mutex::new(build_pairing_storage_entry(app, current)));
    let mut guard = cell.lock().map_err(|_| AppError::Misc("Failed to lock pairing storage".into()))?;
    if guard.keyring_enabled != current {
        info!("Pairing storage backend changed, recreating storage");
        *guard = build_pairing_storage_entry(app, current);
    }
    f(guard.storage.as_ref())
}

fn current_device(device_state: &DeviceInfoMutex) -> Result<DeviceInfoWithPairing, AppError> {
    device_state.lock().unwrap().clone().ok_or(AppError::NoDeviceSelected)
}

pub async fn pairing_file(
    app: &AppHandle,
    device: &DeviceInfo,
    usbmuxd: &mut UsbmuxdConnection,
    cancel: CancellationToken,
) -> Result<Vec<u8>, AppError> {
    let provider = get_provider(device).await?;
    let lockdown_plist = tokio::select! {
        _ = cancel.cancelled() => return Err(AppError::Canceled("Pairing".into())),
        res = generate_lockdown_plist(device, &provider, usbmuxd) => res?
    };
    if is_ios_version_below(&device.version, 17, 4) {
        let dict = lockdown_plist.as_dictionary().cloned().ok_or_else(|| {
            AppError::LockdownPairing(
                "Lockdown plist was not a dictionary".into(),
                "Invalid lockdown plist".into(),
            )
        })?;
        return Ok(plist_to_xml_bytes(&dict));
    }
    let cache_key = format!("rppairing_file_{}", device.udid);
    let cached: Option<Vec<u8>> = with_pairing_storage(app, |s| {
        s.retrieve_data(&cache_key)
            .map_err(|e| AppError::Storage("Failed to get RPPairing from storage".into(), e.to_string()))
    })?;
    let rppairing_plist = match cached {
        Some(bytes) => match plist::Value::from_reader_xml(std::io::Cursor::new(&bytes)) {
            Ok(p) => p,
            Err(e) => {
                warn!("Cached RPPairing invalid for {}, regenerating: {e}", device.name);
                generate_and_store(&provider, app, &cache_key, &cancel).await?
            }
        },
        None => {
            info!("Generating new RPPairing for device {}", device.name);
            generate_and_store(&provider, app, &cache_key, &cancel).await?
        }
    };
    if cancel.is_cancelled() {
        return Err(AppError::Canceled("Pairing".into()));
    }
    let merged = plist!(dict {
        :< lockdown_plist,
        :< rppairing_plist,
    });
    Ok(plist_to_xml_bytes(&merged))
}

async fn generate_and_store(
    provider: &UsbmuxdProvider,
    app: &AppHandle,
    cache_key: &str,
    cancel: &CancellationToken,
) -> Result<plist::Value, AppError> {
    let (plist, bytes) = tokio::select! {
        _ = cancel.cancelled() => return Err(AppError::Canceled("Pairing".into())),
        res = generate_rppairing_plist(provider) => res.map_err(|e| AppError::RemotePairing(e.to_string()))?
    };
    with_pairing_storage(app, |s| {
        s.store_data(cache_key, &bytes)
            .map_err(|e| AppError::Storage("Failed to store RPPairing".into(), e.to_string()))
    })?;
    Ok(plist)
}

#[tauri::command]
pub async fn delete_stored_rppairing(
    device_state: State<'_, DeviceInfoMutex>,
    app: AppHandle,
) -> Result<(), AppError> {
    let device = current_device(&device_state)?;
    let cache_key = format!("rppairing_file_{}", device.info.udid);
    with_pairing_storage(&app, |s| {
        s.delete(&cache_key)
            .map_err(|e| AppError::Storage("Failed to delete stored RPPairing".into(), e.to_string()))
    })
}

#[tauri::command]
pub async fn has_stored_rppairing(device: DeviceInfo, app: AppHandle) -> Result<bool, AppError> {
    if is_ios_version_below(&device.version, 17, 4) {
        return Ok(true);
    }
    let cache_key = format!("rppairing_file_{}", device.udid);
    with_pairing_storage(&app, |s| {
        s.retrieve_data(&cache_key)
            .map_err(|e| AppError::Storage("Failed to retrieve stored RPPairing".into(), e.to_string()))
    })
    .map(|o| o.is_some())
}

#[tauri::command]
pub async fn installed_pairing_apps(
    device_state: State<'_, DeviceInfoMutex>,
) -> Result<Vec<PairingAppInfo>, AppError> {
    let device = current_device(&device_state)?;
    let provider = get_provider(&device.info).await?;
    let mut proxy = InstallationProxyClient::connect(&provider).await.map_err(|e| {
        AppError::DeviceComsWithMessage("Failed to connect to installation proxy".into(), e.to_string())
    })?;
    let installed = proxy.get_apps(Some("User"), None).await.map_err(|e| {
        AppError::DeviceComsWithMessage("Failed to get installed apps".into(), e.to_string())
    })?;
    collect_pairing_apps(installed)
}

pub async fn get_sidestore_info(
    device: &DeviceInfo,
    live_container: bool,
) -> Result<Option<PairingAppInfo>, AppError> {
    let provider = get_provider(device).await?;
    let mut proxy = InstallationProxyClient::connect(&provider).await.map_err(|e| {
        AppError::DeviceComsWithMessage("Failed to connect to installation proxy".into(), e.to_string())
    })?;
    let installed = proxy.get_apps(Some("User"), None).await.map_err(|e| {
        AppError::DeviceComsWithMessage("Failed to get installed apps".into(), e.to_string())
    })?;
    for (bundle_id, app) in installed {
        let n = app
            .as_dictionary()
            .and_then(|x| x.get("CFBundleDisplayName").and_then(|x| x.as_string()))
            .ok_or_else(|| AppError::Misc("Failed to parse installed apps".into()))?;
        if n == "SideStore" || (live_container && n == "LiveContainer") {
            return Ok(Some(PairingAppInfo {
                name: n.to_string(),
                bundle_id: bundle_id.to_string(),
                path: PAIRING_APPS
                    .iter()
                    .find(|(name, _)| name == &n)
                    .map(|(_, p)| p.to_string())
                    .unwrap_or_default(),
            }));
        }
    }
    Ok(None)
}

fn collect_pairing_apps(
    installed: HashMap<String, plist::Value>,
) -> Result<Vec<PairingAppInfo>, AppError> {
    let mut found = HashMap::new();
    for (bundle_id, app) in installed {
        let n = app
            .as_dictionary()
            .and_then(|x| x.get("CFBundleDisplayName").and_then(|x| x.as_string()))
            .ok_or_else(|| AppError::Misc("Failed to parse installed apps".into()))?;
        if PAIRING_APPS.iter().any(|(name, _)| name == &n) {
            if bundle_id.contains("com.stik.stikdebug") {
                found.insert(format!("{n} (Sideloaded)"), bundle_id);
            } else {
                found.insert(n.to_string(), bundle_id);
            }
        }
    }
    let mut out = Vec::new();
    for (name, path) in PAIRING_APPS {
        if let Some(bundle_id) = found.get(*name) {
            out.push(PairingAppInfo {
                name: name.to_string(),
                bundle_id: bundle_id.to_string(),
                path: path.to_string(),
            });
        }
    }
    Ok(out)
}

fn parse_component(segment: Option<&str>) -> u32 {
    segment
        .and_then(|s| {
            let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.is_empty() { None } else { digits.parse().ok() }
        })
        .unwrap_or(0)
}

fn is_ios_version_below(version: &str, major: u32, minor: u32) -> bool {
    let mut parts = version.split('.');
    (parse_component(parts.next()), parse_component(parts.next())) < (major, minor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(is_ios_version_below("17.3.1", 17, 4));
        assert!(!is_ios_version_below("17.4", 17, 4));
        assert!(!is_ios_version_below("18.0", 17, 4));
    }
}
