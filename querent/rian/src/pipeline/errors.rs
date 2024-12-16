// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.ai).

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

	#[error("Missing License Key")]
	MissingLicenseKey,
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
			PipelineErrors::MissingLicenseKey =>
				tonic::Status::invalid_argument("Missing License Key"),
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
