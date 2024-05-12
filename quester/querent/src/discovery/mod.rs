use std::sync::Arc;

use actors::ActorExitStatus;
use common::DocumentPayload;
use storage::Storage;

pub mod agent;
pub mod chain;
pub mod discovery_agent;
pub mod discovery_searcher;
pub mod discovery_service;
pub mod discovery_traverser;
pub mod document_loaders;
pub mod embedding;
pub mod language_models;
pub mod llm;
pub mod memory;
pub mod prompt;
pub mod schemas;
pub mod semantic_router;
pub mod text_splitter;
pub mod tools;
pub mod vectorstore;

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
