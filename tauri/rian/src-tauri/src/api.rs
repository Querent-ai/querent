use log::info;

use crate::{UpdateResult, QUERENT_SERVICES, UPDATE_RESULT};
use node::ApiKeyPayload;

#[tauri::command]
#[specta::specta]
pub fn get_update_result() -> (bool, Option<UpdateResult>) {
    let result = UPDATE_RESULT.lock();
    if let Some(update_result) = &*result {
        (true, update_result.clone())
    } else {
        (false, None)
    }
}

#[tauri::command]
#[specta::specta]
pub fn check_if_service_is_running() -> bool {
    QUERENT_SERVICES.get().is_some()
}

#[tauri::command]
#[specta::specta]
pub async fn has_rian_license_key() -> bool {
    let secret_store = QUERENT_SERVICES.get().unwrap().secret_store.clone();
    let result = node::serve::health_check_api::get_api_key(secret_store).await;
    if result.is_ok() {
        match result {
            Ok(key) => {
                if key.key.is_none() {
                    info!("License key not found");
                    false
                } else {
                    info!("License key found");
                    true
                }
            }
            Err(_) => {
                info!("License key not found");
                false
            }
        }
    } else {
        info!("License key not found");
        false
    }
}

#[tauri::command]
#[specta::specta]
pub async fn set_rian_license_key(key: String) -> bool {
    let secret_store = QUERENT_SERVICES.get().unwrap().secret_store.clone();
    let payload = ApiKeyPayload { key };
    let result = node::serve::health_check_api::set_api_key(payload, secret_store).await;
    if result.is_ok() {
        info!("License key set successfully");
        true
    } else {
        info!("Failed to set license key");
        false
    }
}
