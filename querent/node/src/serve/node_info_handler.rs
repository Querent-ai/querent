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

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use proto::config::NodeConfig;
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
