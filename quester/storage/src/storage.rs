use std::{fmt, sync::Arc};

use async_trait::async_trait;
use common::{SemanticKnowledgePayload, VectorPayload};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Storage error kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StorageErrorKind {
	/// Error in collection creation.
	CollectionCreation,
	/// Error in collection building.
	CollectionBuilding,
	/// Insertion error.
	Insertion,
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
	async fn insert_vector(&self, payload: Vec<(String, VectorPayload)>) -> StorageResult<()>;

	/// Insert SemanticKnowledgePayload into storage
	async fn insert_graph(
		&self,
		payload: Vec<(String, SemanticKnowledgePayload)>,
	) -> StorageResult<()>;

	/// Index triples for search
	async fn index_triples(
		&self,
		payload: Vec<(String, SemanticKnowledgePayload)>,
	) -> StorageResult<()>;
}
