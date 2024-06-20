use async_stream::stream;
use async_trait::async_trait;
use futures::{pin_mut, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{fmt, io, sync::Arc};
use thiserror::Error;

/// Insight error kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum InsightErrorKind {
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

/// Generic InsightError.
#[derive(Debug, Clone, Error)]
#[error("source error(kind={kind:?}, source={source})")]
#[allow(missing_docs)]
pub struct InsightError {
	pub kind: InsightErrorKind,
	#[source]
	pub source: Arc<anyhow::Error>,
}

/// Generic Result type for source type operations.
pub type InsightResult<T> = Result<T, InsightError>;

impl InsightError {
	pub fn new(kind: InsightErrorKind, source: Arc<anyhow::Error>) -> Self {
		InsightError { kind, source }
	}

	/// Add some context to the wrapper error.
	pub fn add_context<C>(self, ctx: C) -> Self
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		InsightError {
			kind: self.kind,
			source: Arc::new(anyhow::anyhow!("{ctx}").context(self.source)),
		}
	}

	/// Returns the corresponding `InsightErrorKind` for this error.
	pub fn kind(&self) -> InsightErrorKind {
		self.kind
	}
}

impl From<io::Error> for InsightError {
	fn from(err: io::Error) -> InsightError {
		match err.kind() {
			io::ErrorKind::NotFound => {
				InsightError::new(InsightErrorKind::NotFound, Arc::new(err.into()))
			},
			_ => InsightError::new(InsightErrorKind::Io, Arc::new(err.into())),
		}
	}
}

impl From<serde_json::Error> for InsightError {
	fn from(err: serde_json::Error) -> InsightError {
		InsightError::new(InsightErrorKind::Io, Arc::new(err.into()))
	}
}
