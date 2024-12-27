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

use async_openai::config::Config;
use reqwest::header::HeaderMap;
use secrecy::Secret;
use serde::Deserialize;

const OLLAMA_API_BASE: &str = "http://localhost:11434/v1";

/// Ollama has [OpenAI compatiblity](https://ollama.com/blog/openai-compatibility), meaning that you can use it as an OpenAI API.
///
/// This struct implements the `Config` trait of OpenAI, and has the necessary setup for OpenAI configurations for you to use Ollama.
///
/// ## Example
///
/// ```rs
/// let ollama = OpenAI::new(OllamaConfig::default()).with_model("llama3");
/// let response = ollama.invoke("Say hello!").await.unwrap();
/// ```
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct OllamaConfig {
	api_key: Secret<String>,
}

impl OllamaConfig {
	pub fn new() -> Self {
		Self::default()
	}
}

impl Config for OllamaConfig {
	fn api_key(&self) -> &Secret<String> {
		&self.api_key
	}

	fn api_base(&self) -> &str {
		OLLAMA_API_BASE
	}

	fn headers(&self) -> HeaderMap {
		HeaderMap::default()
	}

	fn query(&self) -> Vec<(&str, &str)> {
		vec![]
	}

	fn url(&self, path: &str) -> String {
		format!("{}{}", self.api_base(), path)
	}
}

impl Default for OllamaConfig {
	fn default() -> Self {
		Self { api_key: Secret::new("ollama".to_string()) }
	}
}
