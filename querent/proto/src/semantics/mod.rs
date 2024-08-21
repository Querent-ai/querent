use crate::error::{GrpcServiceError, ServiceError, ServiceErrorCode};
use actors::AskError;
use bytes::Bytes;
use bytestring::ByteString;
use common::{
	CollectionCounter, EventStreamerCounters, EventType, EventsCounter, IndexerCounters,
	IngestorCounters, StorageMapperCounters,
};
use prost::DecodeError;
use serde::{Deserialize, Serialize};
use std::{
	fmt::{Debug, Display, Formatter},
	sync::atomic::Ordering,
};
include!("../codegen/querent/production.semantics.rs");
pub use collector_config::*;

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

impl IndexingStatistics {
	/// Increment the number of documents processed
	pub fn increment_total_docs(&mut self) {
		self.total_docs += 1;
	}

	/// Increment the number of events processed
	pub fn increment_total_events(&mut self) {
		self.total_events += 1;
	}

	/// Increment the number of graph events processed
	pub fn increment_total_graph_events(&mut self) {
		self.total_graph_events += 1;
	}

	/// Increment the number of vector events processed
	pub fn increment_total_vector_events(&mut self) {
		self.total_vector_events += 1;
	}

	pub fn add_counters(
		mut self,
		qflow_counters: &serde_json::Value,
		event_streamer_counters: &EventStreamerCounters,
		indexer_counters: &IndexerCounters,
		storage_mapper_counters: &StorageMapperCounters,
		ingestor_counters: &IngestorCounters,
		collection_counters: Vec<serde_json::Value>,
	) -> Self {
		self.total_docs = 0;
		self.total_data_processed_size = 0;
		let qflow_counters: EventsCounter =
			serde_json::from_value(qflow_counters.clone()).unwrap_or_default();
		for counter in collection_counters {
			let counter: CollectionCounter = serde_json::from_value(counter).unwrap_or_default();
			let total_docs_u64 = counter.total_docs.load(Ordering::Relaxed);
			self.total_docs += total_docs_u64 as u32;
		}
		self.total_events = qflow_counters.total.load(Ordering::Relaxed) as u32;
		self.total_events_processed = qflow_counters.processed.load(Ordering::Relaxed) as u32;
		self.total_events_received =
			event_streamer_counters.events_received.load(Ordering::Relaxed) as u32;
		self.total_events_sent =
			event_streamer_counters.events_processed.load(Ordering::Relaxed) as u32;
		self.total_batches =
			event_streamer_counters.batches_received.load(Ordering::Relaxed) as u32;
		self.total_sentences =
			indexer_counters.total_sentences_indexed.load(Ordering::Relaxed) as u32;
		self.total_subjects =
			indexer_counters.total_subjects_indexed.load(Ordering::Relaxed) as u32;
		self.total_predicates =
			indexer_counters.total_predicates_indexed.load(Ordering::Relaxed) as u32;
		self.total_objects = indexer_counters.total_objects_indexed.load(Ordering::Relaxed) as u32;
		let total_event_map = &storage_mapper_counters.event_count_map;
		for (event_type, counter) in total_event_map {
			match event_type {
				EventType::Graph => {
					self.total_graph_events = counter.load(Ordering::Relaxed) as u32;
				},
				EventType::Vector => {
					self.total_vector_events = counter.load(Ordering::Relaxed) as u32;
				},
				_ => {},
			}
		}
		self.total_data_processed_size +=
			ingestor_counters.total_megabytes.load(Ordering::Relaxed) as u32;
		self
	}
}

impl Display for IndexingStatistics {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"total_docs: {}, total_events: {}, total_events_processed: {}, total_events_received: {}, total_events_sent: {}, total_batches: {}, total_sentences: {}, total_subjects: {}, total_predicates: {}, total_objects: {}, total_graph_events: {}, total_vector_events: {}",
			self.total_docs,
			self.total_events,
			self.total_events_processed,
			self.total_events_received,
			self.total_events_sent,
			self.total_batches,
			self.total_sentences,
			self.total_subjects,
			self.total_predicates,
			self.total_objects,
			self.total_graph_events,
			self.total_vector_events,
		)
	}
}

#[derive(Clone, Default, Eq, PartialEq, Hash, Ord, PartialOrd, utoipa::ToSchema, specta::Type)]
pub enum StorageType {
	Index,
	Vector,
	#[default]
	Graph,
}

impl Display for StorageType {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Index => write!(f, "index"),
			Self::Vector => write!(f, "vector"),
			Self::Graph => write!(f, "graph"),
		}
	}
}

impl Debug for StorageType {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Index => write!(f, "index"),
			Self::Vector => write!(f, "vector"),
			Self::Graph => write!(f, "graph"),
		}
	}
}

impl StorageType {
	pub fn as_bytes(&self) -> Bytes {
		match self {
			Self::Index => Bytes::from("index"),
			Self::Vector => Bytes::from("vector"),
			Self::Graph => Bytes::from("graph"),
		}
	}

	pub fn from_i32(value: i32) -> Self {
		match value {
			0 => Self::Index,
			1 => Self::Vector,
			2 => Self::Graph,
			_ => panic!("invalid storage type"),
		}
	}

	pub fn as_i32(&self) -> i32 {
		match self {
			Self::Index => 0,
			Self::Vector => 1,
			Self::Graph => 2,
		}
	}

	pub fn as_str(&self) -> &str {
		match self {
			Self::Index => "index",
			Self::Vector => "vector",
			Self::Graph => "graph",
		}
	}
}

impl From<ByteString> for StorageType {
	fn from(value: ByteString) -> Self {
		match &value[..] {
			"index" | "Index" => Self::Index,
			"vector" | "Vector" => Self::Vector,
			"graph" | "Graph" => Self::Graph,
			_ => panic!("invalid storage type"),
		}
	}
}

impl From<String> for StorageType {
	fn from(storage_type: String) -> Self {
		Self::from(ByteString::from(storage_type))
	}
}

impl Serialize for StorageType {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.collect_str(self)
	}
}

impl<'de> Deserialize<'de> for StorageType {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let type_str = String::deserialize(deserializer)?;
		Ok(Self::from(type_str))
	}
}

impl PartialEq<StorageType> for &StorageType {
	#[inline]
	fn eq(&self, other: &StorageType) -> bool {
		*self == other
	}
}

impl prost::Message for StorageType {
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
		const STRUCT_NAME: &str = "StorageType";

		match tag {
			1u32 => {
				let mut value = Vec::new();
				prost::encoding::bytes::merge(wire_type, &mut value, buf, ctx).map_err(
					|mut error| {
						error.push(STRUCT_NAME, "storage_type");
						error
					},
				)?;
				let byte_string = ByteString::try_from(value)
					.map_err(|_| DecodeError::new("storage_type is not valid UTF-8"))?;
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
