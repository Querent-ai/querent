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
