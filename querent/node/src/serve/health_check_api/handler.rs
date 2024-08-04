use std::sync::Arc;

use actors::{Healthz, MessageBus};
use cluster::Cluster;
use common::ServiceErrorCode;
use rian::{verify_key, SemanticService};
use serde::{Deserialize, Serialize};
use storage::Storage;
use tracing::error;
use warp::{hyper::StatusCode, reply::with_status, Filter, Rejection};

use crate::{
	extract_format_from_qs, make_json_api_response,
	serve::{require, with_arg},
	ApiError,
};

#[derive(utoipa::OpenApi)]
#[openapi(
	paths(get_liveness, get_readiness, set_api_key, get_api_key),
	components(schemas(ApiKeyPayload, ApiKeyResponse))
)]
pub struct HealthCheckApi;

/// Health check handlers.
pub fn health_check_handlers(
	cluster: Cluster,
	semantic_service: Option<MessageBus<SemanticService>>,
	secret_store: Arc<dyn Storage>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	liveness_handler(semantic_service)
		.or(readiness_handler(cluster))
		.or(set_api_key_handler(secret_store.clone()))
		.or(get_api_key_handler(secret_store))
}

fn liveness_handler(
	semantic_service: Option<MessageBus<SemanticService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("health" / "livez")
		.and(warp::get())
		.and(with_arg(semantic_service))
		.then(get_liveness)
}

fn readiness_handler(
	cluster: Cluster,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("health" / "readyz")
		.and(warp::get())
		.and(with_arg(cluster))
		.then(get_readiness)
}

#[utoipa::path(
    get,
    tag = "Node Health",
    path = "/livez",
    responses(
        (status = 200, description = "The service is live.", body = bool),
        (status = 503, description = "The service is not live.", body = bool),
    ),
)]
/// Get Node Liveliness
async fn get_liveness(semantic_service: Option<MessageBus<SemanticService>>) -> impl warp::Reply {
	let mut is_live = true;

	if let Some(semantic_service) = semantic_service {
		if !semantic_service.ask(Healthz).await.unwrap_or(false) {
			error!("the semantic service is unhealthy");
			is_live = false;
		}
	}
	let status_code = if is_live { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
	with_status(warp::reply::json(&is_live), status_code)
}

#[utoipa::path(
    get,
    tag = "Node Health",
    path = "/readyz",
    responses(
        (status = 200, description = "The service is ready.", body = bool),
        (status = 503, description = "The service is not ready.", body = bool),
    ),
)]
/// Get Node Readiness
async fn get_readiness(cluster: Cluster) -> impl warp::Reply {
	let is_ready = cluster.is_self_node_ready().await;
	let status_code = if is_ready { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
	with_status(warp::reply::json(&is_ready), status_code)
}

/// API to set the API Key in secret_store: Arc<dyn Storage>
/// This API is used to set the API Key for RIAN in the secret store.
/// The API Key is used to authenticate the RIAN service.
/// API key is a string as is sent to node to be stored in secret store.
/// Below are post and get apis for setting and getting the API Key.
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiKeyPayload {
	pub key: String,
}

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiKeyResponse {
	pub key: Option<String>,
}

#[utoipa::path(
	post,
	tag = "Node Health",
	path = "/apikey",
	responses(
		(status = 200, description = "The API Key is set.", body = ApiKeyResponse),
		(status = 400, description = "The API Key is not set.", body = ApiKeyResponse),
	),
)]
/// Set the API Key
pub async fn set_api_key(
	payload: ApiKeyPayload,
	secret_store: Arc<dyn Storage>,
) -> Result<ApiKeyPayload, ApiError> {
	let key = payload.key;
	let key = key.trim();
	if key.is_empty() {
		return Err(ApiError {
			service_code: ServiceErrorCode::BadRequest,
			message: "API Key is empty".to_string(),
		})
	}

	let key = key.to_string();
	let valid_license_key = verify_key(key.clone()).map_err(|e| {
		error!("failed to verify the Licence Key: {}", e);
		ApiError {
			service_code: ServiceErrorCode::BadRequest,
			message: "Invalid Licence Key".to_string(),
		}
	})?;
	if !valid_license_key {
		return Err(ApiError {
			service_code: ServiceErrorCode::BadRequest,
			message: "Invalid Licence Key".to_string(),
		});
	}
	secret_store.set_rian_api_key(&key).await.map_err(|e| {
		error!("failed to set the API Key: {}", e);
		ApiError {
			service_code: ServiceErrorCode::Internal,
			message: "Failed to set the API Key".to_string(),
		}
	})?;
	Ok(ApiKeyPayload { key })
}

/// set api key fileter
pub fn set_api_key_handler(
	secret_store: Arc<dyn Storage>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("health" / "apikey")
		.and(warp::body::json())
		.and(warp::post())
		.and(require(Some(secret_store)))
		.then(set_api_key)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}

#[utoipa::path(
	get,
	tag = "Node Health",
	path = "/apikey",
	responses(
		(status = 200, description = "The API Key is fetched.", body = ApiKeyResponse),
		(status = 400, description = "The API Key is not fetched.", body = ApiKeyResponse),
	),
)]
/// Get the API Key
/// This API is used to get the API Key for RIAN from the secret store.
/// The API Key is used to authenticate the RIAN service.
/// API key is a string as is sent to node to be stored in secret store.
pub async fn get_api_key(secret_store: Arc<dyn Storage>) -> Result<ApiKeyResponse, ApiError> {
	let key = secret_store.get_rian_api_key().await.map_err(|e| {
		error!("failed to get the API Key: {}", e);
		ApiError {
			service_code: ServiceErrorCode::Internal,
			message: "Failed to get the API Key".to_string(),
		}
	})?;
	Ok(ApiKeyResponse { key })
}

/// get api key fileter
pub fn get_api_key_handler(
	secret_store: Arc<dyn Storage>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("health" / "apikey")
		.and(warp::get())
		.and(require(Some(secret_store)))
		.then(get_api_key)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}
