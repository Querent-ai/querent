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

use async_trait::async_trait;
use proto::semantics::IngestedTokens;
use regex::Regex;

use crate::{AsyncProcessor, IngestorResult};

pub struct TextCleanupProcessor;

impl TextCleanupProcessor {
	pub fn new() -> Self {
		Self {}
	}

	async fn cleanup_text(&self, data: &str) -> String {
		let mut data = data.to_string();

		let cleaned_data: String = data
			.split_whitespace()
			.filter(|word| word.len() <= 50)
			.collect::<Vec<&str>>()
			.join(" ");

		data = cleaned_data.replace("\"", "").replace('“', "").replace('”', "");
		data = data.replace("\\n", " ").replace("\\t", " ");

		// Replace special characters (except letters, numbers, and spaces) with a space
		let re_special = Regex::new(r"[^a-zA-Z0-9\s]").unwrap();
		data = re_special.replace_all(&data, " ").to_string();

		let re_hex = Regex::new(r"\\x[0-9a-fA-F]{2}").unwrap();
		data = re_hex.replace_all(&data, "").to_string();

		let re_control = Regex::new(r"[\x00-\x1F\x7F]+").unwrap();
		data = re_control.replace_all(&data, "").to_string();
		let re_whitespace = Regex::new(r"\s+").unwrap();
		data = re_whitespace.replace_all(&data, " ").to_string();

		data = data.chars().filter(|&c| c.is_ascii() || !c.is_control()).collect();

		data
	}
}

#[async_trait]
impl AsyncProcessor for TextCleanupProcessor {
	async fn process_text(&self, data: IngestedTokens) -> IngestorResult<IngestedTokens> {
		let tokens = data.clone().data.clone();
		let mut cleaned_tokens = Vec::new();
		for token in tokens.iter() {
			let cleaned_text = self.cleanup_text(&token).await;
			cleaned_tokens.push(cleaned_text.clone());
		}

		let mut data = data.clone();
		data.data = cleaned_tokens;
		Ok(data)
	}
}
