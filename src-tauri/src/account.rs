use futures::FutureExt;
use isideload::{
    anisette::remote_v3::RemoteV3AnisetteProvider,
    auth::apple_account::{AppleAccount, TwoFactorCallbackParams, TwoFactorCallbackResponse},
    dev::{
        app_ids::{AppIdsApi, ListAppIdsResponse},
        certificates::{CertificatesApi, DevelopmentCertificate},
        developer_session::DeveloperSession,
    },
    sideload::{SideloaderBuilder, builder::MaxCertsBehavior, sideloader::Sideloader},
    util::callbacks::MaxCertsCallbackBox,
};
use rootcause::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Listener, State, Window};
use tauri_plugin_store::StoreExt;
use tracing::debug;

use crate::{
    device::PROVIDER_LABEL,
    error::AppError,
    network_allowlist::require_anisette_url,
    secure_storage::{create_sideloading_storage, credential_entry},
    sideload::{SideloaderGuard, SideloaderMutex},
};

/// Signs in with an Apple ID. The password is kept in memory only for the
/// login call; only the keyring holds it afterwards (and only if the user
/// opts into "save credentials"). Apple-ID e-mails (not secrets) live in
/// `data.json` under app-data.
#[tauri::command]
pub async fn login_new(
    handle: AppHandle,
    window: Window,
    sideloader_state: State<'_, SideloaderMutex>,
    email: String,
    password: String,
    anisette_server: String,
    save_credentials: bool,
) -> Result<(), AppError> {
    let account = login(&handle, window, &email, &password, anisette_server).await?;
    *sideloader_state.lock().unwrap() = Some(account);
    if save_credentials {
        credential_entry(&email)?.set_password(&password).map_err(|e| {
            AppError::KeyringWithMessage("Failed to save credentials".into(), e.to_string())
        })?;
        let store = handle
            .store("data.json")
            .map_err(|e| AppError::Misc(format!("Failed to get store: {e:?}")))?;
        let mut ids = store
            .get("ids")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default();
        let value = Value::String(email.clone());
        if !ids.contains(&value) {
            ids.push(value);
        }
        store.set("ids", Value::Array(ids));
    }
    Ok(())
}

#[tauri::command]
pub async fn login_stored(
    handle: AppHandle,
    window: Window,
    email: String,
    anisette_server: String,
    sideloader_state: State<'_, SideloaderMutex>,
) -> Result<(), AppError> {
    let password = credential_entry(&email)?.get_password().map_err(|e| {
        AppError::KeyringWithMessage("Failed to get credentials".to_string(), e.to_string())
    })?;
    let account = login(&handle, window, &email, &password, anisette_server).await?;
    *sideloader_state.lock().unwrap() = Some(account);
    Ok(())
}

#[tauri::command]
pub fn delete_account(handle: AppHandle, email: String) -> Result<(), AppError> {
    let store = handle
        .store("data.json")
        .map_err(|e| AppError::Misc(format!("Failed to get store: {e:?}")))?;
    let mut ids = store
        .get("ids")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    ids.retain(|v| v.as_str().is_none_or(|s| s != email));
    store.set("ids", Value::Array(ids));
    credential_entry(&email)?.delete_credential().map_err(|e| {
        AppError::KeyringWithMessage("Failed to delete credentials".into(), e.to_string())
    })?;
    Ok(())
}

#[tauri::command]
pub fn logged_in_as(sideloader_state: State<'_, SideloaderMutex>) -> Option<String> {
    sideloader_state
        .lock()
        .unwrap()
        .as_ref()
        .map(|a| a.get_email().to_string())
}

#[tauri::command]
pub fn invalidate_account(sideloader_state: State<'_, SideloaderMutex>) {
    *sideloader_state.lock().unwrap() = None;
}

#[tauri::command]
pub fn reset_anisette_state() -> Result<bool, AppError> {
    let entry = credential_entry("anisette_state")?;
    match entry.delete_credential() {
        Ok(_) => {
            debug!("Anisette state deleted from keyring.");
            Ok(true)
        }
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(AppError::KeyringWithMessage(
            "Failed to delete anisette state".into(),
            e.to_string(),
        )),
    }
}

