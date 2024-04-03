use discovery::{error::DiscoveryError, DiscoveryService};
use proto::{
	discovery::{
		DiscoveryRequest, DiscoveryResponse, DiscoverySessionRequest, DiscoverySessionResponse,
		Insight, MilvusConfig, Neo4jConfig, PostgresConfig, StorageConfig,
	},
	semantics::StorageType,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::{reject::Rejection, Filter};

use crate::{extract_format_from_qs, make_json_api_response, serve::require};

#[derive(utoipa::OpenApi)]
#[openapi(
	paths(discovery_post_handler, discovery_get_handler,),
	components(schemas(
		DiscoveryRequest,
		DiscoveryResponse,
		DiscoveryRequestParam,
		Insight,
		DiscoverySessionRequest,
		DiscoverySessionResponse,
		StorageConfig,
		PostgresConfig,
		Neo4jConfig,
		MilvusConfig,
		StorageType
	),)
)]
pub struct DiscoveryApi;

#[utoipa::path(
	post,
	tag = "Discover",
	path = "/discovery/session",
	request_body = DiscoverySessionRequest,
	responses(
		(status = 200, description = "Successfully started a discovery session.", body = DiscoverySessionResponse)
	),
)]
/// Start Discovery Session
/// REST POST insights handler.
pub async fn start_discovery_session_handler(
	request: DiscoverySessionRequest,
	discovery_service: Option<Arc<dyn DiscoveryService>>,
) -> Result<DiscoverySessionResponse, DiscoveryError> {
	if discovery_service.is_none() {
		return Err(DiscoveryError::Unavailable("Discovery service is not available".to_string()));
	}
	let response = discovery_service.unwrap().start_discovery_session(request).await?;
	Ok(response)
}

pub fn start_discovery_session_filter(
	discovery_service: Option<Arc<dyn DiscoveryService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("session")
		.and(warp::body::json())
		.and(warp::post())
		.and(require(Some(discovery_service)))
		.then(start_discovery_session_handler)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}

#[utoipa::path(
    post,
    tag = "Discover",
    path = "/discovery/search",
    request_body = DiscoveryRequest,
    responses(
        (status = 200, description = "Successfully discovered valuable information.", body = DiscoveryResponse)
    ),
)]
/// Make Discovery (POST Variant)
///
/// REST POST insights handler.
///
/// Parses the discovery request from the request body.
pub async fn discovery_post_handler(
	request: DiscoveryRequest,
	discovery_service: Option<Arc<dyn DiscoveryService>>,
) -> Result<DiscoveryResponse, DiscoveryError> {
	if discovery_service.is_none() {
		return Err(DiscoveryError::Unavailable("Discovery service is not available".to_string()));
	}
	let response = discovery_service.unwrap().discover_insights(request).await?;
	Ok(response)
}

pub fn discover_post_filter(
	discovery_service: Option<Arc<dyn DiscoveryService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("search")
		.and(warp::body::json())
		.and(warp::post())
		.and(require(Some(discovery_service)))
		.then(discovery_post_handler)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}
#[utoipa::path(
    get,
    tag = "Discover",
    path = "/discovery/search",
    responses(
        (status = 200, description = "Successfully discovered valuable information.", body = DiscoveryResponse)
    ),
    params(
        DiscoveryRequestParam,
	)
)]
/// Make Discovery (GET Variant)
///
/// REST GET insights handler.
pub async fn discovery_get_handler(
	request: DiscoveryRequestParam,
	discovery_service: Option<Arc<dyn DiscoveryService>>,
) -> Result<DiscoveryResponse, DiscoveryError> {
	if discovery_service.is_none() {
		return Err(DiscoveryError::Unavailable("Discovery service is not available".to_string()));
	}
	let request_required =
		DiscoveryRequest { query: request.query, session_id: request.session_id };
	let response = discovery_service.unwrap().discover_insights(request_required).await?;
	Ok(response)
}

pub fn discover_get_filter(
	discovery_service: Option<Arc<dyn DiscoveryService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("search")
		.and(warp::query::<DiscoveryRequestParam>())
		.and(warp::get())
		.and(require(Some(discovery_service)))
		.then(discovery_get_handler)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}

#[derive(
	Debug, Default, Eq, PartialEq, Serialize, Deserialize, utoipa::IntoParams, utoipa::ToSchema,
)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryRequestParam {
	/// The session id to use.
	pub session_id: String,
	/// The query to search for.
	pub query: String,
}
