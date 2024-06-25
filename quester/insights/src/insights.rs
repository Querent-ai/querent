use anyhow::Error as AnyhowError;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, io, sync::Arc};
use thiserror::Error;

use crate::{ConfigCallbackResponse, InsightConfig, InsightInfo};

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
	pub source: Arc<AnyhowError>,
}

/// Generic Result type for source type operations.
pub type InsightResult<T> = Result<T, InsightError>;

impl InsightError {
	pub fn new(kind: InsightErrorKind, source: Arc<AnyhowError>) -> Self {
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
			io::ErrorKind::NotFound =>
				InsightError::new(InsightErrorKind::NotFound, Arc::new(err.into())),
			_ => InsightError::new(InsightErrorKind::Io, Arc::new(err.into())),
		}
	}
}

impl From<serde_json::Error> for InsightError {
	fn from(err: serde_json::Error) -> InsightError {
		InsightError::new(InsightErrorKind::Io, Arc::new(err.into()))
	}
}

/// The `Insight` trait, representing a generic insight operation.
#[async_trait]
pub trait Insight: Send + Sync {
	/// The input type for the insight.
	type Input: Send + Sync + Serialize + for<'de> Deserialize<'de>;
	/// The output type for the insight.
	type Output: Send + Sync + Serialize + for<'de> Deserialize<'de>;
	/// The streaming input type for the insight.
	type InputStream: Stream<Item = Self::Input> + Send + Sync;
	/// The streaming output type for the insight.
	type OutputStream: Stream<Item = InsightResult<Self::Output>> + Send + Sync;

	/// Constructor so creation can be generalized
	async fn new() -> Box<Self>
	where
		Self: Sized;

	/// Get info about this platform
	async fn info(&self) -> InsightInfo;

	/// Does insight support streaming
	fn supports_streaming(&self) -> bool;

	/// Callback from config
	/// Use Value as data type for the sake of object safety, FFI, simplicity, and it's used in JS anyway
	fn config_callback(&mut self, _name: &str, _config: Value) -> ConfigCallbackResponse {
		ConfigCallbackResponse::Empty
	}

	/// Get InsightRunner for this insight
	fn get_runner(
		&mut self,
		config: &InsightConfig,
	) -> InsightResult<
		Arc<
			dyn InsightRunner<
				Input = Self::Input,
				Output = Self::Output,
				InputStream = Self::InputStream,
				OutputStream = Self::OutputStream,
			>,
		>,
	>;
}

/// The `InsightRunner` trait, representing a generic insight operation.
/// This is the actual implementation of the insight.
/// It is separated from the `Insight` trait to allow for more flexibility in the implementation.
/// For example, the `Insight` trait can be used to create a new instance of the insight, while the `InsightRunner` trait
/// can be used to run the insight.
#[async_trait]
pub trait InsightRunner: Send + Sync {
	/// The input type for the insight.
	type Input: Send + Sync + Serialize + for<'de> Deserialize<'de>;
	/// The output type for the insight.
	type Output: Send + Sync + Serialize + for<'de> Deserialize<'de>;
	/// The streaming input type for the insight.
	type InputStream: Stream<Item = Self::Input> + Send + Sync;
	/// The streaming output type for the insight.
	type OutputStream: Stream<Item = InsightResult<Self::Output>> + Send + Sync;

	/// Run the insight with the given input.
	async fn run(&self, input: Self::Input) -> InsightResult<Self::Output>;

	/// Run the insight with the given input stream.
	async fn run_stream(&self, input: Self::InputStream) -> Self::OutputStream;
}
