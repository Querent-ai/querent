use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct VectorPayload {
	pub id: String,
	pub embeddings: Vec<f32>,
	pub size: u64,
	pub namespace: String,
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
}

impl SemanticKnowledgePayload {
	pub fn to_cypher_query(&self) -> String {
		format!(
            "CREATE (:{subject_type} {{subject: $subject}})-[:{predicate_type} {{predicate: $predicate}}]->(:{object_type} {{object: $object}}) SET sentence = $sentence",
            subject_type = &self.subject_type,
            predicate_type = &self.predicate_type,
            object_type = &self.object_type,
        )
	}
}
