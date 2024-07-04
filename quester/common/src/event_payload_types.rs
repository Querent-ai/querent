use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct DocumentPayload {
	pub doc_id: String,
	pub doc_source: String,
	pub sentence: String,
	pub knowledge: String,
	pub subject: String,
	pub object: String,
	pub cosine_distance: Option<f64>,
	pub query_embedding: Option<Vec<f32>>,
	pub session_id: Option<String>,
	pub score: f32,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct VectorPayload {
	pub event_id: String,
	pub embeddings: Vec<f32>,
	pub score: f32,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SemanticKnowledgePayload {
	pub subject: String,
	pub subject_type: String,
	pub object: String,
	pub object_type: String,
	pub predicate: String,
	pub predicate_type: String,
	pub sentence: String,
	pub image_id: Option<String>,
	pub blob: Option<String>,
	pub event_id: String,
	pub source_id: String,
}

impl SemanticKnowledgePayload {
	pub fn to_cypher_query(&self) -> String {
		format!(
			"MERGE (n1:`{entity_type1}` {{name: $entity1}}) \
			MERGE (n2:`{entity_type2}` {{name: $entity2}}) \
			MERGE (n1)-[:`{predicate}` {{sentence: $sentence, document_id: $document_id, document_source: $document_source, predicate_type: $predicate_type, image_id: $image_id}}]->(n2)",
			entity_type1 = &self.subject_type,
			predicate = &self.predicate,
			entity_type2 = &self.object_type,
		)
	}
}
