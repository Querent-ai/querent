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

use crate::error::{GrpcServiceError, ServiceError, ServiceErrorCode};
pub use crate::semantics::{Neo4jConfig, PostgresConfig, StorageConfig, StorageType};
use actors::AskError;
use bytes::Bytes;
use bytestring::ByteString;
use prost::DecodeError;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

include!("../codegen/querent/querent.layer.rs");

#[derive(Debug, thiserror::Error, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerError {
	#[error("internal error: {0}")]
	Internal(String),
	#[error("request timed out: {0}")]
	Timeout(String),
	#[error("service unavailable: {0}")]
	Unavailable(String),
}

impl ServiceError for LayerError {
	fn error_code(&self) -> ServiceErrorCode {
		match self {
			Self::Internal(_) => ServiceErrorCode::Internal,
			Self::Timeout(_) => ServiceErrorCode::Timeout,
			Self::Unavailable(_) => ServiceErrorCode::Unavailable,
		}
	}
}

impl GrpcServiceError for LayerError {
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

impl From<AskError<LayerError>> for LayerError {
	fn from(error: AskError<LayerError>) -> Self {
		match error {
			AskError::ErrorReply(error) => error,
			AskError::MessageNotDelivered =>
				Self::new_unavailable("request could not be delivered to pipeline".to_string()),
			AskError::ProcessMessageError =>
				Self::new_internal("an error occurred while processing the request".to_string()),
		}
	}
}

#[derive(Clone, Default, Eq, PartialEq, Hash, Ord, PartialOrd, utoipa::ToSchema, specta::Type)]
pub enum LayerAgentType {
	#[default]
	Link,
}

impl Display for LayerAgentType {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Link => write!(f, "link"),
		}
	}
}

impl Debug for LayerAgentType {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Link => write!(f, "link"),
		}
	}
}

impl LayerAgentType {
	pub fn as_bytes(&self) -> Bytes {
		match self {
			Self::Link => Bytes::from("link"),
		}
	}

	pub fn from_i32(value: i32) -> Self {
		match value {
			0 => Self::Link,
			_ => panic!("invalid layer agent type"),
		}
	}

	pub fn as_i32(&self) -> i32 {
		match self {
			Self::Link => 0,
		}
	}

	pub fn as_str(&self) -> &str {
		match self {
			Self::Link => "link",
		}
	}
}

impl From<ByteString> for LayerAgentType {
	fn from(value: ByteString) -> Self {
		match &value[..] {
			"link" | "Link" => Self::Link,
			_ => panic!("invalid layer session type"),
		}
	}
}

impl From<String> for LayerAgentType {
	fn from(session_type: String) -> Self {
		Self::from(ByteString::from(session_type))
	}
}

impl Serialize for LayerAgentType {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.collect_str(self)
	}
}

impl<'de> Deserialize<'de> for LayerAgentType {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let type_str = String::deserialize(deserializer)?;
		Ok(Self::from(type_str))
	}
}

impl PartialEq<LayerAgentType> for &LayerAgentType {
	#[inline]
	fn eq(&self, other: &LayerAgentType) -> bool {
		*self == other
	}
}

impl prost::Message for LayerAgentType {
	fn encode_raw<B>(&self, buf: &mut B)
	where
		B: prost::bytes::BufMut,
	{
		prost::encoding::bytes::encode(1u32, &self.as_bytes(), buf);
	}

	fn merge_field<B>(
		&mut self,
		tag: u32,
		wire_type: prost::encoding::WireType,
		buf: &mut B,
		ctx: prost::encoding::DecodeContext,
	) -> ::core::result::Result<(), prost::DecodeError>
	where
		B: prost::bytes::Buf,
	{
		const STRUCT_NAME: &str = "LayerAgentType";

		match tag {
			1u32 => {
				let mut value = Vec::new();
				prost::encoding::bytes::merge(wire_type, &mut value, buf, ctx).map_err(
					|mut error| {
						error.push(STRUCT_NAME, "session_type");
						error
					},
				)?;
				let byte_string = ByteString::try_from(value)
					.map_err(|_| DecodeError::new("layer_agent_type is not valid UTF-8"))?;
				*self = Self::from(byte_string);
				Ok(())
			},
			_ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
		}
	}

	#[inline]
	fn encoded_len(&self) -> usize {
		prost::encoding::bytes::encoded_len(1u32, &self.as_bytes())
	}

	fn clear(&mut self) {
		*self = Self::default();
	}
}
