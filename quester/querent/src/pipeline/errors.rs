use actors::AskError;
use anyhow::anyhow;
use common::{ServiceError, ServiceErrorCode};

#[derive(Debug, thiserror::Error)]
pub enum PipelineErrors {
	#[error("semantic pipeline `{pipeline_id}` does not exist")]
	PipelineNotFound { pipeline_id: String },

	#[error("semantic pipeline `{pipeline_id}` already exists")]
	PipelineAlreadyExists { pipeline_id: String },

	#[error("invalid params: {0}")]
	InvalidParams(anyhow::Error),

	#[error("unknown error: {0}")]
	UnknownError(String),
}

impl From<AskError<PipelineErrors>> for PipelineErrors {
	fn from(error: AskError<PipelineErrors>) -> Self {
		match error {
			AskError::ErrorReply(error) => error,
			AskError::MessageNotDelivered =>
				PipelineErrors::UnknownError("the message could not be delivered".to_string()),
			AskError::ProcessMessageError => PipelineErrors::UnknownError(
				"an error occurred while processing the request".to_string(),
			),
		}
	}
}

impl From<PipelineErrors> for tonic::Status {
	fn from(error: PipelineErrors) -> Self {
		match error {
			PipelineErrors::PipelineNotFound { pipeline_id } =>
				tonic::Status::not_found(format!("missing pipeline `{pipeline_id}`")),
			PipelineErrors::PipelineAlreadyExists { pipeline_id } =>
				tonic::Status::already_exists(format!("pipeline `{pipeline_id}` already exists")),
			PipelineErrors::InvalidParams(error) =>
				tonic::Status::invalid_argument(error.to_string()),
			PipelineErrors::UnknownError(error) => tonic::Status::internal(error),
		}
	}
}

impl From<tonic::Status> for PipelineErrors {
	fn from(status: tonic::Status) -> Self {
		match status.code() {
			tonic::Code::NotFound =>
				PipelineErrors::PipelineNotFound { pipeline_id: "".to_string() },
			tonic::Code::AlreadyExists =>
				PipelineErrors::PipelineAlreadyExists { pipeline_id: "".to_string() },
			_ => PipelineErrors::InvalidParams(anyhow!(status.message().to_string())),
		}
	}
}

impl ServiceError for PipelineErrors {
	fn error_code(&self) -> ServiceErrorCode {
		match self {
			PipelineErrors::PipelineNotFound { .. } => ServiceErrorCode::NotFound,
			PipelineErrors::PipelineAlreadyExists { .. } => ServiceErrorCode::BadRequest,
			_ => ServiceErrorCode::Internal,
		}
	}
}