async fn login(
    app: &AppHandle,
    window: Window,
    email: &str,
    password: &str,
    anisette_server: String,
) -> Result<Sideloader<MaxCertsCallbackBox>, AppError> {
    let anisette_url = require_anisette_url(&anisette_server)?;

    let tfa_closure = {
        let window_clone = window.clone();
        move |params: TwoFactorCallbackParams| {
            let window_clone = window_clone.clone();
            async move {
                window_clone
                    .emit("2fa-required", params)
                    .context("Failed to emit 2fa-required event")?;
                let (tx, rx) = std::sync::mpsc::channel::<String>();
                let handler_id = window_clone.listen("2fa-recieved", move |event| {
                    let _ = tx.send(event.payload().to_string());
                });
                let result = rx.recv_timeout(Duration::from_secs(120))?;
                window_clone.unlisten(handler_id);
                Ok(TwoFactorCallbackResponse::SubmitCode(
                    result.trim_matches('"').to_string(),
                ))
            }
            .boxed()
        }
    };

    let mut account = AppleAccount::builder(&email.to_lowercase())
        .anisette_provider(
            RemoteV3AnisetteProvider::default()?
                .set_serial_number("0".to_string())
                .set_storage(create_sideloading_storage(app)?)
                .set_url(&anisette_url),
        )
        .login(password, Box::new(tfa_closure))
        .await?;
    debug!("Logged in");
    let dev_session = DeveloperSession::from_account(&mut account).await?;
    debug!("Created developer session");

    let max_certs_callback: MaxCertsCallbackBox =
        Box::new(move |certs: Vec<DevelopmentCertificate>| {
            let window_clone = window.clone();
            Box::pin(async move {
                let infos: Vec<CertificateInfo> = certs
                    .iter()
                    .map(|c| CertificateInfo {
                        name: c.name.clone(),
                        certificate_id: c.certificate_id.clone(),
                        serial_number: c.serial_number.clone(),
                        machine_name: c.machine_name.clone(),
                        machine_id: c.machine_id.clone(),
                    })
                    .collect();
                window_clone.emit("max-certs-reached", infos)?;
                let (tx, rx) = std::sync::mpsc::channel::<Option<Vec<String>>>();
                let handler_id = window_clone.listen("max-certs-response", move |event| {
                    let certs =
                        serde_json::from_str::<Option<Vec<String>>>(event.payload()).unwrap_or(None);
                    let _ = tx.send(certs);
                });
                let result = rx.recv_timeout(Duration::from_secs(300));
                window_clone.unlisten(handler_id);
                Ok(result?)
            })
        });

    Ok(SideloaderBuilder::new(dev_session, email.to_lowercase())
        .machine_name(PROVIDER_LABEL.into())
        .storage(create_sideloading_storage(app)?)
        .max_certs_behavior(MaxCertsBehavior::Prompt(max_certs_callback))
        .build())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateInfo {
    pub name: Option<String>,
    pub certificate_id: Option<String>,
    pub serial_number: Option<String>,
    pub machine_name: Option<String>,
    pub machine_id: Option<String>,
}

#[tauri::command]
pub async fn get_certificates(
    sideloader_state: State<'_, SideloaderMutex>,
) -> Result<Vec<CertificateInfo>, AppError> {
    let mut s = SideloaderGuard::take(&sideloader_state)?;
    let team = s.get_mut().get_team().await?;
    let certs = s
        .get_mut()
        .get_dev_session()
        .list_all_development_certs(&team, None)
        .await?;
    Ok(certs
        .into_iter()
        .map(|c| CertificateInfo {
            name: c.name,
            certificate_id: c.certificate_id,
            serial_number: c.serial_number,
            machine_name: c.machine_name,
            machine_id: c.machine_id,
        })
        .collect())
}

#[tauri::command]
pub async fn revoke_certificate(
    serial_number: String,
    sideloader_state: State<'_, SideloaderMutex>,
) -> Result<(), AppError> {
    let mut s = SideloaderGuard::take(&sideloader_state)?;
    let team = s.get_mut().get_team().await?;
    s.get_mut()
        .get_dev_session()
        .revoke_development_cert(&team, &serial_number, None)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn list_app_ids(
    sideloader_state: State<'_, SideloaderMutex>,
) -> Result<ListAppIdsResponse, AppError> {
    let mut s = SideloaderGuard::take(&sideloader_state)?;
    let team = s.get_mut().get_team().await?;
    let response = s
        .get_mut()
        .get_dev_session()
        .list_app_ids(&team, None)
        .await?;
    Ok(response.clone())
}

#[tauri::command]
pub async fn delete_app_id(
    app_id_id: String,
    sideloader_state: State<'_, SideloaderMutex>,
) -> Result<(), AppError> {
    let mut s = SideloaderGuard::take(&sideloader_state)?;
    let team = s.get_mut().get_team().await?;
    s.get_mut()
        .get_dev_session()
        .delete_app_id(&team, &app_id_id, None)
        .await?;
    Ok(())
}
