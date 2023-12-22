use async_trait::async_trait;
use common::{SemanticKnowledgePayload, VectorPayload};
/// Storage is a trait for all storage types.
/// Currently we support Graph, Vector and Index storages.
#[async_trait]
pub trait Storage: Send + Sync + 'static {
	/// Check storage connection if applicable
	async fn check_connectivity(&self) -> anyhow::Result<()>;

	/// Insert VectorPayload into storage
	async fn insert_vector(&self, payload: Vec<(String, VectorPayload)>) -> anyhow::Result<()>;

	/// Insert SemanticKnowledgePayload into storage
	async fn insert_graph(
		&self,
		payload: Vec<(String, SemanticKnowledgePayload)>,
	) -> anyhow::Result<()>;
}
