use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use crate::{
	discovery_api::discovery_service::{start_discovery_service, DiscoveryService},
	grpc,
	insight_api::insights_service::InsightService,
	rest,
};
use actors::{ActorExitStatus, MessageBus, Quester};
use cluster::{start_cluster_service, Cluster};
use common::{BoxFutureInfaillible, EventType, Host, PubSubBroker, RuntimesConfig};
use proto::config::NodeConfig;
use querent::{start_semantic_service, SemanticService};
use storage::{create_secret_store, create_storages, Storage};
use tokio::sync::oneshot;
use tracing::{debug, error, info};

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
	pub discovery_service: Option<Arc<dyn DiscoveryService>>,
	pub insight_service: Option<Arc<dyn InsightService>>,
	pub event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	pub index_storages: Vec<Arc<dyn Storage>>,
	pub secret_store: Arc<dyn Storage>,
}

pub async fn serve_quester(
	node_config: NodeConfig,
	_runtimes_config: RuntimesConfig,
	shutdown_signal: BoxFutureInfaillible<()>,
) -> anyhow::Result<HashMap<String, ActorExitStatus>> {
	let cluster = start_cluster_service(&node_config).await?;
	let event_broker = PubSubBroker::default();
	let quester_cloud = Quester::new();
	let (event_storages, index_storages) = create_storages(&node_config.storage_configs.0).await?;

	info!("Serving Querent Node 🚀");
	info!("Node ID: {}", node_config.node_id);
	info!("Starting Querent Base 🏁");
	let semantic_service_bus: MessageBus<SemanticService> =
		start_semantic_service(&node_config, &quester_cloud, &cluster, &event_broker)
			.await
			.expect("Failed to start semantic service");

	let discovery_service = start_discovery_service(
		&node_config,
		&quester_cloud,
		&cluster,
		event_storages.clone(),
		index_storages.clone(),
	)
	.await?;
	let listen_host = node_config.listen_address.parse::<Host>()?;
	let listen_ip = listen_host.resolve().await?;
	let grpc_listen_addr = SocketAddr::new(listen_ip, node_config.grpc_config.listen_port);
	let rest_listen_addr = SocketAddr::new(listen_ip, node_config.rest_config.listen_port);
	let grpc_config = node_config.grpc_config.clone();
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

	info!("Creating storages 🗄️");
	let (event_storages, index_storages) = create_storages(&node_config.storage_configs.0).await?;
	let secert_store_path = std::path::Path::new("/tmp/querent_secret_store");
	let secret_store = create_secret_store(secert_store_path.to_path_buf()).await?;
	let services = Arc::new(QuesterServices {
		node_config,
		cluster: cluster.clone(),
		event_broker,
		semantic_service_bus,
		event_storages,
		index_storages,
		discovery_service: Some(discovery_service),
		insight_service: None,
		secret_store,
	});
	info!("Starting REST server 📡: check /api-doc.json for available APIs");
	info!("Rest server listening on {}", rest_listen_addr);
	let rest_server = rest::start_rest_server(
		rest_listen_addr,
		services.clone(),
		rest_readiness_trigger,
		rest_shutdown_signal,
	);
	// Setup and start gRPC server.
	let (grpc_readiness_trigger_tx, grpc_readiness_signal_rx) = oneshot::channel::<()>();
	let grpc_readiness_trigger = Box::pin(async move {
		if grpc_readiness_trigger_tx.send(()).is_err() {
			debug!("gRPC server readiness signal receiver was dropped 📡");
		}
	});
	let (grpc_shutdown_trigger_tx, grpc_shutdown_signal_rx) = oneshot::channel::<()>();
	let grpc_shutdown_signal = Box::pin(async move {
		if grpc_shutdown_signal_rx.await.is_err() {
			debug!("gRPC server shutdown trigger sender was dropped 📡");
		}
	});

	info!("Starting gRPC server 📡");
	info!("Starting gRPC server on {}", grpc_listen_addr);
	let grpc_server = grpc::start_grpc_server(
		grpc_listen_addr,
		grpc_config.max_message_size,
		services.clone(),
		grpc_readiness_trigger,
		grpc_shutdown_signal,
	);

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
	let grpc_join_handle = tokio::spawn(grpc_server);

	let (grpc_res, rest_res) = tokio::try_join!(grpc_join_handle, rest_join_handle)
		.expect("tokio task running rest and gRPC servers should not panic or be cancelled");

	if let Err(rest_err) = rest_res {
		error!("REST server failed: {:?}", rest_err);
	}

	if let Err(grpc_err) = grpc_res {
		error!("gRPC server failed: {:?}", grpc_err);
	}

	let actor_exit_statuses = shutdown_handle.await?;
	Ok(actor_exit_statuses)
}
