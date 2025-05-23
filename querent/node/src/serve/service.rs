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

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use crate::{
	discovery_api::discovery_service::{start_discovery_service, DiscoveryService},
	grpc,
	insight_api::insights_service::InsightService,
	insights_service::start_insight_service,
	layer_api::layer_service::{start_layer_service, LayerService},
	rest,
};
use actors::{ActorExitStatus, MessageBus, Querent};
use cluster::{start_cluster_service, Cluster};
use common::{BoxFutureInfaillible, EventType, Host, PubSubBroker, RuntimesConfig, TerimateSignal};
use once_cell::sync::OnceCell;
use proto::config::NodeConfig;
use rian_core::{start_semantic_service, SemanticService, ShutdownPipeline};
use storage::{
	create_metadata_store, create_secret_store, create_storages, MetaStorage, SecretStorage,
	Storage,
};
use tokio::sync::oneshot;
use tracing::{debug, error, info};

use super::node_readiness;
pub static QUERENT_SERVICES_ONCE: OnceCell<Arc<QuerentServices>> = OnceCell::new();
const _READINESS_REPORTING_INTERVAL: Duration = if cfg!(any(test, feature = "testsuite")) {
	Duration::from_millis(25)
} else {
	Duration::from_secs(10)
};

pub struct QuerentServices {
	pub pipeline_id: Option<String>,
	pub node_config: NodeConfig,
	pub cluster: Cluster,
	pub event_broker: PubSubBroker,
	pub semantic_service_bus: MessageBus<SemanticService>,
	pub discovery_service: Option<Arc<dyn DiscoveryService>>,
	pub insight_service: Option<Arc<dyn InsightService>>,
	pub layer_service: Option<Arc<dyn LayerService>>,
	pub event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	pub index_storages: Vec<Arc<dyn Storage>>,
	pub secret_store: Arc<dyn SecretStorage>,
	pub metadata_store: Arc<dyn MetaStorage>,
}

fn get_querent_data_path() -> PathBuf {
	let data_path = dirs::data_dir().expect("Failed to get Querent data directory");
	data_path.join("querent_data")
}

