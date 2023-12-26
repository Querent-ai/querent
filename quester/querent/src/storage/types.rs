use common::{SemanticKnowledgePayload, VectorPayload};
use querent_synapse::callbacks::{EventState, EventType};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ContextualTriples {
	event_type: EventType,
	pub qflow_id: String,
	pub triple_states: Vec<EventState>,
	pub timestamp: u64,
}

impl ContextualTriples {
	pub fn new(qflow_id: String, triple_states: Vec<EventState>, timestamp: u64) -> Self {
		Self { event_type: EventType::Graph, qflow_id, triple_states, timestamp }
	}

	pub fn is_empty(&self) -> bool {
		self.triple_states.is_empty()
	}

	pub fn len(&self) -> usize {
		self.triple_states.len()
	}

	pub fn timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn qflow_id(&self) -> String {
		self.qflow_id.clone()
	}

	pub fn triples(&self) -> &Vec<EventState> {
		&self.triple_states
	}

	pub fn event_type(&self) -> EventType {
		self.event_type.clone()
	}

	pub fn event_payload(&self) -> Vec<(String, SemanticKnowledgePayload)> {
		self.triple_states
			.iter()
			.map(|x| (x.file.clone(), serde_json::from_str(&x.payload).unwrap_or_default()))
			.collect()
	}
}

#[derive(Debug, Serialize)]
pub struct ContextualEmbeddings {
	event_type: EventType,
	pub qflow_id: String,
	pub vector_states: Vec<EventState>,
	pub timestamp: u64,
}

impl ContextualEmbeddings {
	pub fn new(qflow_id: String, vector_states: Vec<EventState>, timestamp: u64) -> Self {
		Self { event_type: EventType::Vector, qflow_id, vector_states, timestamp }
	}

	pub fn is_empty(&self) -> bool {
		self.vector_states.is_empty()
	}

	pub fn len(&self) -> usize {
		self.vector_states.len()
	}

	pub fn timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn qflow_id(&self) -> String {
		self.qflow_id.clone()
	}

	pub fn triples(&self) -> &Vec<EventState> {
		&self.vector_states
	}

	pub fn event_type(&self) -> EventType {
		self.event_type.clone()
	}

	pub fn event_payload(&self) -> Vec<(String, VectorPayload)> {
		self.vector_states
			.iter()
			.map(|x| (x.file.clone(), serde_json::from_str(&x.payload).unwrap_or_default()))
			.collect()
	}
}

#[derive(Debug, Serialize)]
pub struct IndexerKnowledge {
	pub qflow_id: String,
	pub timestamp: u64,
	pub triples: Vec<(String, SemanticKnowledgePayload)>,
}

impl IndexerKnowledge {
	pub fn new(
		qflow_id: String,
		timestamp: u64,
		triples: Vec<(String, SemanticKnowledgePayload)>,
	) -> Self {
		Self { qflow_id, timestamp, triples }
	}
}
