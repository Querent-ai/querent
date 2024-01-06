use std::{collections::HashMap, sync::atomic::Ordering};

use querent_synapse::{
	callbacks::EventType,
	comm::{MessageState, MessageType},
};
use serde::Serialize;

use crate::{
	indexer::IndexerCounters, EventStreamerCounters, EventsCounter, StorageMapperCounters,
};

#[derive(Clone, Debug)]
pub struct MessageStateBatches {
	pub pipeline_id: String,
	pub message_state_batches: HashMap<MessageType, Vec<MessageState>>,
}

/// A Struct that holds all statistical data about indexing
#[derive(Clone, Debug, Default, Serialize)]
pub struct IndexingStatistics {
	/// Number of document processed (valid or not)
	pub total_docs: u64,
	/// Number events processed
	pub total_events: u64,
	/// Number of events processed
	pub total_events_processed: u64,
	/// Number of events received by the event streamer
	pub total_events_received: u64,
	/// Number of events sent by the event streamer
	pub total_events_sent: u64,
	/// Number of batches processed
	pub total_batches: u64,
	/// Number of sentences processed
	pub total_sentences: u64,
	/// Number of subjects processed
	pub total_subjects: u64,
	/// Number of predicates processed
	pub total_predicates: u64,
	/// Number of objects processed
	pub total_objects: u64,
	/// Number of graph events processed
	pub total_graph_events: u64,
	/// Number of vector events processed
	pub total_vector_events: u64,
	/// Number of graph events sent to storage
	pub total_graph_events_sent: u64,
	/// Number of vector events sent to storage
	pub total_vector_events_sent: u64,
	/// Number of semantic knowledge indexed
	pub total_semantic_knowledge: u64,
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
		self.total_events = qflow_counters.total.load(Ordering::Relaxed);
		self.total_events_processed = qflow_counters.processed.load(Ordering::Relaxed);
		self.total_events_received =
			event_streamer_counters.events_received.load(Ordering::Relaxed);
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
