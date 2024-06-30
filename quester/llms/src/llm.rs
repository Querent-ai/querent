use async_openai::error::OpenAIError;
use async_trait::async_trait;
use candle_core::Tensor;
use ollama_rs::error::OllamaError;
use serde::{Deserialize, Serialize};
use std::{fmt, io, sync::Arc};
use thiserror::Error;

use crate::{GenerateResult, Message};

/// Ingestor error kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LLMErrorKind {
	/// Io error.
	Io,
	/// Not found error.
	NotFound,
	/// PyTorch error.
	PyTorch,
	/// Safetensors error.
	SafeTensors,
	/// Model error.
	ModelError,
	// Add more error kinds here if needed
}

/// Generic IngestorError.
#[derive(Debug, Clone, Error)]
#[error("source error(kind={kind:?}, source={source})")]
#[allow(missing_docs)]
pub struct LLMError {
	pub kind: LLMErrorKind,
	#[source]
	pub source: Arc<anyhow::Error>,
}

/// Generic Result type for source type operations.
pub type LLMResult<T> = Result<T, LLMError>;

impl LLMError {
	pub fn new(kind: LLMErrorKind, source: Arc<anyhow::Error>) -> Self {
		LLMError { kind, source }
	}

	/// Add some context to the wrapper error.
	pub fn add_context<C>(self, ctx: C) -> Self
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		LLMError {
			kind: self.kind,
			source: Arc::new(anyhow::anyhow!("{ctx}").context(self.source)),
		}
	}

	/// Returns the corresponding `IngestorErrorKind` for this error.
	pub fn kind(&self) -> LLMErrorKind {
		self.kind
	}
}

impl From<io::Error> for LLMError {
	fn from(err: io::Error) -> LLMError {
		match err.kind() {
			io::ErrorKind::NotFound => LLMError::new(LLMErrorKind::NotFound, Arc::new(err.into())),
			_ => LLMError::new(LLMErrorKind::Io, Arc::new(err.into())),
		}
	}
}

impl From<serde_json::Error> for LLMError {
	fn from(err: serde_json::Error) -> LLMError {
		LLMError::new(LLMErrorKind::Io, Arc::new(err.into()))
	}
}

impl From<OpenAIError> for LLMError {
	fn from(err: OpenAIError) -> LLMError {
		LLMError::new(LLMErrorKind::Io, Arc::new(err.into()))
	}
}

impl From<OllamaError> for LLMError {
	fn from(err: OllamaError) -> LLMError {
		LLMError::new(LLMErrorKind::Io, Arc::new(err.into()))
	}
}

#[async_trait]
pub trait LLM: Send + Sync {
	async fn init_token_idx_2_word_doc_idx(&self) -> Vec<(String, i32)>;
	async fn num_start_tokens(&self) -> usize;
	async fn append_last_token(&self, listing: &mut Vec<(String, i32)>);
	async fn model_input(
		&self,
		tokenized_sequence: Vec<i32>,
	) -> LLMResult<std::collections::HashMap<String, Tensor>>;
	async fn tokenize(&self, word: &str) -> LLMResult<Vec<i32>>;
	async fn inference_attention(
		&self,
		model_input: std::collections::HashMap<String, Tensor>,
	) -> LLMResult<Tensor>;
	async fn maximum_tokens(&self) -> usize;
	async fn tokens_to_words(&self, tokens: &[i32]) -> Vec<String>;
	async fn attention_tensor_to_2d_vector(
		&self,
		attention_weights: &Tensor,
	) -> LLMResult<Vec<Vec<f32>>>;
	async fn token_classification(
		&self,
		model_input: std::collections::HashMap<String, Tensor>,
		labels: Option<&Tensor>,
	) -> LLMResult<Vec<(String, String)>>;
	async fn generate(&self, messages: &[Message]) -> LLMResult<GenerateResult>;
	async fn invoke(&self, prompt: &str) -> Result<String, LLMError> {
		self.generate(&[Message::new_human_message(prompt)])
			.await
			.map(|res| res.generation)
	}
}
