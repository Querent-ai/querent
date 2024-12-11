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

pub mod build_info;
use std::{convert::Infallible, time::Duration};

pub use build_info::*;
pub mod service;
use cluster::Cluster;
pub use service::*;
pub mod cluster_api;
pub use cluster_api::*;
pub mod format;
pub use format::*;
pub mod json_api_response;
pub use json_api_response::*;
pub mod semantic_api;
pub use semantic_api::*;
pub mod discovery_api;
pub mod insight_api;
pub use insight_api::*;
pub mod health_check_api;
pub use health_check_api::*;
pub mod openapi;
pub use openapi::*;
pub mod node_info_handler;
pub mod rest;
pub use node_info_handler::*;
pub mod metrics;
pub use metrics::*;
pub mod metrics_api;
pub use metrics_api::*;
use tokio::sync::oneshot;
use tracing::info;
pub mod web_interface;
use warp::{reject::Rejection, Filter};
pub use web_interface::*;
pub mod grpc;

const READINESS_REPORTING_INTERVAL: Duration = if cfg!(any(test, feature = "testsuite")) {
	Duration::from_millis(25)
} else {
	Duration::from_secs(10)
};

fn require<T: Clone + Send>(
	val_opt: Option<T>,
) -> impl Filter<Extract = (T,), Error = Rejection> + Clone {
	warp::any().and_then(move || {
		let val_opt_clone = val_opt.clone();
		async move {
			if let Some(val) = val_opt_clone {
				Ok(val)
			} else {
				Err(warp::reject())
			}
		}
	})
}

fn with_arg<T: Clone + Send>(arg: T) -> impl Filter<Extract = (T,), Error = Infallible> + Clone {
	warp::any().map(move || arg.clone())
}

async fn node_readiness(
	cluster: Cluster,
	grpc_readiness_signal_rx: oneshot::Receiver<()>,
	rest_readiness_signal_rx: oneshot::Receiver<()>,
) {
	if grpc_readiness_signal_rx.await.is_err() {
		// the gRPC server failed.
		return;
	};
	info!("gRPC server is ready");

	if rest_readiness_signal_rx.await.is_err() {
		// the REST server failed.
		return;
	};
	info!("REST server is ready");

	let mut interval = tokio::time::interval(READINESS_REPORTING_INTERVAL);

	loop {
		interval.tick().await;
		cluster.set_self_node_readiness(true).await;
	}
}
