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

pub mod llm;
pub use llm::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub mod schemas;
pub mod transformers;
pub mod utils;
pub use schemas::*;
pub mod generative;
pub use generative::*;
pub mod options;
pub use options::*;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GenerateResult {
	pub tokens: Option<TokenUsage>,
	pub generation: String,
}

impl GenerateResult {
	pub fn to_hashmap(&self) -> HashMap<String, String> {
		let mut map = HashMap::new();

		// Insert the 'generation' field into the hashmap
		map.insert("generation".to_string(), self.generation.clone());

		// Check if 'tokens' is Some and insert its fields into the hashmap
		if let Some(ref tokens) = self.tokens {
			map.insert("prompt_tokens".to_string(), tokens.prompt_tokens.to_string());
			map.insert("completion_tokens".to_string(), tokens.completion_tokens.to_string());
			map.insert("total_tokens".to_string(), tokens.total_tokens.to_string());
		}

		map
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TokenUsage {
	pub prompt_tokens: u32,
	pub completion_tokens: u32,
	pub total_tokens: u32,
}

impl TokenUsage {
	pub fn sum(&self, other: &TokenUsage) -> TokenUsage {
		TokenUsage {
			prompt_tokens: self.prompt_tokens + other.prompt_tokens,
			completion_tokens: self.completion_tokens + other.completion_tokens,
			total_tokens: self.total_tokens + other.total_tokens,
		}
	}

	pub fn add(&mut self, other: &TokenUsage) {
		self.prompt_tokens += other.prompt_tokens;
		self.completion_tokens += other.completion_tokens;
		self.total_tokens += other.total_tokens;
	}
}

impl TokenUsage {
	pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
		Self { prompt_tokens, completion_tokens, total_tokens: prompt_tokens + completion_tokens }
	}
}
