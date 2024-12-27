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

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnthropicError {
	#[error("Anthropic API error: Invalid request - {0}")]
	InvalidRequestError(String),

	#[error("Anthropic API error: Authentication failed - {0}")]
	AuthenticationError(String),

	#[error("Anthropic API error: Permission denied - {0}")]
	PermissionError(String),

	#[error("Anthropic API error: Not found - {0}")]
	NotFoundError(String),

	#[error("Anthropic API error: Rate limit exceeded - {0}")]
	RateLimitError(String),

	#[error("Anthropic API error: Internal error - {0}")]
	ApiError(String),

	#[error("Anthropic API error: Overloaded - {0}")]
	OverloadedError(String),
}
