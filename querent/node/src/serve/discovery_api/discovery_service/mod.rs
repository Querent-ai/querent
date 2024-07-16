use actors::Querent;
use client::DiscoveryServiceClient;
use cluster::Cluster;
use common::{EventType, Pool};
use error::DiscoveryError;
use proto::NodeConfig;
use rian::discovery_service::DiscoveryAgentService;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use storage::Storage;
use tracing::info;

mod client;
pub mod error;
pub mod service;
pub use service::DiscoveryService;

pub type InsightsPool = Pool<SocketAddr, DiscoveryServiceClient>;

pub type Result<T> = std::result::Result<T, DiscoveryError>;

pub async fn start_discovery_service(
	node_config: &NodeConfig,
	querent: &Querent,
	cluster: &Cluster,
	event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	index_storages: Vec<Arc<dyn Storage>>,
	metadata_store: Arc<dyn Storage>,
) -> anyhow::Result<Arc<dyn DiscoveryService>> {
	let discovery_agent_service = DiscoveryAgentService::new(
		node_config.node_id.clone(),
		cluster.clone(),
		event_storages.clone(),
	);
	let (discovery_service_mailbox, _) = querent.spawn_builder().spawn(discovery_agent_service);
	info!("Starting discovery agent service");

	let discovery_service = Arc::new(service::DiscoveryImpl::new(
		event_storages,
		index_storages,
		metadata_store,
		discovery_service_mailbox,
	));
	Ok(discovery_service)
}
