use serde::Serialize;

/// A Struct that holds all statistical data about indexing
#[derive(Clone, Debug, Default, Serialize)]
pub struct IndexingStatistics {
	/// Number of document processed (valid or not)
	pub total_docs: u64,
	/// Number events processed
	pub total_events: u64,
	/// Number of graph events processed
	pub total_graph_events: u64,
	/// Number of vector events processed
	pub total_vector_events: u64,
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
}
