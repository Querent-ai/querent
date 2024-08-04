use crate::error::{GrpcServiceError, ServiceError, ServiceErrorCode};
pub use crate::semantics::{MilvusConfig, Neo4jConfig, PostgresConfig, StorageConfig, StorageType};
use actors::AskError;
use bytes::Bytes;
use bytestring::ByteString;
use prost::DecodeError;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

include!("../codegen/querent/querent.discovery.rs");

#[derive(Debug, thiserror::Error, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryError {
	#[error("internal error: {0}")]
	Internal(String),
	#[error("request timed out: {0}")]
	Timeout(String),
	#[error("service unavailable: {0}")]
	Unavailable(String),
}

impl ServiceError for DiscoveryError {
	fn error_code(&self) -> ServiceErrorCode {
		match self {
			Self::Internal(_) => ServiceErrorCode::Internal,
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

impl From<AskError<DiscoveryError>> for DiscoveryError {
	fn from(error: AskError<DiscoveryError>) -> Self {
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
pub enum DiscoveryAgentType {
	#[default]
	Retriever,
	Traverser,
}

impl Display for DiscoveryAgentType {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Retriever => write!(f, "retriever"),
			Self::Traverser => write!(f, "traverser"),
		}
	}
}

impl Debug for DiscoveryAgentType {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Retriever => write!(f, "retriever"),
			Self::Traverser => write!(f, "traverser"),
		}
	}
}

impl DiscoveryAgentType {
	pub fn as_bytes(&self) -> Bytes {
		match self {
			Self::Retriever => Bytes::from("retriever"),
			Self::Traverser => Bytes::from("traverser"),
		}
	}

	pub fn from_i32(value: i32) -> Self {
		match value {
			0 => Self::Retriever,
			1 => Self::Traverser,
			_ => panic!("invalid discovery agent type"),
		}
	}

	pub fn as_i32(&self) -> i32 {
		match self {
			Self::Retriever => 0,
			Self::Traverser => 1,
		}
	}

	pub fn as_str(&self) -> &str {
		match self {
			Self::Retriever => "retriever",
			Self::Traverser => "traverser",
		}
	}
}

impl From<ByteString> for DiscoveryAgentType {
	fn from(value: ByteString) -> Self {
		match &value[..] {
			"retriever" | "Retriever" => Self::Retriever,
			"traverser" | "Traverser" => Self::Traverser,
			_ => panic!("invalid discovery session type"),
		}
	}
}

impl From<String> for DiscoveryAgentType {
	fn from(session_type: String) -> Self {
		Self::from(ByteString::from(session_type))
	}
}

impl Serialize for DiscoveryAgentType {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.collect_str(self)
	}
}

impl<'de> Deserialize<'de> for DiscoveryAgentType {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let type_str = String::deserialize(deserializer)?;
		Ok(Self::from(type_str))
	}
}

impl PartialEq<DiscoveryAgentType> for &DiscoveryAgentType {
	#[inline]
	fn eq(&self, other: &DiscoveryAgentType) -> bool {
		*self == other
	}
}

impl prost::Message for DiscoveryAgentType {
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
		const STRUCT_NAME: &str = "DiscoveryAgentType";

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
					.map_err(|_| DecodeError::new("discovery_agent_type is not valid UTF-8"))?;
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
