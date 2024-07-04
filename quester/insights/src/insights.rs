use anyhow::Error as AnyhowError;
use async_trait::async_trait;
use futures::Stream;
use proto::{grpc_error_to_grpc_status, GrpcServiceError, ServiceError, ServiceErrorCode};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, io, pin::Pin, sync::Arc};
use thiserror::Error;
use tokio::task::JoinError;

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
	/// Inference error.
	Inference,
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

impl Serialize for InsightError {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut state = serializer.serialize_struct("InsightError", 2)?;
		state.serialize_field("kind", &self.kind)?;
		state.serialize_field("source", &self.source.to_string())?;
		state.end()
	}
}

impl<'de> Deserialize<'de> for InsightError {
	fn deserialize<D>(deserializer: D) -> Result<InsightError, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let value = serde_json::Value::deserialize(deserializer)?;
		let kind = value["kind"].as_str().unwrap_or_default();
		let source = value["source"].as_str().unwrap_or_default();
		Ok(InsightError {
			kind: match kind {
				"NotSupported" => InsightErrorKind::NotSupported,
				"Io" => InsightErrorKind::Io,
				"NotFound" => InsightErrorKind::NotFound,
				"Unauthorized" => InsightErrorKind::Unauthorized,
				"Internal" => InsightErrorKind::Internal,
				_ => InsightErrorKind::Internal,
			},
			source: Arc::new(anyhow::anyhow!("{source}")),
		})
	}
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
impl ServiceError for InsightError {
	fn error_code(&self) -> ServiceErrorCode {
		match self.kind {
			InsightErrorKind::NotSupported => ServiceErrorCode::BadRequest,
			InsightErrorKind::Io => ServiceErrorCode::Internal,
			InsightErrorKind::NotFound => ServiceErrorCode::NotFound,
			InsightErrorKind::Unauthorized => ServiceErrorCode::Unauthenticated,
			InsightErrorKind::Internal => ServiceErrorCode::Internal,
			InsightErrorKind::Inference => ServiceErrorCode::Internal,
		}
	}
}

impl common::ServiceError for InsightError {
	fn error_code(&self) -> common::ServiceErrorCode {
		match self.kind {
			InsightErrorKind::NotSupported => common::ServiceErrorCode::BadRequest,
			InsightErrorKind::Io => common::ServiceErrorCode::Internal,
			InsightErrorKind::NotFound => common::ServiceErrorCode::NotFound,
			InsightErrorKind::Unauthorized => common::ServiceErrorCode::MethodNotAllowed,
			InsightErrorKind::Internal => common::ServiceErrorCode::Internal,
			InsightErrorKind::Inference => common::ServiceErrorCode::Internal,
		}
	}
}

impl GrpcServiceError for InsightError {
	fn new_internal(message: String) -> Self {
		InsightError::new(
			InsightErrorKind::Internal,
			Arc::new(anyhow::anyhow!("internal error: {message}")),
		)
	}

	fn new_timeout(message: String) -> Self {
		InsightError::new(
			InsightErrorKind::Internal,
			Arc::new(anyhow::anyhow!("request timed out: {message}")),
		)
	}

	fn new_unavailable(message: String) -> Self {
		InsightError::new(
			InsightErrorKind::NotSupported,
			Arc::new(anyhow::anyhow!("service unavailable: {message}")),
		)
	}
}

impl From<InsightError> for tonic::Status {
	fn from(error: InsightError) -> Self {
		grpc_error_to_grpc_status(error)
	}
}

impl From<anyhow::Error> for InsightError {
	fn from(any_error: anyhow::Error) -> Self {
		InsightError::new(InsightErrorKind::Internal, Arc::new(any_error))
	}
}

impl From<JoinError> for InsightError {
	fn from(join_error: JoinError) -> InsightError {
		InsightError::new(InsightErrorKind::Internal, Arc::new(anyhow::anyhow!("{join_error}")))
	}
}

impl From<std::convert::Infallible> for InsightError {
	fn from(infallible: std::convert::Infallible) -> InsightError {
		match infallible {}
	}
}

/// Generic insight input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightInput {
	pub data: Value,
}

/// Generic insight output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightOutput {
	pub data: Value,
}

/// The `Insight` trait, representing a generic insight operation.
#[async_trait]
pub trait Insight: Send + Sync {
	/// Constructor so creation can be generalized
	async fn new() -> Arc<Self>
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
	fn get_runner(&self, config: &InsightConfig) -> InsightResult<Arc<dyn InsightRunner>>;
}

/// The `InsightRunner` trait, representing a generic insight operation.
/// This is the actual implementation of the insight.
/// It is separated from the `Insight` trait to allow for more flexibility in the implementation.
/// For example, the `Insight` trait can be used to create a new instance of the insight, while the `InsightRunner` trait
/// can be used to run the insight.
#[async_trait]
pub trait InsightRunner: Send + Sync {
	/// Run the insight with the given input.
	async fn run(&self, input: InsightInput) -> InsightResult<InsightOutput>;

	/// Run the insight with the given input stream.
	async fn run_stream(
		&self,
		input: Pin<Box<dyn Stream<Item = InsightInput> + Send + 'static>>,
	) -> InsightResult<Pin<Box<dyn Stream<Item = InsightResult<InsightOutput>> + Send + 'static>>>;
}
