use proto::{grpc_error_to_grpc_status, GrpcServiceError, ServiceError, ServiceErrorCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::task::JoinError;

/// Possible DiscoveryError
#[allow(missing_docs)]
#[derive(Error, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryError {
	#[error("internal error: `{0}`")]
	Internal(String),
	#[error("Invalid argument: {0}")]
	InvalidArgument(String),
	#[error("storage not found: `{0}`)")]
	StorageError(String),
	#[error("request timed out: {0}")]
	Timeout(String),
	#[error("service unavailable: {0}")]
	Unavailable(String),
}

impl ServiceError for DiscoveryError {
	fn error_code(&self) -> ServiceErrorCode {
		match self {
			Self::Internal(_) => ServiceErrorCode::Internal,
			Self::InvalidArgument(_) => ServiceErrorCode::BadRequest,
			Self::StorageError(_) => ServiceErrorCode::Internal,
			Self::Timeout(_) => ServiceErrorCode::Timeout,
			Self::Unavailable(_) => ServiceErrorCode::Unavailable,
		}
	}
}

impl GrpcServiceError for DiscoveryError {
	fn new_internal(message: String) -> Self {
		Self::Internal(message)
	}

	fn new_timeout(message: String) -> Self {
		Self::Timeout(message)
	}

	fn new_unavailable(message: String) -> Self {
		Self::Unavailable(message)
	}
}

impl From<DiscoveryError> for tonic::Status {
	fn from(error: DiscoveryError) -> Self {
		grpc_error_to_grpc_status(error)
	}
}

impl From<anyhow::Error> for DiscoveryError {
	fn from(any_error: anyhow::Error) -> Self {
		DiscoveryError::Internal(any_error.to_string())
	}
}

impl From<JoinError> for DiscoveryError {
	fn from(join_error: JoinError) -> DiscoveryError {
		DiscoveryError::Internal(format!("spawned task in root join failed: {join_error}"))
	}
}

impl From<std::convert::Infallible> for DiscoveryError {
	fn from(infallible: std::convert::Infallible) -> DiscoveryError {
		match infallible {}
	}
}
