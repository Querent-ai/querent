// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use proto::{
	layer::{
		Insight, LayerAgentType, LayerRequest, LayerResponse, LayerSessionRequest,
		LayerSessionRequestInfo, LayerSessionRequestInfoList, LayerSessionResponse, Neo4jConfig,
		PostgresConfig, StopLayerSessionRequest, StopLayerSessionResponse, StorageConfig,
	},
	semantics::StorageType,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::{reject::Rejection, Filter};

use crate::{
	extract_format_from_qs,
	layer_api::layer_service::{error::LayerError, LayerService},
	make_json_api_response,
	serve::require,
};

#[derive(utoipa::OpenApi)]
#[openapi(
	paths(
		layer_post_handler,
		layer_get_handler,
		start_layer_session_handler,
		stop_layer_session_handler,
		get_pipelines_history
	),
	components(schemas(
		LayerRequest,
		LayerResponse,
		LayerRequestParam,
		Insight,
		LayerSessionRequest,
		LayerSessionResponse,
		StorageConfig,
		PostgresConfig,
		Neo4jConfig,
		StorageType,
		StopLayerSessionRequest,
		StopLayerSessionResponse,
		LayerAgentType,
		LayerSessionRequestInfoList,
		LayerSessionRequestInfo,
	),)
)]
pub struct LayerApi;

#[utoipa::path(
	post,
	tag = "Layer",
	path = "/layer/session",
	request_body = LayerSessionRequest,
	responses(
		(status = 200, description = "Successfully started a layer session.", body = LayerSessionResponse)
	),
)]
/// Start Layer Session
/// REST POST insights handler.
pub async fn start_layer_session_handler(
	request: LayerSessionRequest,
	layer_service: Option<Arc<dyn LayerService>>,
) -> Result<LayerSessionResponse, LayerError> {
	if layer_service.is_none() {
		return Err(LayerError::Unavailable("Layer service is not available".to_string()));
	}
	let response = layer_service.unwrap().start_layer_session(request).await?;
	Ok(response)
}

pub fn start_layer_session_filter(
	layer_service: Option<Arc<dyn LayerService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("layer" / "session")
		.and(warp::body::json())
		.and(warp::post())
		.and(require(Some(layer_service)))
		.then(start_layer_session_handler)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}

#[utoipa::path(
    post,
    tag = "Layer",
    path = "/layer/search",
    request_body = LayerRequest,
    responses(
        (status = 200, description = "Successfully discovered valuable information.", body = LayerResponse)
    ),
)]
/// Make Layer (POST Variant)
///
/// REST POST insights handler.
///
/// Parses the layer request from the request body.
pub async fn layer_post_handler(
	request: LayerRequest,
	layer_service: Option<Arc<dyn LayerService>>,
) -> Result<LayerResponse, LayerError> {
	if layer_service.is_none() {
		return Err(LayerError::Unavailable("Layer service is not available".to_string()));
	}
	let response = layer_service.unwrap().layer_insights(request).await?;
	Ok(response)
}

pub fn layer_post_filter(
	layer_service: Option<Arc<dyn LayerService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("layer" / "search")
		.and(warp::body::json())
		.and(warp::post())
		.and(require(Some(layer_service)))
		.then(layer_post_handler)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}
#[utoipa::path(
    get,
    tag = "Layer",
    path = "/layer/search",
    responses(
        (status = 200, description = "Successfully discovered valuable information.", body = LayerResponse)
    ),
    params(
        LayerRequestParam,
	)
)]
/// Make Layer (GET Variant)
///
/// REST GET insights handler.
pub async fn layer_get_handler(
	request: LayerRequestParam,
	layer_service: Option<Arc<dyn LayerService>>,
) -> Result<LayerResponse, LayerError> {
	if layer_service.is_none() {
		return Err(LayerError::Unavailable("Layer service is not available".to_string()));
	}
	let request_required = LayerRequest {
		query: request.query,
		session_id: request.session_id,
		top_pairs: request.top_pairs.unwrap_or_default(),
	};
	let response = layer_service.unwrap().layer_insights(request_required).await?;
	Ok(response)
}

pub fn layer_get_filter(
	layer_service: Option<Arc<dyn LayerService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("layer" / "search")
		.and(warp::query::<LayerRequestParam>())
		.and(warp::get())
		.and(require(Some(layer_service)))
		.then(layer_get_handler)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}

#[derive(
	Debug, Default, Eq, PartialEq, Serialize, Deserialize, utoipa::IntoParams, utoipa::ToSchema,
)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub struct LayerRequestParam {
	/// The session id to use.
	pub session_id: String,
	/// The query to search for.
	pub query: String,
	/// The subject-object pairs based on the user-selected filter. This field is optional.
	pub top_pairs: Option<Vec<String>>,
}

#[utoipa::path(
	post,
	tag = "Layer",
	path = "/layer/session/stop",
	request_body = StopLayerSessionRequest,
	responses(
		(status = 200, description = "Successfully stopped the layer session.", body = StopLayerSessionResponse)
	),
)]
/// Stop Layer Session
/// REST POST insights handler.
pub async fn stop_layer_session_handler(
	request: StopLayerSessionRequest,
	layer_service: Option<Arc<dyn LayerService>>,
) -> Result<StopLayerSessionResponse, LayerError> {
	if layer_service.is_none() {
		return Err(LayerError::Unavailable("Layer service is not available".to_string()));
	}
	let response = layer_service.unwrap().stop_layer_session(request).await?;
	Ok(response)
}

pub fn stop_layer_session_filter(
	layer_service: Option<Arc<dyn LayerService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("layer" / "session" / "stop")
		.and(warp::body::json())
		.and(warp::post())
		.and(require(Some(layer_service)))
		.then(stop_layer_session_handler)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}

#[utoipa::path(
	get,
	tag = "Layer",
	path = "/layer/list",
	responses(
		(status = 200, description = "Get pipelines metadata", body = LayerSessionRequestInfoList)
	),
)]

/// Get pipelines metadata
pub async fn get_pipelines_history(
	layer_service: Option<Arc<dyn LayerService>>,
) -> Result<proto::layer::LayerSessionRequestInfoList, LayerError> {
	if layer_service.is_none() {
		return Err(LayerError::Unavailable("Layer service is not available".to_string()));
	}
	let response = layer_service.unwrap().get_layer_session_list().await?;
	Ok(response)
}

pub fn get_layer_history_handler(
	layer_service: Option<Arc<dyn LayerService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("layer" / "list")
		.and(warp::get())
		.and(require(Some(layer_service)))
		.then(get_pipelines_history)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}
