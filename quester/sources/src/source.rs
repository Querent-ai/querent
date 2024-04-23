use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
	fmt::{self, Debug},
	sync::Arc,
};
use thiserror::Error;

use crate::CollectedBytes;

/// Storage error kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SourceErrorKind {
	/// Connection error.
	Connection,
	/// Polling error.
	Polling,
	/// Not supported error.
	NotSupported,
}

/// Generic StorageError.
#[derive(Debug, Clone, Error)]
#[error("storage error(kind={kind:?}, source={source})")]
#[allow(missing_docs)]
pub struct SourceError {
	pub kind: SourceErrorKind,
	#[source]
	pub source: Arc<anyhow::Error>,
}

/// Generic Result type for source type operations.
pub type SourceResult<T> = Result<T, SourceError>;

impl SourceError {
	pub fn new(kind: SourceErrorKind, source: Arc<anyhow::Error>) -> Self {
		SourceError { kind, source }
	}

	/// Add some context to the wrapper error.
	pub fn add_context<C>(self, ctx: C) -> Self
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		SourceError {
			kind: self.kind,
			source: Arc::new(anyhow::anyhow!("{ctx}").context(self.source)),
		}
	}

	/// Returns the corresponding `StorageErrorKind` for this error.
	pub fn kind(&self) -> SourceErrorKind {
		self.kind
	}
}

/// Sources is all possible data sources that can be used to create a `Document`.
/// These can be blob, local file, remote file, etc.
/// Such as buckets, jira, git, etc.
#[async_trait]
pub trait Source: Send + Sync + 'static {
	async fn connect(&self) -> Result<(), SourceError>;

	async fn poll(&self) -> Result<CollectedBytes, SourceError>;

	async fn disconnect(&self) -> Result<(), SourceError>;
}
