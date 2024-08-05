use log::info;
use proto::semantics::SemanticPipelineResponse;

use crate::{UpdateResult, QUERENT_SERVICES, RUNNING_PIPELINE_ID, UPDATE_RESULT};
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

#[tauri::command]
#[specta::specta]
pub async fn set_collectors(collectors: Vec<proto::semantics::CollectorConfig>) -> bool {
    let secret_store = QUERENT_SERVICES.get().unwrap().secret_store.clone();
    let result = node::serve::semantic_api::set_collectors_all(collectors, secret_store).await;
    if result.is_ok() {
        info!("Collectors set successfully");
        true
    } else {
        info!("Failed to set collectors");
        false
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_collectors() -> proto::semantics::ListCollectorConfig {
    let secret_store = QUERENT_SERVICES.get().unwrap().secret_store.clone();
    let result = node::serve::semantic_api::list_collectors(secret_store).await;
    if result.is_ok() {
        info!("Collectors retrieved successfully");
        result.unwrap()
    } else {
        info!("Failed to retrieve collectors");
        proto::semantics::ListCollectorConfig { config: vec![] }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn start_agn_fabric(
    request: proto::semantics::SemanticPipelineRequest,
) -> Result<SemanticPipelineResponse, String> {
    let secret_store = QUERENT_SERVICES.get().unwrap().secret_store.clone();
    let semantic_service_mailbox = QUERENT_SERVICES.get().unwrap().semantic_service_bus.clone();
    let event_storages = QUERENT_SERVICES.get().unwrap().event_storages.clone();
    let index_storages = QUERENT_SERVICES.get().unwrap().index_storages.clone();
    let metadata_store = QUERENT_SERVICES.get().unwrap().metadata_store.clone();

    let result = node::serve::semantic_api::start_pipeline(
        request.clone(),
        semantic_service_mailbox,
        event_storages,
        index_storages,
        secret_store,
        metadata_store,
    )
    .await;

    match result {
        Ok(response) => {
            info!("Pipeline started successfully");
            let pipeline_id = response.pipeline_id.clone();
            RUNNING_PIPELINE_ID.lock().push((pipeline_id, request));
            Ok(response)
        }
        Err(e) => {
            info!("Failed to start pipeline: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_running_pipelines() -> Vec<(String, proto::semantics::SemanticPipelineRequest)> {
    let running_pipelines = RUNNING_PIPELINE_ID.lock();
    running_pipelines.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn stop_agn_fabric(pipeline_id: String) -> Result<(), String> {
    let message_bus = QUERENT_SERVICES.get().unwrap().semantic_service_bus.clone();
    let result = node::serve::semantic_api::stop_pipeline(pipeline_id.clone(), message_bus).await;
    match result {
        Ok(_) => {
            info!("Pipeline stopped successfully");
            RUNNING_PIPELINE_ID
                .lock()
                .retain(|(id, _)| id != &pipeline_id);
            Ok(())
        }
        Err(e) => {
            info!("Failed to stop pipeline: {:?}", e);
            Err(e.to_string())
        }
    }
}
