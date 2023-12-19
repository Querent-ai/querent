use querent_synapse::callbacks::{EventState, EventType};

pub struct ContextualTriples {
	event_type: EventType,
	pub qflow_id: String,
	pub triples: Vec<EventState>,
	pub timestamp: u64,
}

impl ContextualTriples {
	pub fn new(qflow_id: String, triples: Vec<EventState>, timestamp: u64) -> Self {
		Self { event_type: EventType::ContextualTriples, qflow_id, triples, timestamp }
	}

	pub fn is_empty(&self) -> bool {
		self.triples.is_empty()
	}

	pub fn len(&self) -> usize {
		self.triples.len()
	}

	pub fn timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn qflow_id(&self) -> String {
		self.qflow_id.clone()
	}

	pub fn triples(&self) -> Vec<EventState> {
		self.triples.clone()
	}

	pub fn event_type(&self) -> EventType {
		self.event_type.clone()
	}
}

pub struct RdfContextualTriples {
	event_type: EventType,
	pub qflow_id: String,
	pub triples: Vec<EventState>,
	pub timestamp: u64,
}

impl RdfContextualTriples {
	pub fn new(qflow_id: String, triples: Vec<EventState>, timestamp: u64) -> Self {
		Self { event_type: EventType::RdfContextualTriples, qflow_id, triples, timestamp }
	}

	pub fn is_empty(&self) -> bool {
		self.triples.is_empty()
	}

	pub fn len(&self) -> usize {
		self.triples.len()
	}

	pub fn timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn qflow_id(&self) -> String {
		self.qflow_id.clone()
	}

	pub fn triples(&self) -> Vec<EventState> {
		self.triples.clone()
	}

	pub fn event_type(&self) -> EventType {
		self.event_type.clone()
	}
}

pub struct RdfSemanticTriples {
	event_type: EventType,
	pub qflow_id: String,
	pub triples: Vec<EventState>,
	pub timestamp: u64,
}

impl RdfSemanticTriples {
	pub fn new(qflow_id: String, triples: Vec<EventState>, timestamp: u64) -> Self {
		Self { event_type: EventType::RdfSemanticTriples, qflow_id, triples, timestamp }
	}

	pub fn is_empty(&self) -> bool {
		self.triples.is_empty()
	}

	pub fn len(&self) -> usize {
		self.triples.len()
	}

	pub fn timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn qflow_id(&self) -> String {
		self.qflow_id.clone()
	}

	pub fn triples(&self) -> Vec<EventState> {
		self.triples.clone()
	}

	pub fn event_type(&self) -> EventType {
		self.event_type.clone()
	}
}

pub struct ContextualEmbeddings {
	event_type: EventType,
	pub qflow_id: String,
	pub triples: Vec<EventState>,
	pub timestamp: u64,
}

impl ContextualEmbeddings {
	pub fn new(qflow_id: String, triples: Vec<EventState>, timestamp: u64) -> Self {
		Self { event_type: EventType::ContextualEmbeddings, qflow_id, triples, timestamp }
	}

	pub fn is_empty(&self) -> bool {
		self.triples.is_empty()
	}

	pub fn len(&self) -> usize {
		self.triples.len()
	}

	pub fn timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn qflow_id(&self) -> String {
		self.qflow_id.clone()
	}

	pub fn triples(&self) -> Vec<EventState> {
		self.triples.clone()
	}

	pub fn event_type(&self) -> EventType {
		self.event_type.clone()
	}
}
