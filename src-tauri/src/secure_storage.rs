//! All sensitive storage fails closed; never write a filesystem fallback.
use crate::error::AppError;
use isideload::util::{keyring_storage::KeyringStorage, storage::SideloadingStorage};
use tauri::AppHandle;
pub const SERVICE: &str = "denizsigner";

#[tauri::command]
pub fn keyring_available() -> bool {
    let Ok(entry) = credential_entry("availability-probe") else {
        return false;
    };
    let probe = uuid::Uuid::new_v4().to_string();
    if entry.set_password(&probe).is_err() {
        return false;
    }
    let matched = entry.get_password().is_ok_and(|v| v == probe);
    let deleted = entry.delete_credential().is_ok();
    matched && deleted
}
pub fn credential_entry(account: &str) -> Result<keyring::Entry, AppError> {
    keyring::Entry::new(SERVICE, account).map_err(|_| {
        AppError::Keyring("Secure credential store unavailable. Unlock your OS keyring.".into())
    })
}
pub fn create_sideloading_storage(_: &AppHandle) -> Result<Box<dyn SideloadingStorage>, AppError> {
    if !keyring_available() {
        return Err(AppError::Keyring(
            "Unlock your OS keyring. No unencrypted fallback is allowed.".into(),
        ));
    }
    Ok(Box::new(KeyringStorage::new(SERVICE.to_string())))
}
