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

use async_openai::error::OpenAIError;
use async_trait::async_trait;
use candle_core::Tensor;
use ollama_rs::error::OllamaError;
use serde::{Deserialize, Serialize};
use std::{fmt, io, sync::Arc};
use thiserror::Error;

use crate::{GenerateResult, Message};

/// Ingestor error kind.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
	// Anthropic error.
	AnthropicError(String),
	// Add more error kinds here if needed
}

/// A generic error type for LLM operations, encapsulating an error kind and its source.
#[derive(Debug, Clone, Error)]
#[error("source error(kind={kind:?}, source={source})")]
#[allow(missing_docs)]
pub struct LLMError {
	/// The kind of error.
	pub kind: LLMErrorKind,
	/// The source of the error.
	#[source]
	pub source: Arc<anyhow::Error>,
}

/// A type alias for results returned by LLM operations.
pub type LLMResult<T> = Result<T, LLMError>;

impl LLMError {
	/// Creates a new `LLMError` with the specified kind and source.
	pub fn new(kind: LLMErrorKind, source: Arc<anyhow::Error>) -> Self {
		LLMError { kind, source }
	}

	/// Adds some context to the existing error.
	pub fn add_context<C>(self, ctx: C) -> Self
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		LLMError {
			kind: self.kind,
			source: Arc::new(anyhow::anyhow!("{ctx}").context(self.source)),
		}
	}

	/// Returns the kind of this error.
	pub fn kind(&self) -> LLMErrorKind {
		self.kind.clone()
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

/// An asynchronous trait defining the operations for a Large Language Model (LLM).
#[async_trait]
pub trait LLM: Send + Sync {
	/// Initializes the token to word mapping for the document.
	async fn init_token_idx_2_word_doc_idx(&self) -> Vec<(String, i32)>;

	/// Returns the number of start tokens.
	async fn num_start_tokens(&self) -> usize;

	/// Appends the last token to the listing.
	async fn append_last_token(&self, listing: &mut Vec<(String, i32)>);

	/// Prepares the model input using a tokenized sequence.
	async fn model_input(
		&self,
		tokenized_sequence: Vec<i32>,
	) -> LLMResult<std::collections::HashMap<String, Tensor>>;

	/// Tokenizes a given word.
	async fn tokenize(&self, word: &str) -> LLMResult<Vec<i32>>;

	/// Performs inference using attention mechanism on the model input.
	async fn inference_attention(
		&self,
		model_input: std::collections::HashMap<String, Tensor>,
	) -> LLMResult<Tensor>;

	/// Returns the maximum number of tokens allowed.
	async fn maximum_tokens(&self) -> usize;

	/// Converts a sequence of tokens to their corresponding words.
	async fn tokens_to_words(&self, tokens: &[i32]) -> Vec<String>;

	/// Converts the attention tensor to a 2D vector.
	async fn attention_tensor_to_2d_vector(
		&self,
		attention_weights: &Tensor,
	) -> LLMResult<Vec<Vec<f32>>>;

	/// Classifies tokens based on the model input and optional labels.
	async fn token_classification(
		&self,
		model_input: std::collections::HashMap<String, Tensor>,
		labels: Option<&Tensor>,
	) -> LLMResult<Vec<(String, String)>>;

	/// Generates a response based on the provided messages.
	async fn generate(&self, messages: &[Message]) -> LLMResult<GenerateResult>;

	/// Invokes the model with a prompt and returns the generated result.
	async fn invoke(&self, prompt: &str) -> Result<String, LLMError> {
		self.generate(&[Message::new_human_message(prompt)])
			.await
			.map(|res| res.generation)
	}
}
