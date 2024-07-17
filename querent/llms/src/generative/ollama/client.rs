use crate::{
	GenerateResult, LLMError, LLMErrorKind, LLMResult, Message, MessageType, TokenUsage, LLM,
};
use async_trait::async_trait;
use candle_core::Tensor;
pub use ollama_rs::{
	error::OllamaError,
	generation::{
		chat::{request::ChatMessageRequest, ChatMessage, MessageRole},
		options::GenerationOptions,
	},
	Ollama as OllamaClient,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Ollama {
	pub(crate) client: Arc<OllamaClient>,
	pub(crate) model: String,
	pub(crate) options: Option<GenerationOptions>,
}

/// [llama3](https://ollama.com/library/llama3) is a 8B parameters, 4.7GB model.
const DEFAULT_MODEL: &str = "llama3";

impl Ollama {
	pub fn new<S: Into<String>>(
		client: Arc<OllamaClient>,
		model: S,
		options: Option<GenerationOptions>,
	) -> Self {
		Ollama { client, model: model.into(), options }
	}

	pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
		self.model = model.into();
		self
	}

	pub fn with_options(mut self, options: GenerationOptions) -> Self {
		self.options = Some(options);
		self
	}

	fn generate_request(&self, messages: &[Message]) -> ChatMessageRequest {
		let mapped_messages = messages.iter().map(|message| message.into()).collect();
		ChatMessageRequest::new(self.model.clone(), mapped_messages)
	}
}

impl From<&Message> for ChatMessage {
	fn from(message: &Message) -> Self {
		ChatMessage {
			content: message.content.clone(),
			images: None,
			role: message.message_type.clone().into(),
		}
	}
}

impl From<MessageType> for MessageRole {
	fn from(message_type: MessageType) -> Self {
		match message_type {
			MessageType::AIMessage => MessageRole::Assistant,
			MessageType::ToolMessage => MessageRole::Assistant,
			MessageType::SystemMessage => MessageRole::System,
			MessageType::HumanMessage => MessageRole::User,
		}
	}
}

impl Default for Ollama {
	fn default() -> Self {
		let client = Arc::new(OllamaClient::default());
		Ollama::new(client, String::from(DEFAULT_MODEL), None)
	}
}

#[async_trait]
impl LLM for Ollama {
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
		let request = self.generate_request(messages);
		let result = self.client.send_chat_messages(request).await?;

		let generation = match result.message {
			Some(message) => message.content,
			None => return Err(OllamaError::from("No message in response".to_string()).into()),
		};

		let tokens = result.final_data.map(|final_data| {
			let prompt_tokens = final_data.prompt_eval_count as u32;
			let completion_tokens = final_data.eval_count as u32;
			TokenUsage {
				prompt_tokens,
				completion_tokens,
				total_tokens: prompt_tokens + completion_tokens,
			}
		});

		Ok(GenerateResult { tokens, generation })
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	#[ignore]
	async fn test_generate() {
		let ollama = Ollama::default().with_model("llama3");
		let response = ollama.invoke("Hello From Querent").await.unwrap();
		println!("{}", response);
	}
}
