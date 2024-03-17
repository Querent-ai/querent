use actors::AskError;
use common::{EventStreamerCounters, EventsCounter, IndexerCounters, StorageMapperCounters};
use querent_synapse::callbacks::EventType;
use std::{
	fmt::{Display, Formatter},
	sync::atomic::Ordering,
};

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

	/// Increment the number of semantic knowledge indexed
	pub fn increment_total_semantic_knowledge(&mut self) {
		self.total_semantic_knowledge += 1;
	}

	pub fn add_counters(
		mut self,
		qflow_counters: &serde_json::Value,
		event_streamer_counters: &EventStreamerCounters,
		indexer_counters: &IndexerCounters,
		storage_mapper_counters: &StorageMapperCounters,
	) -> Self {
		let qflow_counters: EventsCounter =
			serde_json::from_value(qflow_counters.clone()).unwrap_or_default();
		self.total_events = qflow_counters.total.load(Ordering::Relaxed) as u64;
		self.total_events_processed = qflow_counters.processed.load(Ordering::Relaxed) as u64;
		self.total_events_received =
			event_streamer_counters.events_received.load(Ordering::Relaxed) as u64;
		self.total_events_sent = event_streamer_counters.events_processed.load(Ordering::Relaxed);
		self.total_batches = event_streamer_counters.batches_received.load(Ordering::Relaxed);
		self.total_docs = indexer_counters.total_documents_indexed.load(Ordering::Relaxed);
		self.total_sentences = indexer_counters.total_sentences_indexed.load(Ordering::Relaxed);
		self.total_subjects = indexer_counters.total_subjects_indexed.load(Ordering::Relaxed);
		self.total_predicates = indexer_counters.total_predicates_indexed.load(Ordering::Relaxed);
		self.total_objects = indexer_counters.total_objects_indexed.load(Ordering::Relaxed);
		let total_event_map = &storage_mapper_counters.event_count_map;
		for (event_type, counter) in total_event_map {
			match event_type {
				EventType::Graph => {
					self.total_graph_events = counter.load(Ordering::Relaxed);
				},
				EventType::Vector => {
					self.total_vector_events = counter.load(Ordering::Relaxed);
				},
				_ => {},
			}
		}
		let total_event_to_storage_map = &storage_mapper_counters.event_to_storage_map;
		for (event_type, counter) in total_event_to_storage_map {
			match event_type {
				EventType::Graph => {
					self.total_graph_events_sent = counter.load(Ordering::Relaxed);
				},
				EventType::Vector => {
					self.total_vector_events_sent = counter.load(Ordering::Relaxed);
				},
				_ => {},
			}
		}
		self
	}
}

impl Display for IndexingStatistics {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"total_docs: {}, total_events: {}, total_events_processed: {}, total_events_received: {}, total_events_sent: {}, total_batches: {}, total_sentences: {}, total_subjects: {}, total_predicates: {}, total_objects: {}, total_graph_events: {}, total_vector_events: {}, total_graph_events_sent: {}, total_vector_events_sent: {}, total_semantic_knowledge: {}",
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
			self.total_graph_events_sent,
			self.total_vector_events_sent,
			self.total_semantic_knowledge,
		)
	}
}
