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

use crate::{
	llm::LLM, options::CallOptions, GenerateResult, LLMError, LLMErrorKind, LLMResult, Message,
	StreamData, TokenUsage,
};
use async_trait::async_trait;
use candle_core::Tensor;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::{collections::HashMap, pin::Pin, sync::Arc};

use super::models::{ApiResponse, ClaudeMessage, Payload};

pub enum ClaudeModel {
	Claude3pus20240229,
	Claude3sonnet20240229,
	Claude3haiku20240307,
}

impl ToString for ClaudeModel {
	fn to_string(&self) -> String {
		match self {
			ClaudeModel::Claude3pus20240229 => "claude-3-opus-20240229".to_string(),
			ClaudeModel::Claude3sonnet20240229 => "claude-3-sonnet-20240229".to_string(),
			ClaudeModel::Claude3haiku20240307 => "claude-3-haiku-20240307".to_string(),
		}
	}
}

#[derive(Clone)]
pub struct Claude {
	model: String,
	options: CallOptions,
	api_key: String,
	anthropic_version: String,
}

impl Default for Claude {
	fn default() -> Self {
		Self::new()
	}
}

impl Claude {
	pub fn new() -> Self {
		Self {
			model: ClaudeModel::Claude3pus20240229.to_string(),
			options: CallOptions::default(),
			api_key: std::env::var("CLAUDE_API_KEY").unwrap_or_default(),
			anthropic_version: "2023-06-01".to_string(),
		}
	}

	pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
		self.model = model.into();
		self
	}

	pub fn with_options(mut self, options: CallOptions) -> Self {
		self.options = options;
		self
	}

	pub fn with_api_key<S: Into<String>>(mut self, api_key: S) -> Self {
		self.api_key = api_key.into();
		self
	}

	pub fn with_anthropic_version<S: Into<String>>(mut self, version: S) -> Self {
		self.anthropic_version = version.into();
		self
	}

	async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError> {
		let client = Client::new();
		let is_stream = self.options.streaming_func.is_some();

		let payload = self.build_payload(messages, is_stream);
		let res = client
			.post("https://api.anthropic.com/v1/messages")
			.header("x-api-key", &self.api_key)
			.header("anthropic-version", self.anthropic_version.clone())
			.header("content-type", "application/json; charset=utf-8")
			.json(&payload)
			.send()
			.await
			.map_err(|e| {
				LLMError::new(
					LLMErrorKind::AnthropicError("request".to_string()),
					Arc::new(e.into()),
				)
			})?;
		let res = match res.status().as_u16() {
			401 => Err(LLMError::new(
				LLMErrorKind::AnthropicError("Authentication Error".to_string()),
				Arc::new(anyhow::anyhow!("Authentication Error").into()),
			)),
			403 => Err(LLMError::new(
				LLMErrorKind::AnthropicError("Permission Error".to_string()),
				Arc::new(anyhow::anyhow!("Permission Error").into()),
			)),
			404 => Err(LLMError::new(
				LLMErrorKind::AnthropicError("Not Found Error".to_string()),
				Arc::new(anyhow::anyhow!("Not Found Error").into()),
			)),
			429 => Err(LLMError::new(
				LLMErrorKind::AnthropicError("Rate Limit Error".to_string()),
				Arc::new(anyhow::anyhow!("Rate Limit Error").into()),
			)),
			503 => Err(LLMError::new(
				LLMErrorKind::AnthropicError("Overloaded Error".to_string()),
				Arc::new(anyhow::anyhow!("Overloaded Error").into()),
			)),
			_ => Ok(res.json::<ApiResponse>().await.map_err(|e| {
				LLMError::new(
					LLMErrorKind::AnthropicError("response".to_string()),
					Arc::new(e.into()),
				)
			})?),
		}?;

		let generation = res.content.first().map(|c| c.text.clone()).unwrap_or_default();

		let tokens = Some(TokenUsage {
			prompt_tokens: res.usage.input_tokens,
			completion_tokens: res.usage.output_tokens,
			total_tokens: res.usage.input_tokens + res.usage.output_tokens,
		});

		Ok(GenerateResult { tokens, generation })
	}

	fn build_payload(&self, messages: &[Message], stream: bool) -> Payload {
		let mut payload = Payload {
			model: self.model.clone(),
			messages: messages.iter().map(ClaudeMessage::from_message).collect::<Vec<_>>(),
			max_tokens: self.options.max_tokens.unwrap_or(1024),
			stream: None,
			stop_sequences: self.options.stop_words.clone(),
			temperature: self.options.temperature,
			top_p: self.options.top_p,
			top_k: self.options.top_k,
		};
		if stream {
			payload.stream = Some(true);
		}
		payload
	}

	async fn stream(
		&self,
		messages: &[Message],
	) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamData>> + Send>>> {
		let client = Client::new();
		let payload = self.build_payload(messages, true);
		let request = client
			.post("https://api.anthropic.com/v1/messages")
			.header("x-api-key", &self.api_key)
			.header("anthropic-version", &self.anthropic_version)
			.header("content-type", "application/json; charset=utf-8")
			.json(&payload)
			.build()
			.map_err(|e| {
				LLMError::new(
					LLMErrorKind::AnthropicError("request".to_string()),
					Arc::new(e.into()),
				)
			})?;

		// Instead of sending the request directly, return a stream wrapper
		let stream = client.execute(request).await.map_err(|e| {
			LLMError::new(LLMErrorKind::AnthropicError("request".to_string()), Arc::new(e.into()))
		})?;
		let stream = stream.bytes_stream();
		// Process each chunk as it arrives
		let processed_stream = stream.then(move |result| {
			async move {
				match result {
					Ok(bytes) => {
						let value: Value = parse_sse_to_json(&String::from_utf8_lossy(&bytes))?;
						if value["type"].as_str().unwrap_or("") == "content_block_delta" {
							let content = value["delta"]["text"].clone();
							// Return StreamData based on the parsed content
							Ok(StreamData::new(value, content.as_str().unwrap_or("")))
						} else {
							Ok(StreamData::new(value, ""))
						}
					},
					Err(e) => Err(LLMError::new(LLMErrorKind::Io, Arc::new(e.into()))),
				}
			}
		});

		Ok(Box::pin(processed_stream))
	}
}

