use std::{collections::HashMap, sync::Arc};

use actors::Querent;

use cluster::Cluster;
use common::EventType;
use proto::NodeConfig;
use rian_core::InsightAgentService;

use storage::Storage;
use tracing::info;
pub mod client;
pub use client::*;
pub mod service;
pub use service::*;

pub async fn start_insight_service(
	node_config: &NodeConfig,
	querent: &Querent,
	cluster: &Cluster,
	event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	index_storages: Vec<Arc<dyn Storage>>,
	metadata_store: Arc<dyn Storage>,
	secret_store: Arc<dyn Storage>,
) -> anyhow::Result<Arc<dyn InsightService>> {
	let license_key = secret_store
		.get_rian_api_key()
		.await
		.map_err(|e| anyhow::anyhow!("Failed to get license key: {:?}", e))?;
	let insight_agent_service = InsightAgentService::new(
		node_config.node_id.clone(),
		cluster.clone(),
		event_storages.clone(),
		index_storages.clone(),
		license_key.clone(),
	);
	let (insight_service_mailbox, _) = querent.spawn_builder().spawn(insight_agent_service);
	info!("Starting insight agent service");

	let insight_service = Arc::new(service::InsightImpl::new(
		event_storages,
		index_storages,
		metadata_store,
		insight_service_mailbox,
	));
	Ok(insight_service)
}
