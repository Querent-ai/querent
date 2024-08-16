use std::{
	fmt::{self, Debug},
	sync::Arc,
};

use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use proto::{semantics::SemanticPipelineRequest, DiscoverySessionRequest, InsightAnalystRequest};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{postgres_index::QuerySuggestion, DiscoveredKnowledge};

pub const RIAN_API_KEY: &str = "RIAN_API_KEY";

/// Storage error kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StorageErrorKind {
	/// Error in collection creation.
	CollectionCreation,
	/// Error in collection building.
	CollectionBuilding,
	/// Error in collection retrieval.
	CollectionRetrieval,
	/// Insertion error.
	Insertion,
	/// Query error.
	Query,
	/// PartitionCreation error for vector storage.
	PartitionCreation,
	/// Database error.
	Database,
	/// The target index does not exist.
	NotFound,
	/// The request credentials do not allow for this operation.
	Unauthorized,
	/// A third-party service forbids this operation, or is misconfigured.
	Service,
	/// Any generic internal error.
	Internal,
	/// A timeout occurred during the operation.
	Timeout,
	/// Io error.
	Io,
	/// A index creation error for pgvector.
	IndexCreation,
	/// Serialization error.
	Serialization,
}

/// Generic StorageError.
#[derive(Debug, Clone, Error)]
#[error("storage error(kind={kind:?}, source={source})")]
#[allow(missing_docs)]
pub struct StorageError {
	pub kind: StorageErrorKind,
	#[source]
	pub source: Arc<anyhow::Error>,
}

/// Generic Result type for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

impl StorageError {
	/// Add some context to the wrapper error.
	pub fn add_context<C>(self, ctx: C) -> Self
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		StorageError {
			kind: self.kind,
			source: Arc::new(anyhow::anyhow!("{ctx}").context(self.source)),
		}
	}

	/// Returns the corresponding `StorageErrorKind` for this error.
	pub fn kind(&self) -> StorageErrorKind {
		self.kind
	}
}

/// Storage is a trait for all storage types.
/// Currently we support Graph, Vector and Index storages.
#[async_trait]
pub trait Storage: Send + Sync + 'static {
	/// Check storage connection if applicable
	async fn check_connectivity(&self) -> anyhow::Result<()>;

	/// Insert VectorPayload into storage
	async fn insert_vector(
		&self,
		collection_id: String,
		payload: &Vec<(String, String, Option<String>, VectorPayload)>,
	) -> StorageResult<()>;

	/// Insert DiscoveryPayload into storage
	async fn insert_discovered_knowledge(
		&self,
		payload: &Vec<DocumentPayload>,
	) -> StorageResult<()>;

	/// Insert SemanticKnowledgePayload into storage
	async fn insert_graph(
		&self,
		collection_id: String,
		payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()>;

	/// Index knowledge for search
	async fn index_knowledge(
		&self,
		collection_id: String,
		payload: &Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> StorageResult<()>;

	async fn similarity_search_l2(
		&self,
		session_id: String,
		query: String,
		collection_id: String,
		payload: &Vec<f32>,
		max_results: i32,
		offset: i64,
		top_pairs_embeddings: Vec<Vec<f32>>,
	) -> StorageResult<Vec<DocumentPayload>>;

	async fn traverse_metadata_table(
		&self,
		filtered_pairs: &[(String, String)],
	) -> StorageResult<Vec<(String, String, String, String, String, String, String, f32)>>;

	/// Get discovered data based on session_id
	async fn get_discovered_data(
		&self,
		session_id: String,
	) -> StorageResult<Vec<DiscoveredKnowledge>>;

	/// Store key value pair
	async fn store_secret(&self, key: &String, value: &String) -> StorageResult<()>;

	/// Get value for key
	async fn get_secret(&self, key: &String) -> StorageResult<Option<String>>;

	/// Delete the key value pair
	async fn delete_secret(&self, key: &String) -> StorageResult<()>;

	/// Get all key value pair
	async fn get_all_secrets(&self) -> StorageResult<Vec<(String, String)>>;

	/// Get all SemanticPipeline ran by this node
	async fn get_all_pipelines(&self) -> StorageResult<Vec<(String, SemanticPipelineRequest)>>;

	/// Set SemanticPipeline ran by this node
	async fn set_pipeline(
		&self,
		pipeline_id: &String,
		pipeline: SemanticPipelineRequest,
	) -> StorageResult<()>;

	/// Get semantic pipeline by id
	async fn get_pipeline(
		&self,
		pipeline_id: &String,
	) -> StorageResult<Option<SemanticPipelineRequest>>;

	/// Delete semantic pipeline by id
	async fn delete_pipeline(&self, pipeline_id: &String) -> StorageResult<()>;

	/// Get all Discovery sessions ran by this node
	async fn get_all_discovery_sessions(
		&self,
	) -> StorageResult<Vec<(String, DiscoverySessionRequest)>>;

	/// Set Discovery session ran by this node
	async fn set_discovery_session(
		&self,
		session_id: &String,
		session: DiscoverySessionRequest,
	) -> StorageResult<()>;

	/// Get Discovery session by id
	async fn get_discovery_session(
		&self,
		session_id: &String,
	) -> StorageResult<Option<DiscoverySessionRequest>>;

	/// Get all Insight sessions ran by this node
	async fn get_all_insight_sessions(&self)
		-> StorageResult<Vec<(String, InsightAnalystRequest)>>;

	/// Set Insight session ran by this node
	async fn set_insight_session(
		&self,
		session_id: &String,
		session: InsightAnalystRequest,
	) -> StorageResult<()>;

	/// Get Insight session by id
	async fn get_insight_session(
		&self,
		session_id: &String,
	) -> StorageResult<Option<InsightAnalystRequest>>;

	/// Insert InsightKnowledge into storage
	async fn insert_insight_knowledge(
		&self,
		query: Option<String>,
		session_id: Option<String>,
		response: Option<String>,
	) -> StorageResult<()>;

	/// Set API key for RIAN
	async fn set_rian_api_key(&self, api_key: &String) -> StorageResult<()>;

	/// Get API key for RIAN
	async fn get_rian_api_key(&self) -> StorageResult<Option<String>>;

	/// Asynchronously fetches popular queries .
	async fn autogenerate_queries(
		&self,
		max_suggestions: i32,
	) -> StorageResult<Vec<QuerySuggestion>>;

	/// Retrieve Filetered Results when query is empty and semantic pair filters are provided
	async fn filter_and_query(
		&self,
		session_id: &String,
		top_pairs: &Vec<String>,
		max_results: i32,
		offset: i64,
	) -> StorageResult<Vec<DocumentPayload>>;
}

impl Debug for dyn Storage {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Storage").finish()
	}
}