pub async fn serve_quester(
	node_config: NodeConfig,
	_runtimes_config: RuntimesConfig,
	shutdown_signal: BoxFutureInfaillible<()>,
) -> anyhow::Result<HashMap<String, ActorExitStatus>> {
	let cluster = start_cluster_service(&node_config).await?;
	let event_broker = PubSubBroker::default();
	let quester_cloud: Querent = Querent::new();
	info!("Creating storages 🗄️");
	let querent_data_path = get_querent_data_path();
	let secret_store = create_secret_store(querent_data_path.clone().to_path_buf()).await?;
	let metadata_store = create_metadata_store(querent_data_path.clone().to_path_buf()).await?;

	let (event_storages, index_storages) =
		create_storages(&node_config.storage_configs.0, querent_data_path.clone()).await?;

	info!("Serving Querent RIAN Node 🚀");
	info!("Node ID: {}", node_config.node_id);
	info!("Starting Querent RIAN 🏁");
	log::info!("Node ID: {}", node_config.node_id);
	let semantic_service_bus: MessageBus<SemanticService> = start_semantic_service(
		&node_config,
		&quester_cloud,
		&cluster,
		&event_broker,
		secret_store.clone(),
	)
	.await
	.expect("Failed to start semantic service");

	info!("Starting Discovery Service 🕵️");
	let discovery_service = start_discovery_service(
		&node_config,
		&quester_cloud,
		&cluster,
		event_storages.clone(),
		index_storages.clone(),
		metadata_store.clone(),
		secret_store.clone(),
	)
	.await?;
	log::info!("Starting Layer Service 🧠");
	let layer_service = start_layer_service(
		&node_config,
		&quester_cloud,
		&cluster,
		event_storages.clone(),
		index_storages.clone(),
		metadata_store.clone(),
		secret_store.clone(),
	)
	.await?;
	log::info!("Starting Insight Service 🧠");
	let insight_service = start_insight_service(
		&node_config,
		&quester_cloud,
		&cluster,
		event_storages.clone(),
		index_storages.clone(),
		metadata_store.clone(),
		secret_store.clone(),
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

	let services = Arc::new(QuerentServices {
		pipeline_id: None,
		node_config,
		cluster: cluster.clone(),
		event_broker,
		semantic_service_bus,
		event_storages,
		index_storages,
		discovery_service: Some(discovery_service),
		insight_service: Some(insight_service),
		layer_service: Some(layer_service),
		secret_store,
		metadata_store,
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

/// Serve Querent without starting REST and gRPC servers.
/// This is useful for testing and other scenarios where REST and gRPC servers are not needed.
/// This function returns a handle to the services that can be used to interact with the Querent node.
/// The caller is responsible for shutting down the Querent node by calling `quit` on the returned handle.
/// The caller is also responsible for shutting down the Querent node by calling `quit` on the returned handle.
pub async fn serve_quester_without_servers(
	node_config: NodeConfig,
	terminate_sig: TerimateSignal,
) -> anyhow::Result<()> {
	let cluster = start_cluster_service(&node_config).await?;
	let event_broker = PubSubBroker::default();
	let quester_cloud = Querent::new();
	info!("Creating storages 🗄️");
	let querent_data_path = get_querent_data_path();
	let secret_store = create_secret_store(querent_data_path.clone().to_path_buf()).await?;
	let metadata_store = create_metadata_store(querent_data_path.clone().to_path_buf()).await?;

	let (event_storages, index_storages) =
		create_storages(&node_config.storage_configs.0, querent_data_path.clone()).await?;

	info!("Serving Querent RIAN Node 🚀");
	info!("Node ID: {}", node_config.node_id);
	info!("Starting Querent RIAN 🏁");
	log::info!("Node ID: {}", node_config.node_id);
	let semantic_service_bus: MessageBus<SemanticService> = start_semantic_service(
		&node_config,
		&quester_cloud,
		&cluster,
		&event_broker,
		secret_store.clone(),
	)
	.await
	.expect("Failed to start semantic service");

	info!("Starting Discovery Service 🕵️");
	let discovery_service = start_discovery_service(
		&node_config,
		&quester_cloud,
		&cluster,
		event_storages.clone(),
		index_storages.clone(),
		metadata_store.clone(),
		secret_store.clone(),
	)
	.await?;

	info!("Starting Insight Service 🧠");
	let insight_service = start_insight_service(
		&node_config,
		&quester_cloud,
		&cluster,
		event_storages.clone(),
		index_storages.clone(),
		metadata_store.clone(),
		secret_store.clone(),
	)
	.await?;

	info!("Starting Layer Service 🧠");
	let layer_service = start_layer_service(
		&node_config,
		&quester_cloud,
		&cluster,
		event_storages.clone(),
		index_storages.clone(),
		metadata_store.clone(),
		secret_store.clone(),
	)
	.await?;

	let services = Arc::new(QuerentServices {
		pipeline_id: None,
		node_config,
		cluster,
		event_broker,
		semantic_service_bus,
		discovery_service: Some(discovery_service),
		insight_service: Some(insight_service),
		layer_service: Some(layer_service),
		event_storages,
		index_storages,
		secret_store,
		metadata_store,
	});
	// set the QuerentServices in the global static variable
	let set_res = QUERENT_SERVICES_ONCE.set(services.clone());
	if set_res.is_err() {
		error!("Failed to set QuerentServices in global static variable");
		return Err(anyhow::anyhow!("Failed to set QuerentServices in global static variable"));
	}
	// start the terminate signal listener
	// Keep the task running as long as the termination signal is not received.
	while !terminate_sig.is_dead() {
		// Sleep for a small duration to prevent high CPU usage
		tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
	}
	Ok(())
}

pub async fn shutdown_querent(services: &Arc<QuerentServices>) -> anyhow::Result<()> {
	info!("Shutting down Querent RIAN Node 🛑");
	if services.pipeline_id.is_none() {
		info!("Querent RIAN Node is not running");
		return Ok(());
	}
	let shutdown = ShutdownPipeline { pipeline_id: services.pipeline_id.clone().unwrap() };
	let _ = services.semantic_service_bus.send_message(shutdown).await;
	info!("Shutting down Discovery Service 🛑");
	Ok(())
}
