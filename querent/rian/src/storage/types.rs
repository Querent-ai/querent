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

use common::{EventState, EventType, SemanticKnowledgePayload, VectorPayload};
use serde::Serialize;
use tracing::error;

#[derive(Debug, Serialize, Clone)]
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

	pub fn event_payload(&self) -> Vec<(String, String, Option<String>, SemanticKnowledgePayload)> {
		let mut triples: Vec<(String, String, Option<String>, SemanticKnowledgePayload)> =
			Vec::new();
		for triple in &self.triple_states {
			let payload = serde_json::from_str(&triple.payload);
			match payload {
				Ok(payload) => triples.push((
					triple.file.clone(),
					triple.doc_source.clone(),
					triple.image_id.clone(),
					payload,
				)),
				Err(e) => error!("Failed to deserialize payload: {:?}", e),
			}
		}
		triples
	}
}

#[derive(Debug, Serialize, Clone)]
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

	pub fn event_payload(&self) -> Vec<(String, String, Option<String>, VectorPayload)> {
		self.vector_states
			.iter()
			.map(|x| {
				(
					x.file.clone(),
					x.doc_source.clone(),
					x.image_id.clone(),
					serde_json::from_str(&x.payload).unwrap_or_default(),
				)
			})
			.collect()
	}
}

#[derive(Debug, Serialize, Clone)]
pub struct IndexerKnowledge {
	pub qflow_id: String,
	pub timestamp: u64,
	pub triples: Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
}

impl IndexerKnowledge {
	pub fn new(
		qflow_id: String,
		timestamp: u64,
		triples: Vec<(String, String, Option<String>, SemanticKnowledgePayload)>,
	) -> Self {
		Self { qflow_id, timestamp, triples }
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

	pub fn triples(&self) -> &Vec<(String, String, Option<String>, SemanticKnowledgePayload)> {
		&self.triples
	}
}
