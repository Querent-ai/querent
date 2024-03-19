use std::{net::SocketAddr, sync::Arc};

use bytesize::ByteSize;
use cluster::cluster_grpc_server;
use common::BoxFutureInfaillible;
use proto::semantics_service_grpc_server::SemanticsServiceGrpcServer;
use tonic::transport::Server;
use tracing::info;

use crate::{QuesterServices, SemanticsGrpcAdapter};

/// Starts and binds gRPC services to `grpc_listen_addr`.
pub(crate) async fn start_grpc_server(
	grpc_listen_addr: SocketAddr,
	max_message_size: ByteSize,
	services: Arc<QuesterServices>,
	readiness_trigger: BoxFutureInfaillible<()>,
	shutdown_signal: BoxFutureInfaillible<()>,
) -> anyhow::Result<()> {
	let mut server = Server::builder();
	let cluster_grpc_service = cluster_grpc_server(services.cluster.clone());
	let grpc_semantic_adapater = SemanticsGrpcAdapter::new(
		services.semantic_service_bus.clone(),
		services.event_storages.clone(),
		services.index_storages.clone(),
	);

	let semantics_service_grpc_server = SemanticsServiceGrpcServer::new(grpc_semantic_adapater)
		.max_decoding_message_size(max_message_size.0 as usize)
		.max_encoding_message_size(max_message_size.0 as usize);

	let server_router = server
		.add_service(cluster_grpc_service)
		.add_service(semantics_service_grpc_server);

	info!(
		grpc_listen_addr=?grpc_listen_addr,
		"Starting gRPC server listening on {grpc_listen_addr}."
	);
	let serve_fut = server_router.serve_with_shutdown(grpc_listen_addr, shutdown_signal);
	let (serve_res, _trigger_res) = tokio::join!(serve_fut, readiness_trigger);
	serve_res?;
	Ok(())
}
