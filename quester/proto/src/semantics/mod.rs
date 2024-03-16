use actors::AskError;
use thiserror;

use crate::error::{GrpcServiceError, ServiceError, ServiceErrorCode};
use serde::{Deserialize, Serialize};

include!("../codegen/querent/querent.semantics.rs");

pub type SemanticsResult<T> = std::result::Result<T, SemanticsError>;

#[derive(Debug, thiserror::Error, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticsError {
	#[error("internal error: {0}")]
	Internal(String),
	#[error("request timed out: {0}")]
	Timeout(String),
	#[error("service unavailable: {0}")]
	Unavailable(String),
}

impl ServiceError for SemanticsError {
	fn error_code(&self) -> ServiceErrorCode {
		match self {
			Self::Internal(_) => ServiceErrorCode::Internal,
			Self::Timeout(_) => ServiceErrorCode::Timeout,
			Self::Unavailable(_) => ServiceErrorCode::Unavailable,
		}
	}
}

impl GrpcServiceError for SemanticsError {
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

impl From<AskError<SemanticsError>> for SemanticsError {
	fn from(error: AskError<SemanticsError>) -> Self {
		match error {
			AskError::ErrorReply(error) => error,
			AskError::MessageNotDelivered =>
				Self::new_unavailable("request could not be delivered to pipeline".to_string()),
			AskError::ProcessMessageError =>
				Self::new_internal("an error occurred while processing the request".to_string()),
		}
	}
}