#[async_trait]
impl LLM for Claude {
	async fn init_token_idx_2_word_doc_idx(&self) -> Vec<(String, i32)> {
		Vec::new()
	}
	async fn num_start_tokens(&self) -> usize {
		0
	}
	async fn append_last_token(&self, listing: &mut Vec<(String, i32)>) {
		listing.push(("".to_string(), 0));
	}
	async fn model_input(
		&self,
		_tokenized_sequence: Vec<i32>,
	) -> LLMResult<std::collections::HashMap<String, Tensor>> {
		Ok(std::collections::HashMap::new())
	}
	async fn tokenize(&self, _word: &str) -> LLMResult<Vec<i32>> {
		Ok(Vec::new())
	}
	async fn inference_attention(
		&self,
		_model_input: std::collections::HashMap<String, Tensor>,
	) -> LLMResult<Tensor> {
		Err(LLMError::new(LLMErrorKind::Io, Arc::new(anyhow::anyhow!("Not implemented"))))
	}
	async fn maximum_tokens(&self) -> usize {
		0
	}

	async fn tokens_to_words(&self, _tokens: &[i32]) -> Vec<String> {
		Vec::new()
	}

	async fn attention_tensor_to_2d_vector(
		&self,
		_attention_weights: &Tensor,
	) -> LLMResult<Vec<Vec<f32>>> {
		Ok(Vec::new())
	}

	async fn token_classification(
		&self,
		_model_input: std::collections::HashMap<String, Tensor>,
		_labels: Option<&Tensor>,
	) -> LLMResult<Vec<(String, String)>> {
		Ok(Vec::new())
	}

