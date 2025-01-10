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

use std::sync::Arc;

pub use async_openai::config::{AzureConfig, Config, OpenAIConfig};
use async_openai::{
	types::{
		ChatChoiceStream, ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessageArgs,
		ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
		ChatCompletionRequestToolMessageArgs, ChatCompletionRequestUserMessageArgs,
		ChatCompletionToolArgs, ChatCompletionToolType, CreateChatCompletionRequest,
		CreateChatCompletionRequestArgs, FunctionObjectArgs,
	},
	Client,
};
use async_trait::async_trait;
use candle_core::Tensor;
use futures::StreamExt;

use crate::{
	CallOptions, FunctionCallBehavior, GenerateResult, LLMError, LLMErrorKind, LLMResult, Message,
	MessageType, TokenUsage, LLM,
};

#[derive(Clone)]
pub enum OpenAIModel {
	Gpt35,
	Gpt4,
	Gpt4Turbo,
	Gpt4o,
}

impl ToString for OpenAIModel {
	fn to_string(&self) -> String {
		match self {
			OpenAIModel::Gpt35 => "gpt-3.5-turbo".to_string(),
			OpenAIModel::Gpt4 => "gpt-4".to_string(),
			OpenAIModel::Gpt4Turbo => "gpt-4-turbo-preview".to_string(),
			OpenAIModel::Gpt4o => "gpt-4o".to_string(),
		}
	}
}

impl Into<String> for OpenAIModel {
	fn into(self) -> String {
		self.to_string()
	}
}

#[derive(Clone)]
pub struct OpenAI<C: Config> {
	config: C,
	options: CallOptions,
	model: String,
}

impl<C: Config> OpenAI<C> {
	pub fn new(config: C) -> Self {
		Self { config, options: CallOptions::default(), model: OpenAIModel::Gpt35.to_string() }
	}

	pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
		self.model = model.into();
		self
	}

	pub fn with_config(mut self, config: C) -> Self {
		self.config = config;
		self
	}

	pub fn with_options(mut self, options: CallOptions) -> Self {
		self.options = options;
		self
	}
}

impl Default for OpenAI<OpenAIConfig> {
	fn default() -> Self {
		Self::new(OpenAIConfig::default())
	}
}

#[async_trait]
impl<C: Config + Send + Sync + 'static> LLM for OpenAI<C> {
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

	async fn generate(&self, prompt: &[Message]) -> LLMResult<GenerateResult> {
		let client = Client::with_config(self.config.clone());
		let request = self.generate_request(prompt)?;
		match &self.options.streaming_func {
			Some(func) => {
				let mut stream = client.chat().create_stream(request).await?;
				let mut complete_response = String::new();
				while let Some(result) = stream.next().await {
					match result {
						Ok(response) =>
							for chat_choice in response.choices.iter() {
								let chat_choice: ChatChoiceStream = chat_choice.clone();
								{
									let mut func = func.lock().await;
									let _ = func(
										serde_json::to_string(&chat_choice).unwrap_or("".into()),
									)
									.await;
								}
								if let Some(content) = chat_choice.delta.content {
									complete_response.push_str(&content);
								}
							},
						Err(err) => {
							eprintln!("Error from streaming response: {:?}", err);
						},
					}
				}
				let mut generate_result = GenerateResult::default();
				generate_result.generation = complete_response;
				Ok(generate_result)
			},
			None => {
				let response = client.chat().create(request).await?;
				let mut generate_result = GenerateResult::default();

				if let Some(usage) = response.usage {
					generate_result.tokens = Some(TokenUsage {
						prompt_tokens: usage.prompt_tokens,
						completion_tokens: usage.completion_tokens,
						total_tokens: usage.total_tokens,
					});
				}

				if let Some(choice) = &response.choices.first() {
					generate_result.generation = choice.message.content.clone().unwrap_or_default();
					if let Some(function) = &choice.message.tool_calls {
						generate_result.generation =
							serde_json::to_string(&function).unwrap_or_default();
					}
				} else {
					generate_result.generation = "".to_string();
				}

				Ok(generate_result)
			},
		}
	}
}

