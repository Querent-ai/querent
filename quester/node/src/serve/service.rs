use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use crate::rest;
use actors::{ActorExitStatus, MessageBus, Quester};
use cluster::{start_cluster_service, Cluster};
use common::{BoxFutureInfaillible, Host, NodeConfig, PubSubBroker, RuntimesConfig};
use querent::{start_semantic_service, SemanticService};
use querent_synapse::callbacks::EventType;
use storage::{create_storages, Storage};
use tokio::sync::oneshot;
use tracing::{debug, error};

use super::node_readiness;

const _READINESS_REPORTING_INTERVAL: Duration = if cfg!(any(test, feature = "testsuite")) {
	Duration::from_millis(25)
} else {
	Duration::from_secs(10)
};

pub struct QuesterServices {
	pub node_config: NodeConfig,
	pub cluster: Cluster,
	pub event_broker: PubSubBroker,
	pub semantic_service_bus: MessageBus<SemanticService>,
	pub event_storages: HashMap<EventType, Arc<dyn Storage>>,
	pub index_storages: Vec<Arc<dyn Storage>>,
}

pub async fn serve_quester(
	node_config: NodeConfig,
	_runtimes_config: RuntimesConfig,
	shutdown_signal: BoxFutureInfaillible<()>,
) -> anyhow::Result<HashMap<String, ActorExitStatus>> {
	let cluster = start_cluster_service(&node_config).await?;
	let event_broker = PubSubBroker::default();
	let quester_cloud = Quester::new();
	let semantic_service_bus: MessageBus<SemanticService> =
		start_semantic_service(&node_config, &quester_cloud, &cluster, &event_broker)
			.await
			.expect("Failed to start semantic service");
	let listen_host = node_config.listen_address.parse::<Host>()?;
	let listen_ip = listen_host.resolve().await?;
	let _grpc_listen_addr = SocketAddr::new(listen_ip, node_config.grpc_listen_port);
	let rest_listen_addr = SocketAddr::new(listen_ip, node_config.rest_config.listen_port);

	// Setup and start REST server.
	let (rest_readiness_trigger_tx, rest_readiness_signal_rx) = oneshot::channel::<()>();
	let rest_readiness_trigger = Box::pin(async move {
		if rest_readiness_trigger_tx.send(()).is_err() {
			debug!("REST server readiness signal receiver was dropped");
		}
	});
	let (rest_shutdown_trigger_tx, rest_shutdown_signal_rx) = oneshot::channel::<()>();
	let rest_shutdown_signal = Box::pin(async move {
		if rest_shutdown_signal_rx.await.is_err() {
			debug!("REST server shutdown trigger sender was dropped");
		}
	});
	let (event_storages, index_storages) = create_storages(&node_config.storage_configs.0).await?;
	let services = Arc::new(QuesterServices {
		node_config,
		cluster: cluster.clone(),
		event_broker,
		semantic_service_bus,
		event_storages,
		index_storages,
	});
	let rest_server = rest::start_rest_server(
		rest_listen_addr,
		services,
		rest_readiness_trigger,
		rest_shutdown_signal,
	);
	// Setup and start gRPC server.
	let (grpc_readiness_trigger_tx, grpc_readiness_signal_rx) = oneshot::channel::<()>();
	let _grpc_readiness_trigger = Box::pin(async move {
		if grpc_readiness_trigger_tx.send(()).is_err() {
			debug!("gRPC server readiness signal receiver was dropped 📡");
		}
	});
	let (grpc_shutdown_trigger_tx, grpc_shutdown_signal_rx) = oneshot::channel::<()>();
	let _grpc_shutdown_signal = Box::pin(async move {
		if grpc_shutdown_signal_rx.await.is_err() {
			debug!("gRPC server shutdown trigger sender was dropped 📡");
		}
	});
	// TODO implement gRPC server
	tokio::spawn(node_readiness(
		cluster.clone(),
		grpc_readiness_signal_rx,
		rest_readiness_signal_rx,
	));
	let shutdown_handle = tokio::spawn(async move {
		shutdown_signal.await;
		debug!("Shutting down node 📴");
		let actor_exit_statuses = quester_cloud.quit().await;

		if grpc_shutdown_trigger_tx.send(()).is_err() {
			debug!("gRPC server shutdown signal receiver was dropped");
		}
		if rest_shutdown_trigger_tx.send(()).is_err() {
			debug!("REST server shutdown signal receiver was dropped");
		}
		actor_exit_statuses
	});
	let rest_join_handle = tokio::spawn(rest_server);
	let (rest_res,) = tokio::try_join!(rest_join_handle)
		.expect("the tasks running the gRPC and REST servers should not panic or be cancelled");
	if let Err(rest_err) = rest_res {
		error!("REST server failed: {:?}", rest_err);
	}
	let actor_exit_statuses = shutdown_handle.await?;
	Ok(actor_exit_statuses)
}
