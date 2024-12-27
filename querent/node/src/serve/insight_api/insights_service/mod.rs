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

use std::{collections::HashMap, sync::Arc};

use actors::Querent;

use cluster::Cluster;
use common::EventType;
use proto::NodeConfig;
use rian_core::InsightAgentService;

use storage::{MetaStorage, SecretStorage, Storage};
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
	metadata_store: Arc<dyn MetaStorage>,
	secret_store: Arc<dyn SecretStorage>,
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
