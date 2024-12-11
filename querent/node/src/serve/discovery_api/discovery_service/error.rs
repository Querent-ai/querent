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

impl common::ServiceError for DiscoveryError {
	fn error_code(&self) -> common::ServiceErrorCode {
		match self {
			Self::Internal(_) => common::ServiceErrorCode::Internal,
			Self::InvalidArgument(_) => common::ServiceErrorCode::BadRequest,
			Self::StorageError(_) => common::ServiceErrorCode::Internal,
			Self::Timeout(_) => common::ServiceErrorCode::Timeout,
			Self::Unavailable(_) => common::ServiceErrorCode::Unavailable,
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
