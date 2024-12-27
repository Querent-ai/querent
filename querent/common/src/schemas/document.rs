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

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// The `Document` struct represents a document with content, metadata, and a score.
/// The `page_content` field is a string that contains the content of the document.
/// The `metadata` field is a `HashMap` where the keys represent metadata properties and the values represent property values.
/// The `score` field represents a relevance score for the document and is a floating point number.
///
/// # Usage
/// ```rust,ignore
/// let my_doc = Document::new("This is the document content.".to_string())
///    .with_metadata({
///       let mut metadata = HashMap::new();
///       metadata.insert("author".to_string(), json!("John Doe"));
///       metadata
///   })
///    .with_score(0.75);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
	pub page_content: String,
	pub metadata: HashMap<String, Value>,
	pub score: f64,
}

impl Document {
	/// Constructs a new `Document` with provided `page_content`, an empty `metadata` map and a `score` of 0.
	pub fn new<S: Into<String>>(page_content: S) -> Self {
		Document { page_content: page_content.into(), metadata: HashMap::new(), score: 0.0 }
	}

	/// Sets the `metadata` Map of the `Document` to the provided HashMap.
	pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
		self.metadata = metadata;
		self
	}

	/// Sets the `score` of the `Document` to the provided float.
	pub fn with_score(mut self, score: f64) -> Self {
		self.score = score;
		self
	}
}

impl Default for Document {
	/// Provides a default `Document` with an empty `page_content`, an empty `metadata` map and a `score` of 0.
	fn default() -> Self {
		Document { page_content: "".to_string(), metadata: HashMap::new(), score: 0.0 }
	}
}
