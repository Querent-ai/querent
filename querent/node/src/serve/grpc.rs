use std::{net::SocketAddr, sync::Arc};

use bytesize::ByteSize;
use cluster::cluster_grpc_server;
use common::BoxFutureInfaillible;
use proto::{
	discovery_server::DiscoveryServer,
	semantics::semantics_service_grpc_server::SemanticsServiceGrpcServer,
};
use tonic::transport::Server;
use tracing::info;

use crate::{
	discovery_api::grpc_discovery_adapter::DiscoveryAdapter,
	insight_api::grpc_insight_adapter::InsightAdapter, QuerentServices, SemanticsGrpcAdapter,
};

/// Starts and binds gRPC services to `grpc_listen_addr`.
pub(crate) async fn start_grpc_server(
	grpc_listen_addr: SocketAddr,
	max_message_size: ByteSize,
	services: Arc<QuerentServices>,
	readiness_trigger: BoxFutureInfaillible<()>,
	shutdown_signal: BoxFutureInfaillible<()>,
) -> anyhow::Result<()> {
	let mut server = Server::builder();
	let cluster_grpc_service = cluster_grpc_server(services.cluster.clone());
	let grpc_semantic_adapater = SemanticsGrpcAdapter::new(
		services.semantic_service_bus.clone(),
		services.event_storages.clone(),
		services.index_storages.clone(),
		services.secret_store.clone(),
		services.metadata_store.clone(),
	);

	let semantics_service_grpc_server = SemanticsServiceGrpcServer::new(grpc_semantic_adapater)
		.max_decoding_message_size(max_message_size.0 as usize)
		.max_encoding_message_size(max_message_size.0 as usize);
	let discovery_grpc_service = if services.discovery_service.is_some() {
		let discovery_service = services.discovery_service.clone().unwrap();
		let grpc_discovery_adapater = DiscoveryAdapter::from(discovery_service);
		Some(
			DiscoveryServer::new(grpc_discovery_adapater)
				.max_decoding_message_size(max_message_size.0 as usize)
				.max_encoding_message_size(max_message_size.0 as usize),
		)
	} else {
		None
	};
	let insights_grpc_service = if services.insight_service.is_some() {
		let insight_service = services.insight_service.clone().unwrap();
		let grpc_insight_adapater = InsightAdapter::from(insight_service);
		Some(
			proto::insights::insight_service_server::InsightServiceServer::new(
				grpc_insight_adapater,
			)
			.max_decoding_message_size(max_message_size.0 as usize)
			.max_encoding_message_size(max_message_size.0 as usize),
		)
	} else {
		None
	};
	let server_router = server
		.add_service(cluster_grpc_service)
		.add_service(semantics_service_grpc_server)
		.add_optional_service(discovery_grpc_service)
		.add_optional_service(insights_grpc_service);

	info!(
		grpc_listen_addr=?grpc_listen_addr,
		"Starting gRPC server listening on {grpc_listen_addr}."
	);
	let serve_fut = server_router.serve_with_shutdown(grpc_listen_addr, shutdown_signal);
	let (serve_res, _trigger_res) = tokio::join!(serve_fut, readiness_trigger);
	serve_res?;
	Ok(())
}