	async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError> {
		match &self.options.streaming_func {
			Some(func) => {
				let mut complete_response = String::new();
				let mut stream = self.stream(messages).await?;
				while let Some(data) = stream.next().await {
					match data {
						Ok(value) => {
							let mut func = func.lock().await;
							complete_response.push_str(&value.content);
							let _ = func(value.content).await;
						},
						Err(e) => return Err(e),
					}
				}
				let mut generate_result = GenerateResult::default();
				generate_result.generation = complete_response;
				Ok(generate_result)
			},
			None => self.generate(messages).await,
		}
	}
}

fn parse_sse_to_json(sse_data: &str) -> LLMResult<Value> {
	if let Ok(json) = serde_json::from_str::<Value>(sse_data) {
		return parse_error(&json);
	}

	let lines: Vec<&str> = sse_data.trim().split('\n').collect();
	let mut event_data: HashMap<&str, String> = HashMap::new();

	for line in lines {
		if let Some((key, value)) = line.split_once(": ") {
			event_data.insert(key, value.to_string());
		}
	}

	if let Some(data) = event_data.get("data") {
		let data: Value = serde_json::from_str(data)?;
		return match data["type"].as_str() {
			Some("error") => parse_error(&data),
			_ => Ok(data),
		};
	}
	log::error!("No data field in the SSE event");
	Err(LLMError::new(
		LLMErrorKind::AnthropicError("No data field in the SSE event".to_string()),
		Arc::new(anyhow::anyhow!("No data field in the SSE event").into()),
	))
}

fn parse_error(json: &Value) -> LLMResult<Value> {
	let error_type = json["error"]["type"].as_str().unwrap_or("");
	let message = json["error"]["message"].as_str().unwrap_or("").to_string();
	match error_type {
		"invalid_request_error" => Err(LLMError::new(
			LLMErrorKind::AnthropicError(message.clone()),
			Arc::new(anyhow::anyhow!(message).into()),
		)),
		"authentication_error" => Err(LLMError::new(
			LLMErrorKind::AnthropicError(message.clone()),
			Arc::new(anyhow::anyhow!(message).into()),
		)),
		"permission_error" => Err(LLMError::new(
			LLMErrorKind::AnthropicError(message.clone()),
			Arc::new(anyhow::anyhow!(message).into()),
		)),
		"not_found_error" => Err(LLMError::new(
			LLMErrorKind::AnthropicError(message.clone()),
			Arc::new(anyhow::anyhow!(message).into()),
		)),
		"rate_limit_error" => Err(LLMError::new(
			LLMErrorKind::AnthropicError(message.clone()),
			Arc::new(anyhow::anyhow!(message).into()),
		)),
		"api_error" => Err(LLMError::new(
			LLMErrorKind::AnthropicError(message.clone()),
			Arc::new(anyhow::anyhow!(message).into()),
		)),
		"overloaded_error" => Err(LLMError::new(
			LLMErrorKind::AnthropicError(message.clone()),
			Arc::new(anyhow::anyhow!(message).into()),
		)),
		_ => Err(LLMError::new(
			LLMErrorKind::AnthropicError(message.clone()),
			Arc::new(anyhow::anyhow!(message).into()),
		)),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tokio::test;

	#[test]
	#[ignore]
	async fn test_cloudia_generate() {
		let cloudia = Claude::new();

		let res = cloudia
			.generate(&[Message::new_human_message("Hi, how are you doing")])
			.await
			.unwrap();

		println!("{:?}", res)
	}

	#[test]
	#[ignore]
	async fn test_cloudia_stream() {
		let cloudia = Claude::new();
		let mut stream = cloudia
			.stream(&[Message::new_human_message("Hi, how are you doing")])
			.await
			.unwrap();
		while let Some(data) = stream.next().await {
			match data {
				Ok(value) => value.to_stdout().unwrap(),
				Err(e) => panic!("Error invoking LLMChain: {:?}", e),
			}
		}
	}
}
