//! Secrets: OS credential store first (DPAPI on Windows, Keychain on
//! macOS), never plaintext JSON. Filesystem fallback is explicit + warned.

use std::sync::atomic::{AtomicBool, Ordering};

use isideload::util::{
    fs_storage::FsStorage, keyring_storage::KeyringStorage, storage::SideloadingStorage,
};
use tauri::{AppHandle, Manager};
use tracing::warn;

use crate::error::AppError;

/// Keyring service name — DenizSigner never uses the upstream service name.
pub const SERVICE: &str = "denizsigner";

static FORCE_DISABLE_KEYRING: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub fn force_disable_keyring(force: bool) {
    FORCE_DISABLE_KEYRING.store(force, Ordering::Relaxed);
    if force {
        warn!("Keyring has been forcefully disabled by the user (insecure fallback).");
    }
}

#[tauri::command]
pub fn keyring_available() -> bool {
    !FORCE_DISABLE_KEYRING.load(Ordering::Relaxed) && check_keyring_available()
}

fn check_keyring_available() -> bool {
    let entry = match keyring::Entry::new(SERVICE, "test") {
        Ok(e) => e,
        Err(_) => return false,
    };
    entry.set_password("test").is_ok() && entry.get_password().is_ok()
}

pub fn credential_entry(account: &str) -> Result<keyring::Entry, AppError> {
    keyring::Entry::new(SERVICE, account).map_err(|e| {
        AppError::KeyringWithMessage("Failed to open credential entry".into(), e.to_string())
    })
}

pub fn create_sideloading_storage(app: &AppHandle) -> Result<Box<dyn SideloadingStorage>, AppError> {
    if keyring_available() {
        Ok(Box::new(KeyringStorage::new(SERVICE.to_string())))
    } else {
        warn!("Keyring unavailable: pairing/anisette data falls back to app-data dir (insecure).");
        let dir = app.path().app_data_dir().map_err(|e| {
            AppError::Misc(format!("Failed to get app data directory: {e:?}"))
        })?;
        Ok(Box::new(FsStorage::new(dir)))
    }
}
