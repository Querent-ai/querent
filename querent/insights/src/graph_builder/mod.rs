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

pub mod gb_insight;
pub mod gb_runner;
use actors::ActorExitStatus;
use common::DocumentPayload;
use std::sync::Arc;
use storage::Storage;

pub async fn insert_discovered_knowledge_async(
	storage: Arc<dyn Storage>,
	storage_items: Vec<DocumentPayload>,
) -> Result<(), ActorExitStatus> {
	let upsert_result = storage.insert_discovered_knowledge(&storage_items).await;
	match upsert_result {
		Ok(()) => Ok(()),
		Err(e) => {
			log::error!("Error while inserting discovered knowledge: {:?}", e);
			Err(ActorExitStatus::Failure(Arc::new(anyhow::anyhow!(
				"Error while inserting discovered knowledge"
			))))
		},
	}
}
