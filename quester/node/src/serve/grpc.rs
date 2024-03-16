use std::{net::SocketAddr, sync::Arc};

use cluster::cluster_grpc_server;
use common::{BoxFutureInfaillible, GrpcMetricsLayer};
use once_cell::sync::Lazy;
use proto::semantics::SemanticsServiceClient;
use tonic::transport::Server;

use crate::QuesterServices;

pub(crate) static SEMANTIC_GRPC_SERVER_METRICS_LAYER: Lazy<GrpcMetricsLayer> =
	Lazy::new(|| GrpcMetricsLayer::new("semantics", "server"));

/// Starts and binds gRPC services to `grpc_listen_addr`.
pub(crate) async fn start_grpc_server(
	grpc_listen_addr: SocketAddr,
	services: Arc<QuesterServices>,
	readiness_trigger: BoxFutureInfaillible<()>,
	shutdown_signal: BoxFutureInfaillible<()>,
) -> anyhow::Result<()> {
	let mut server = Server::builder();
	let cluster_grpc_service = cluster_grpc_server(services.cluster.clone());
	let semantics_grpc_service = SemanticsServiceClient::tower()
		.stack_layer(SEMANTIC_GRPC_SERVER_METRICS_LAYER.clone())
		.build_from_mailbox(services.semantic_service_bus.clone());
	Ok(())
}
