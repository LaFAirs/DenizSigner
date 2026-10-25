mod account;
mod credentials;
mod device;
mod error;
mod ipa;
mod logging;
mod network_allowlist;
mod operation;
mod pairing;
mod secure_storage;
mod sideload;

use crate::{
    account::{
        delete_account, delete_app_id, get_certificates, invalidate_account, list_app_ids,
        logged_in_as, login_new, login_stored, reset_anisette_state, revoke_certificate,
    },
    device::{
        DeviceInfoMutex, PairingCancelToken, cancel_pairing, list_devices, set_selected_device,
    },
    error::friendly_error,
    logging::set_log_level_debug,
    pairing::{
        delete_stored_rppairing, export_pairing_cmd, has_stored_rppairing, installed_pairing_apps,
        place_pairing_cmd,
    },
    secure_storage::keyring_available,
    sideload::{SideloaderMutex, install_sidestore_operation, sideload_operation},
};
use tauri::Manager;
use tracing_subscriber::{Layer, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    keyring::set_default_credential_builder(keyring::secret_service::default_credential_builder());
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let directory = app.path().app_data_dir()?;
                std::fs::create_dir_all(&directory)?;
                std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
            }
            let log_dir = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir")
                .join("logs");
            std::fs::create_dir_all(&log_dir).ok();

            let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("denizsigner")
                .filename_suffix("log")
                .max_log_files(7)
                .build(&log_dir)
                .expect("failed to create log file appender");

            let file_layer = fmt::layer()
                .event_format(logging::RedactedFormat)
                .with_writer(file_appender)
                .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG);

            let frontend_layer = logging::FrontendLoggingLayer::new(app.handle().clone())
                .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG);

            Registry::default()
                .with(file_layer)
                .with(frontend_layer)
                .init();

            std::panic::set_hook(Box::new(|panic_info| {
                let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                    s.clone()
                } else {
                    "<non-string panic payload>".to_string()
                };
                let location = panic_info
                    .location()
                    .map(|loc| format!("{}:{}", loc.file(), loc.line()))
                    .unwrap_or_else(|| "<unknown>".to_string());
                tracing::error!(target: "panic", location = location, message = message, "panic captured");
            }));

            #[cfg(target_os = "linux")]
            if std::env::var("DENIZSIGNER_HYPRBARS").as_deref() == Ok("1") {
                if let Some(window) = app.get_webview_window("main") {
                    window.set_decorations(false)?;
                }
            }
            app.manage(credentials::CredentialAccess::default());
            app.manage(DeviceInfoMutex::new(None));
            app.manage(SideloaderMutex::new(None));
            app.manage(PairingCancelToken::new(None));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login_new,
            invalidate_account,
            logged_in_as,
            login_stored,
            delete_account,
            list_devices,
            sideload_operation,
            set_selected_device,
            install_sidestore_operation,
            get_certificates,
            revoke_certificate,
            list_app_ids,
            delete_app_id,
            installed_pairing_apps,
            place_pairing_cmd,
            reset_anisette_state,
            export_pairing_cmd,
            delete_stored_rppairing,
            keyring_available,
            credentials::credential_unlock,
            credentials::credential_lock,
            credentials::credential_list,
            credentials::credential_import,
            credentials::credential_remove,
            credentials::credential_select,
            credentials::credential_discover,
            credentials::credential_save_token,
            credentials::credential_sign_ipa,
            friendly_error,
            set_log_level_debug,
            cancel_pairing,
            has_stored_rppairing,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
