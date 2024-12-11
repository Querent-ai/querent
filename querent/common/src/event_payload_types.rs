// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.ai).

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
	pub query: Option<String>,
	pub session_id: Option<String>,
	pub score: f32,
	pub collection_id: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct VectorPayload {
	pub event_id: String,
	pub embeddings: Vec<f32>,
	pub score: f32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
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
