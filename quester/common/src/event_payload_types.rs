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
            "MERGE (:{entity_type1} {{name: $entity1 }})-[:{predicate_type} {{sentence: $sentence, document_id: $document_id}}]->(:{entity_type2} {{name: $entity2}})",
            entity_type1 = &self.subject_type,
            predicate_type = &self.predicate_type,
            entity_type2 = &self.object_type,
        )
	}
}
