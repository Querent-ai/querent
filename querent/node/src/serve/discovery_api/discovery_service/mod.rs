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

use actors::Querent;
use client::DiscoveryServiceClient;
use cluster::Cluster;
use common::{EventType, Pool};
use error::DiscoveryError;
use proto::NodeConfig;
use rian_core::discovery_service::DiscoveryAgentService;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use storage::{MetaStorage, SecretStorage, Storage};
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
	metadata_store: Arc<dyn MetaStorage>,
	secret_store: Arc<dyn SecretStorage>,
) -> anyhow::Result<Arc<dyn DiscoveryService>> {
	let licence_key = secret_store
		.get_rian_api_key()
		.await
		.map_err(|e| anyhow::anyhow!("Failed to get licence key: {:?}", e))?;
	let discovery_agent_service = DiscoveryAgentService::new(
		node_config.node_id.clone(),
		cluster.clone(),
		event_storages.clone(),
		licence_key.clone(),
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
