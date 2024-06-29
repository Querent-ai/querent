use std::{
	fmt::{self, Debug},
	sync::Arc,
};

use async_trait::async_trait;
use common::{DocumentPayload, SemanticKnowledgePayload, VectorPayload};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
	) -> StorageResult<Vec<DocumentPayload>>;

	async fn traverse_metadata_table(
		&self,
		filtered_pairs: Vec<(String, String)>,
	) -> StorageResult<Vec<(i32, String, String, String, String, String, String, f32)>>;

	/// Store key value pair
	async fn store_kv(&self, key: &String, value: &String) -> StorageResult<()>;

	/// Get value for key
	async fn get_kv(&self, key: &String) -> StorageResult<Option<String>>;

	/// Delete the key value pair
	async fn delete_kv(&self, key: &String) -> StorageResult<()>;
}

impl Debug for dyn Storage {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Storage").finish()
	}
}
