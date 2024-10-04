use insights::InsightInfo;
use log::info;
use proto::{
    semantics::IndexingStatistics, semantics::IngestedTokens, semantics::SemanticPipelineResponse,
    InsightAnalystResponse,
};

use crate::{
    UpdateResult, QUERENT_SERVICES_ONCE, RUNNING_DISCOVERY_SESSION_ID, RUNNING_INSIGHTS_SESSIONS,
    RUNNING_PIPELINE_ID, UPDATE_RESULT,
};
use node::ApiKeyPayload;
use serde_urlencoded;
use tiny_http::{Header, Response, Server};

use lazy_static::lazy_static;
lazy_static! {
    static ref DRIVE_CLIENT_ID: &'static str =
        "400694410965-n2p8mud6m9sh1bso14r5hp7o6m4lnfug.apps.googleusercontent.com";
}

lazy_static! {
    static ref DRIVE_CLIENT_SECRET: &'static str = "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";
}

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
    QUERENT_SERVICES_ONCE.get().is_some()
}

#[tauri::command]
#[specta::specta]
pub async fn has_rian_license_key() -> bool {
    let secret_store = QUERENT_SERVICES_ONCE.get().unwrap().secret_store.clone();
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
    let secret_store = QUERENT_SERVICES_ONCE.get().unwrap().secret_store.clone();
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
    let secret_store = QUERENT_SERVICES_ONCE.get().unwrap().secret_store.clone();
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
    let secret_store = QUERENT_SERVICES_ONCE.get().unwrap().secret_store.clone();
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
    let secret_store = QUERENT_SERVICES_ONCE.get().unwrap().secret_store.clone();
    let semantic_service_mailbox = QUERENT_SERVICES_ONCE
        .get()
        .unwrap()
        .semantic_service_bus
        .clone();
    let event_storages = QUERENT_SERVICES_ONCE.get().unwrap().event_storages.clone();
    let index_storages = QUERENT_SERVICES_ONCE.get().unwrap().index_storages.clone();
    let metadata_store = QUERENT_SERVICES_ONCE.get().unwrap().metadata_store.clone();
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
pub async fn get_running_agns() -> Vec<(String, proto::semantics::SemanticPipelineRequest)> {
    let running_pipelines = RUNNING_PIPELINE_ID.lock();
    running_pipelines.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn get_past_agns() -> Result<proto::semantics::PipelineRequestInfoList, String> {
    let metadata_store = QUERENT_SERVICES_ONCE.get().unwrap().metadata_store.clone();
    let result = node::serve::semantic_api::rest::get_pipelines_history(metadata_store).await;
    match result {
        Ok(response) => {
            info!("Past pipelines retrieved successfully");
            Ok(response)
        }
        Err(e) => {
            info!("Failed to retrieve past pipelines: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn stop_agn_fabric(pipeline_id: String) -> Result<(), String> {
    let message_bus = QUERENT_SERVICES_ONCE
        .get()
        .unwrap()
        .semantic_service_bus
        .clone();
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

#[tauri::command]
#[specta::specta]
pub async fn send_discovery_retriever_request(
    search_query: String,
    top_pairs: Vec<String>,
) -> Result<proto::discovery::DiscoveryResponse, String> {
    let discovery_session_id = RUNNING_DISCOVERY_SESSION_ID.lock().clone();
    if discovery_session_id.is_empty() {
        let discovery_session_request = proto::discovery::DiscoverySessionRequest {
            agent_name: "R!AN".to_string(),
            semantic_pipeline_id: "".to_string(),
            session_type: Some(proto::discovery::DiscoveryAgentType::Retriever),
        };
        let discover_service = QUERENT_SERVICES_ONCE
            .get()
            .unwrap()
            .discovery_service
            .clone();
        let result = node::serve::discovery_api::start_discovery_session_handler(
            discovery_session_request.clone(),
            discover_service,
        )
        .await;
        match result {
            Ok(response) => {
                info!("Discovery session started successfully");
                *RUNNING_DISCOVERY_SESSION_ID.lock() = response.session_id.clone();
            }
            Err(e) => {
                info!("Failed to start discovery session: {:?}", e);
                return Err(e.to_string());
            }
        }
    }
    let discovery_session_id = RUNNING_DISCOVERY_SESSION_ID.lock().clone();
    if discovery_session_id.is_empty() {
        return Err("Failed to start discovery session".to_string());
    }
    let discovery_request = proto::discovery::DiscoveryRequest {
        query: search_query.clone(),
        session_id: discovery_session_id,
        top_pairs: top_pairs.clone(),
    };
    let discover_service = QUERENT_SERVICES_ONCE
        .get()
        .unwrap()
        .discovery_service
        .clone();
    let result =
        node::serve::discovery_api::discovery_post_handler(discovery_request, discover_service)
            .await;
    match result {
        Ok(response) => {
            info!("Discovery request handled successfully");
            Ok(response)
        }
        Err(e) => {
            info!("Failed to handle discovery request: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn list_available_insights() -> Result<Vec<InsightInfo>, String> {
    let result = node::serve::insight_api::rest::list_insights().await;
    match result {
        Ok(response) => {
            info!("Insights retrieved successfully");
            Ok(response)
        }
        Err(e) => {
            info!("Failed to retrieve insights: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn list_past_insights() -> Result<proto::insights::InsightRequestInfoList, String> {
    let insight_service = QUERENT_SERVICES_ONCE.get().unwrap().insight_service.clone();
    let result = node::serve::insight_api::rest::get_pipelines_history(insight_service).await;
    match result {
        Ok(response) => {
            info!("Insights retrieved successfully");
            Ok(response)
        }
        Err(e) => {
            info!("Failed to retrieve insights: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn trigger_insight_analyst(
    request: proto::insights::InsightAnalystRequest,
) -> Result<InsightAnalystResponse, String> {
    let insight_service = QUERENT_SERVICES_ONCE.get().unwrap().insight_service.clone();
    let result = node::serve::insight_api::rest::start_insight_session_handler(
        request.clone(),
        insight_service,
    )
    .await;
    match result {
        Ok(response) => {
            info!("Insight triggered successfully");
            RUNNING_INSIGHTS_SESSIONS
                .lock()
                .push((response.session_id.clone(), request));
            Ok(response)
        }
        Err(e) => {
            info!("Failed to trigger insight: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_running_insight_analysts() -> Vec<(String, proto::insights::InsightAnalystRequest)>
{
    let running_insights = RUNNING_INSIGHTS_SESSIONS.lock();
    running_insights.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn stop_insight_analyst(session_id: String) -> Result<(), String> {
    let insight_service = QUERENT_SERVICES_ONCE.get().unwrap().insight_service.clone();
    let stop_req = proto::insights::StopInsightSessionRequest {
        session_id: session_id.clone(),
    };
    let result =
        node::serve::insight_api::rest::stop_insight_session_handler(stop_req, insight_service)
            .await;
    match result {
        Ok(_) => {
            info!("Insight stopped successfully");
            RUNNING_INSIGHTS_SESSIONS
                .lock()
                .retain(|(id, _)| id != &session_id);
            Ok(())
        }
        Err(e) => {
            info!("Failed to stop insight: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn prompt_insight_analyst(
    request: proto::insights::InsightQuery,
) -> Result<proto::insights::InsightQueryResponse, String> {
    let insight_service = QUERENT_SERVICES_ONCE.get().unwrap().insight_service.clone();
    let result =
        node::serve::insight_api::rest::insights_prompt_handler(request, insight_service).await;
    match result {
        Ok(response) => {
            info!("Insight prompted successfully");
            Ok(response)
        }
        Err(e) => {
            info!("Failed to prompt insight: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_drive_credentials() -> Result<(String, String), String> {
    let drive_client_id: String = DRIVE_CLIENT_ID.to_string();
    let drive_client_secret: String = DRIVE_CLIENT_SECRET.to_string();

    Ok((drive_client_id, drive_client_secret))
}

#[tauri::command]
#[specta::specta]
pub async fn delete_collectors(id: proto::semantics::DeleteCollectorRequest) -> bool {
    let secret_store = QUERENT_SERVICES_ONCE.get().unwrap().secret_store.clone();
    let result = node::serve::semantic_api::delete_collectors(id, secret_store).await;
    if result.is_ok() {
        info!("Source delete successfully");
        true
    } else {
        info!("Failed to delete Source");
        false
    }
}

#[tauri::command]
#[specta::specta]
pub async fn ingest_tokens(tokens: Vec<IngestedTokens>, pipeline_id: String) -> bool {
    let semantic_service_mailbox = QUERENT_SERVICES_ONCE
        .get()
        .unwrap()
        .semantic_service_bus
        .clone();

    let result =
        node::serve::semantic_api::ingest_tokens(pipeline_id, tokens, semantic_service_mailbox)
            .await;
    if result.is_ok() {
        info!("Data ingested successfully");
        true
    } else {
        info!("Failed to ingest data");
        false
    }
}

#[tauri::command]
#[specta::specta]
pub async fn describe_pipeline(pipeline_id: String) -> Result<IndexingStatistics, String> {
    let semantic_service_mailbox = QUERENT_SERVICES_ONCE.get();
    if semantic_service_mailbox.is_none() {
        return Err("Semantic service not found".to_string());
    }
    let semantic_service_mailbox = semantic_service_mailbox
        .unwrap()
        .semantic_service_bus
        .clone();
    let result =
        node::serve::semantic_api::describe_pipeline(pipeline_id, semantic_service_mailbox).await;
    match result {
        Ok(response) => Ok(response),
        Err(e) => {
            return Err(e.to_string());
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn start_oauth_server() -> Result<String, String> {
    let server = Server::http("localhost:5174").unwrap();

    for request in server.incoming_requests() {
        if request.url().starts_with("/confirmation") {
            let url = request.url().to_string();

            if let Some(query_idx) = url.find('?') {
                let query_str = &url[query_idx + 1..];
                if let Ok(params) = serde_urlencoded::from_str::<Vec<(String, String)>>(query_str) {
                    for (key, value) in params {
                        if key == "code" {
                            let html_response = format!(
                                r#"
                                <html>
                                    <head><title>Authorization Success</title></head>
                                    <body>
                                        <h1>Authorization Successful</h1>
                                        <p>You can close this page and continue on the app now.</p>
                                    </body>
                                </html>
                                "#,
                            );
                            let response =
                                Response::from_string(html_response).with_header(Header {
                                    field: "Content-Type".parse().unwrap(),
                                    value: "text/html; charset=UTF-8".parse().unwrap(),
                                });
                            request.respond(response).unwrap();
                            // Return the code from here
                            return Ok(value);
                        }
                    }
                }
            }

            let response = Response::from_string("Waiting for code...");
            request.respond(response).unwrap();
        } else {
            let response = Response::from_string("Invalid endpoint");
            request.respond(response).unwrap();
        }
    }

    Err("Failed to capture the code".to_string())
}
