use actors::{Healthz, MessageBus};
use cluster::Cluster;
use querent::SemanticService;
use tracing::error;
use warp::{hyper::StatusCode, reply::with_status, Filter, Rejection};

use crate::serve::with_arg;

#[derive(utoipa::OpenApi)]
#[openapi(paths(get_liveness, get_readiness))]
pub struct HealthCheckApi;

/// Health check handlers.
pub(crate) fn health_check_handlers(
	cluster: Cluster,
	semantic_service: Option<MessageBus<SemanticService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	liveness_handler(semantic_service).or(readiness_handler(cluster))
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

	if let Some(indexer_service) = semantic_service {
		if !indexer_service.ask(Healthz).await.unwrap_or(false) {
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
