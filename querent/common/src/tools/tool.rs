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

use std::error::Error;

use async_trait::async_trait;
use serde_json::{json, Value};

#[async_trait]
pub trait Tool: Send + Sync {
	/// Returns the name of the tool.
	fn name(&self) -> String;

	/// Provides a description of what the tool does and when to use it.
	fn description(&self) -> String;
	/// This are the parametters for OpenAi-like function call.
	/// You should return a jsnon like this one
	/// ```json
	/// {
	///     "type": "object",
	///     "properties": {
	///         "command": {
	///             "type": "string",
	///             "description": "The raw command you want executed"
	///                 }
	///     },
	///     "required": ["command"]
	/// }
	///
	/// If there s no implementation the defaul will be the self.description()
	///```
	fn parameters(&self) -> Value {
		json!({
			"type": "object",
				"properties": {
				"input": {
					"type": "string",
					"description":self.description()
				}
			},
			"required": ["input"]
		})
	}

	/// Processes an input string and executes the tool's functionality, returning a `Result`.
	///
	/// This function utilizes `parse_input` to parse the input and then calls `run`.
	/// Its used by the Agent
	async fn call(&self, input: &str) -> Result<String, Box<dyn Error>> {
		let input = self.parse_input(input).await;
		self.run(input).await
	}

	/// Executes the core functionality of the tool.
	///
	/// Example implementation:
	/// ```rust,ignore
	/// async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
	///     let input_str = input.as_str().ok_or("Input should be a string")?;
	///     self.simple_search(input_str).await
	/// }
	/// ```
	async fn run(&self, input: Value) -> Result<String, Box<dyn Error>>;

	/// Parses the input string, which could be a JSON value or a raw string, depending on the LLM model.
	///
	/// Implement this function to extract the parameters needed for your tool. If a simple
	/// string is sufficient, the default implementation can be used.
	async fn parse_input(&self, input: &str) -> Value {
		log::info!("Using default implementation: {}", input);
		match serde_json::from_str::<Value>(input) {
			Ok(input) =>
				if input["input"].is_string() {
					Value::String(input["input"].as_str().unwrap().to_string())
				} else {
					Value::String(input.to_string())
				},
			Err(_) => Value::String(input.to_string()),
		}
	}
}