impl<C: Config> OpenAI<C> {
	fn to_openai_messages(
		&self,
		messages: &[Message],
	) -> LLMResult<Vec<ChatCompletionRequestMessage>> {
		let mut openai_messages: Vec<ChatCompletionRequestMessage> = Vec::new();
		for m in messages {
			match m.message_type {
				MessageType::AIMessage => openai_messages.push(match &m.tool_calls {
					Some(value) => {
						let function: Vec<ChatCompletionMessageToolCall> =
							serde_json::from_value(value.clone())?;
						ChatCompletionRequestAssistantMessageArgs::default()
							.tool_calls(function)
							.content(m.content.clone())
							.build()?
							.into()
					},
					None => ChatCompletionRequestAssistantMessageArgs::default()
						.content(m.content.clone())
						.build()?
						.into(),
				}),
				MessageType::HumanMessage => openai_messages.push(
					ChatCompletionRequestUserMessageArgs::default()
						.content(m.content.clone())
						.build()?
						.into(),
				),
				MessageType::SystemMessage => openai_messages.push(
					ChatCompletionRequestSystemMessageArgs::default()
						.content(m.content.clone())
						.build()?
						.into(),
				),
				MessageType::ToolMessage => {
					openai_messages.push(
						ChatCompletionRequestToolMessageArgs::default()
							.content(m.content.clone())
							.tool_call_id(m.id.clone().unwrap_or_default())
							.build()?
							.into(),
					);
				},
			}
		}
		Ok(openai_messages)
	}

	fn generate_request(&self, messages: &[Message]) -> LLMResult<CreateChatCompletionRequest> {
		let messages: Vec<ChatCompletionRequestMessage> = self.to_openai_messages(messages)?;
		let mut request_builder = CreateChatCompletionRequestArgs::default();
		if let Some(max_tokens) = self.options.max_tokens {
			request_builder.max_tokens(max_tokens as u16);
		}
		request_builder.model(self.model.to_string());
		if let Some(stop_words) = &self.options.stop_words {
			request_builder.stop(stop_words);
		}

		if let Some(behavior) = &self.options.functions {
			let mut functions = Vec::new();
			for f in behavior.iter() {
				let tool = FunctionObjectArgs::default()
					.name(f.name.clone())
					.description(f.description.clone())
					.parameters(f.parameters.clone())
					.build()?;
				functions.push(
					ChatCompletionToolArgs::default()
						.r#type(ChatCompletionToolType::Function)
						.function(tool)
						.build()?,
				)
			}
			request_builder.tools(functions);
		}

		if let Some(behavior) = self.options.function_call_behavior {
			match behavior {
				FunctionCallBehavior::Auto => request_builder.function_call("auto"),
				FunctionCallBehavior::None => request_builder.function_call("none"),
			};
		}
		request_builder.messages(messages);
		Ok(request_builder.build()?)
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use std::sync::Arc;
	use tokio::{sync::Mutex, test};

	#[test]
	#[ignore]
	async fn test_invoke() {
		let message_complete = Arc::new(Mutex::new(String::new()));

		// Define the streaming function
		// This function will append the content received from the stream to `message_complete`
		let streaming_func = {
			let message_complete = message_complete.clone();
			move |content: String| {
				let message_complete = message_complete.clone();
				async move {
					let mut message_complete_lock = message_complete.lock().await;
					println!("Content: {:?}", content);
					message_complete_lock.push_str(&content);
					Ok(())
				}
			}
		};
		let options = CallOptions::new().with_streaming_func(streaming_func);
		// Setup the OpenAI client with the necessary options
		let open_ai = OpenAI::new(OpenAIConfig::default())
			.with_model(OpenAIModel::Gpt35.to_string()) // You can change the model as needed
			.with_options(options);

		// Define a set of messages to send to the generate function

		// Call the generate function
		match open_ai.invoke("hola").await {
			Ok(result) => {
				// Print the response from the generate function
				println!("Generate Result: {:?}", result);
				println!("Message Complete: {:?}", message_complete.lock().await);
			},
			Err(e) => {
				// Handle any errors
				eprintln!("Error calling generate: {:?}", e);
			},
		}
	}
}
