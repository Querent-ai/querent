use async_trait::async_trait;
use futures_util::Stream;
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
	/// Io error.
	Io,
}

/// Generic SourceError.
#[derive(Debug, Clone, Error)]
#[error("source error(kind={kind:?}, source={source})")]
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

	/// Returns the corresponding `SourceErrorKind` for this error.
	pub fn kind(&self) -> SourceErrorKind {
		self.kind
	}
}

/// Sources is all possible data sources that can be used to create a `CollectedBytes`.
#[async_trait]
pub trait Source: Send + Sync + 'static {
	/// Establishes a connection to the source.
	async fn connect(&self) -> SourceResult<()>;

	/// Polls the source to collect data. Returns an iterator over collected bytes.
	async fn poll(&self) -> SourceResult<Box<dyn Stream<Item = CollectedBytes> + Unpin + Send>>;

	/// Disconnects from the source.
	async fn disconnect(&self) -> SourceResult<()>;
}
