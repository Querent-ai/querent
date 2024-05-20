use rand::SeedableRng;
use std::{convert::Infallible, path::PathBuf, pin::Pin};

use async_trait::async_trait;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use futures::Stream;
use tokio::sync::Mutex;

use llm::{InferenceFeedback, InferenceParameters, InferenceRequest, InferenceResponse, Model};

use crate::{
	language_models::{llm::LLM, options::CallOptions, GenerateResult, LLMError, TokenUsage},
	schemas::{messages::Message, StreamData},
};

pub struct LLama {
	options: CallOptions,
	model: Box<dyn Model>,
	session: Mutex<llm::InferenceSession>,
}

impl LLama {
	pub async fn try_new(model_path: PathBuf) -> Self {
		let embedder_options = InitOptions {
			model_name: EmbeddingModel::AllMiniLML6V2,
			show_download_progress: true,
			..Default::default()
		};

		let embedding_model = TextEmbedding::try_new(embedder_options.clone())
			.expect("Failed to initialize TextEmbedding");
		let token_model_repo = embedder_options.clone().cache_dir;
		let token_model_info = TextEmbedding::get_model_info(&embedder_options.model_name);
		let token_model_path: PathBuf =
			token_model_repo.join("models--Qdrant--all-MiniLM-L6-v2-onnx/tokenizer.json");
		let tokenizer_source = llm::TokenizerSource::HuggingFaceTokenizerFile(PathBuf::from("/home/querent/querent/quester/quester/querent/.fastembed_cache/models--Qdrant--all-MiniLM-L6-v2-onnx/snapshots/cb27c1f221563ca44e58ad2a694965db39c8153d/tokenizer.json"));
		drop(embedding_model);
		let model = llm::load(
			model_path.as_path(),
			tokenizer_source.clone(),
			Default::default(),
			llm::load_progress_callback_stdout,
		)
		.expect("Failed to load model");

		let session = model.start_session(Default::default());
		Self { options: CallOptions::default(), model, session: Mutex::new(session) }
	}

	pub fn with_options(mut self, options: CallOptions) -> Self {
		self.options = options;
		self
	}
}

#[async_trait]
impl LLM for LLama {
	async fn generate(&self, prompt: &[Message]) -> Result<GenerateResult, LLMError> {
		let mut rng = rand::rngs::StdRng::from_entropy();
		let inference_parameters = InferenceParameters::default();
		let prompt_text =
			prompt.iter().map(|m| m.content.clone()).collect::<Vec<String>>().join("\n");
		let mut text = String::new();
		let response = {
			let mut session = self.session.lock().await;
			let res = session
				.infer::<Infallible>(
					self.model.as_ref(),
					&mut rng,
					&InferenceRequest {
						prompt: format!("{prompt_text}").as_str().into(),
						parameters: &inference_parameters,
						play_back_previous_tokens: false,
						maximum_token_count: None,
					},
					&mut Default::default(),
					|r| {
						match r {
							InferenceResponse::InferredToken(t) => {
								println!("{}", t);
								text.push_str(&t)
							},
							InferenceResponse::EotToken => return Ok(InferenceFeedback::Halt),
							_ => {},
						};
						Ok(InferenceFeedback::Continue)
					},
				)
				.map_err(|e| LLMError::OtherError(e.to_string()))?;
			res
		};

		let generation = text.trim().to_string();
		let tokens = Some(TokenUsage {
			prompt_tokens: response.prompt_tokens as u32,
			completion_tokens: response.predict_tokens as u32,
			total_tokens: (response.prompt_tokens + response.predict_tokens) as u32,
		});

		Ok(GenerateResult { tokens, generation })
	}

	async fn invoke(&self, prompt: &str) -> Result<String, LLMError> {
		self.generate(&[Message::new_human_message(prompt)])
			.await
			.map(|res| res.generation)
	}

	async fn stream(
		&self,
		_messages: &[Message],
	) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError> {
		// Implement streaming logic if supported by rustformers crate.
		Err(LLMError::OtherError("Streaming not implemented".into()))
	}

	fn add_options(&mut self, options: CallOptions) {
		self.options.merge_options(options)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	#[tokio::test]
	async fn test_llama_generate() {
		let model_path =
			PathBuf::from("/home/querent/querent/quester/model/llama-2-7b-chat.Q5_K_M.gguf");
		let llama = LLama::try_new(model_path.clone()).await;
		let prompt = vec![Message::new_human_message("Hello, world!")];
		let result = llama.generate(&prompt).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_llama_invoke() {
		let model_path =
			PathBuf::from("/home/querent/querent/quester/model/llama-2-7b-chat.Q5_K_M.gguf");
		let llama = LLama::try_new(model_path).await;
		let prompt = "Hello, world!";
		let result = llama.invoke(prompt).await;
		assert!(result.is_ok());
	}
}
