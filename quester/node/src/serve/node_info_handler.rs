use common::NodeConfig;
use serde_json::json;
use std::sync::Arc;
use warp::{Filter, Rejection};

use crate::{BuildInfo, RuntimeInfo};

use super::with_arg;

#[derive(utoipa::OpenApi)]
#[openapi(paths(node_version_handler, node_config_handler,))]
pub struct NodeInfoApi;

pub fn node_info_handler(
	build_info: &'static BuildInfo,
	runtime_info: &'static RuntimeInfo,
	config: Arc<NodeConfig>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	node_version_handler(build_info, runtime_info).or(node_config_handler(config))
}

#[utoipa::path(get, tag = "Node Info", path = "/version")]
fn node_version_handler(
	build_info: &'static BuildInfo,
	runtime_info: &'static RuntimeInfo,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path("version")
		.and(warp::path::end())
		.and(with_arg(build_info))
		.and(with_arg(runtime_info))
		.then(get_version)
}

async fn get_version(
	build_info: &'static BuildInfo,
	runtime_info: &'static RuntimeInfo,
) -> impl warp::Reply {
	warp::reply::json(&json!({
		"build": build_info,
		"runtime": runtime_info,
	}))
}

#[utoipa::path(get, tag = "Node Info", path = "/config")]
fn node_config_handler(
	config: Arc<NodeConfig>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path("config")
		.and(warp::path::end())
		.and(with_arg(config))
		.then(get_config)
}

async fn get_config(config: Arc<NodeConfig>) -> impl warp::Reply {
	// We must redact sensitive information such as credentials.
	let config = (*config).clone();
	warp::reply::json(&config)
}
