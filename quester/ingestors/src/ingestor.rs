use async_trait::async_trait;
use common::CollectedBytes;
use proto::semantics::IngestedTokens;
use serde::{Deserialize, Serialize};
use std::{fmt, io, sync::Arc};
use thiserror::Error;
use tokio::sync::mpsc::Receiver;

/// Ingestor error kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IngestorErrorKind {
	/// Polling error.
	Polling,
	/// Not supported error.
	NotSupported,
	/// Io error.
	Io,
	/// Not found error.
	NotFound,
	/// Unauthorized error.
	Unauthorized,
	/// Internal error.
	Internal,
}

/// Generic IngestorError.
#[derive(Debug, Clone, Error)]
#[error("source error(kind={kind:?}, source={source})")]
#[allow(missing_docs)]
pub struct IngestorError {
	pub kind: IngestorErrorKind,
	#[source]
	pub source: Arc<anyhow::Error>,
}

/// Generic Result type for source type operations.
pub type IngestorResult<T> = Result<T, IngestorError>;

impl IngestorError {
	pub fn new(kind: IngestorErrorKind, source: Arc<anyhow::Error>) -> Self {
		IngestorError { kind, source }
	}

	/// Add some context to the wrapper error.
	pub fn add_context<C>(self, ctx: C) -> Self
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		IngestorError {
			kind: self.kind,
			source: Arc::new(anyhow::anyhow!("{ctx}").context(self.source)),
		}
	}

	/// Returns the corresponding `IngestorErrorKind` for this error.
	pub fn kind(&self) -> IngestorErrorKind {
		self.kind
	}
}

impl From<io::Error> for IngestorError {
	fn from(err: io::Error) -> IngestorError {
		match err.kind() {
			io::ErrorKind::NotFound => {
				IngestorError::new(IngestorErrorKind::NotFound, Arc::new(err.into()))
			},
			_ => IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into())),
		}
	}
}

impl From<serde_json::Error> for IngestorError {
	fn from(err: serde_json::Error) -> IngestorError {
		IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into()))
	}
}

// Define the trait for async processor
#[async_trait]
pub trait AsyncProcessor: Send + Sync {
	async fn process_text(&self, data: &str) -> IngestorResult<IngestedTokens>;
	async fn process_media(&self, image: Vec<u8>) -> IngestorResult<IngestedTokens>;
}

// Define the trait for BaseIngestor
#[async_trait]
pub trait BaseIngestor: Send + Sync {
	fn set_processors(&mut self, processors: Vec<Arc<dyn AsyncProcessor>>);

	async fn ingest(
		&self,
		all_collected_bytes: Vec<CollectedBytes>,
	) -> IngestorResult<Receiver<IngestedTokens>>;

	async fn process_text(&self, data: &str) -> IngestorResult<IngestedTokens>;

	async fn process_media(&self, image: Vec<u8>) -> IngestorResult<IngestedTokens>;
}
