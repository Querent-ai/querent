use std::sync::Arc;

use actors::ActorExitStatus;
use common::DocumentPayload;
use storage::Storage;

pub mod discovery_searcher;
pub mod discovery_service;
pub mod discovery_traverser;

async fn insert_discovered_knowledge_async(
	storage: Arc<dyn Storage>,
	storage_items: Vec<DocumentPayload>,
) -> Result<(), ActorExitStatus> {
	let upsert_result = storage.insert_discovered_knowledge(&storage_items).await;
	match upsert_result {
		Ok(()) => {
			// Increment counters if insertion is successful
			// Note: Access to self.counters would require synchronization if used here
			Ok(())
		},
		Err(e) => {
			// Handle error if insertion fails
			log::error!("Error while inserting discovered knowledge: {:?}", e);
			Err(ActorExitStatus::Failure(Arc::new(anyhow::anyhow!(
				"Error while inserting discovered knowledge"
			))))
		},
	}
}
