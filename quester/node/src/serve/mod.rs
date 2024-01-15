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
pub mod health_check_api;
pub mod openapi;
pub use openapi::*;
pub mod node_info_handler;
pub mod rest;
pub use node_info_handler::*;
pub mod metrics;
pub use metrics::*;
pub mod metrics_api;
pub use metrics_api::*;
pub mod web_interface;
use tokio::sync::oneshot;
use tracing::info;
pub use web_interface::*;
pub mod grpc;

use warp::{reject::Rejection, Filter};

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
