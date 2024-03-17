use crate::error::{GrpcServiceError, ServiceError, ServiceErrorCode};
use serde::{Deserialize, Serialize};

include!("../codegen/querent/querent.cluster.rs");

pub type ClusterResult<T> = std::result::Result<T, ClusterError>;

#[derive(Debug, thiserror::Error, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterError {
	#[error("internal error: {0}")]
	Internal(String),
	#[error("request timed out: {0}")]
	Timeout(String),
	#[error("service unavailable: {0}")]
	Unavailable(String),
}

impl ServiceError for ClusterError {
	fn error_code(&self) -> ServiceErrorCode {
		match self {
			Self::Internal(_) => ServiceErrorCode::Internal,
			Self::Timeout(_) => ServiceErrorCode::Timeout,
			Self::Unavailable(_) => ServiceErrorCode::Unavailable,
		}
	}
}

impl GrpcServiceError for ClusterError {
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
