use async_trait::async_trait;
use futures::Stream;
use querent_synapse::comm::IngestedTokens;
use serde::{Deserialize, Serialize};
use std::{fmt, io, pin::Pin, sync::Arc};
use thiserror::Error;

/// Ingestor error kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EngineErrorKind {
	/// Io error.
	Io,
	/// Not found error.
	NotFound,
}

/// Generic IngestorError.
#[derive(Debug, Clone, Error)]
#[error("source error(kind={kind:?}, source={source})")]
#[allow(missing_docs)]
pub struct EngineError {
	pub kind: EngineErrorKind,
	#[source]
	pub source: Arc<anyhow::Error>,
}

/// Generic Result type for source type operations.
pub type EngineResult<T> = Result<T, EngineError>;

impl EngineError {
	pub fn new(kind: EngineErrorKind, source: Arc<anyhow::Error>) -> Self {
		EngineError { kind, source }
	}

	/// Add some context to the wrapper error.
	pub fn add_context<C>(self, ctx: C) -> Self
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		EngineError {
			kind: self.kind,
			source: Arc::new(anyhow::anyhow!("{ctx}").context(self.source)),
		}
	}

	/// Returns the corresponding `IngestorErrorKind` for this error.
	pub fn kind(&self) -> EngineErrorKind {
		self.kind
	}
}

impl From<io::Error> for EngineError {
	fn from(err: io::Error) -> EngineError {
		match err.kind() {
			io::ErrorKind::NotFound =>
				EngineError::new(EngineErrorKind::NotFound, Arc::new(err.into())),
			_ => EngineError::new(EngineErrorKind::Io, Arc::new(err.into())),
		}
	}
}

impl From<serde_json::Error> for EngineError {
	fn from(err: serde_json::Error) -> EngineError {
		EngineError::new(EngineErrorKind::Io, Arc::new(err.into()))
	}
}

#[async_trait]
pub trait Engine: Send + Sync {
	async fn process_ingested_tokens(
		&mut self,
		tokens: Pin<Box<dyn Stream<Item = IngestedTokens> + Send>>,
	) -> EngineResult<Pin<Box<dyn Stream<Item = EngineResult<IngestedTokens>> + Send + 'static>>>;
}
