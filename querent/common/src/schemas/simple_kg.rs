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

use crate::DocumentPayload;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub struct Node {
	id: String,
	label: String,
	#[serde(rename = "type")]
	node_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Edge {
	source: String,
	target: String,
	#[serde(rename = "type")]
	edge_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphData {
	nodes: Vec<Node>,
	edges: Vec<Edge>,
}

impl GraphData {
	pub fn from_documents(docs_vec: Vec<DocumentPayload>) -> Self {
		let mut nodes = Vec::new();
		let mut edges = Vec::new();
		let mut node_ids = HashSet::new();
		for doc in docs_vec {
			// Add subject node
			let subject_node = Node {
				id: doc.subject.clone(),
				label: doc.subject.clone(),
				node_type: "subject".to_string(),
			};
			if node_ids.insert(subject_node.clone()) {
				nodes.push(subject_node)
			};

			// Add object node
			let object_node = Node {
				id: doc.object.clone(),
				label: doc.object.clone(),
				node_type: "object".to_string(),
			};
			if node_ids.insert(object_node.clone()) {
				nodes.push(object_node)
			};

			let document_node = Node {
				id: doc.doc_id.clone(),
				label: format!("Document: {}", doc.doc_id),
				node_type: "document".to_string(),
			};
			if node_ids.insert(document_node.clone()) {
				nodes.push(document_node)
			};

			// Add sentence node
			let sentence_node = Node {
				id: format!("Sentence_{}", doc.doc_id),
				label: doc.sentence.clone(),
				node_type: "sentence".to_string(),
			};
			if node_ids.insert(sentence_node.clone()) {
				nodes.push(sentence_node)
			};

			// Add edges
			let subject_to_document_edge = Edge {
				source: doc.subject.clone(),
				target: doc.doc_id.clone(),
				edge_type: "contains".to_string(),
			};
			edges.push(subject_to_document_edge);

			let document_to_sentence_edge = Edge {
				source: doc.doc_id.clone(),
				target: format!("Sentence_{}", doc.doc_id),
				edge_type: "has_sentence".to_string(),
			};
			edges.push(document_to_sentence_edge);

			let document_to_object_edge = Edge {
				source: doc.doc_id.clone(),
				target: doc.object.clone(),
				edge_type: "mentions".to_string(),
			};
			edges.push(document_to_object_edge);
		}

		GraphData { nodes, edges }
	}
}
