use rand::SeedableRng;
use std::{convert::Infallible, path::PathBuf, pin::Pin};

use async_trait::async_trait;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use futures::Stream;
use tokio::sync::Mutex;

use llm::{
	InferenceFeedback, InferenceParameters, InferenceRequest, InferenceResponse, Model,
	ModelArchitecture,
};

use crate::{
	language_models::{llm::LLM, options::CallOptions, GenerateResult, LLMError, TokenUsage},
	schemas::{messages::Message, StreamData},
};

pub struct TransformersLLM {
	options: CallOptions,
	model: Box<dyn Model>,
	user_name: String,
	assistant_name: String,
	session: Mutex<llm::InferenceSession>,
}

impl TransformersLLM {
	pub fn new(model_architecture: ModelArchitecture, model_path: PathBuf) -> Self {
		let embedder_options: InitOptions = InitOptions {
			model_name: EmbeddingModel::AllMiniLML6V2,
			show_download_progress: true,
			..Default::default()
		};

		let embedding_model = TextEmbedding::try_new(embedder_options.clone())
			.expect("Failed to initialize TextEmbedding");
		let token_model_repo = embedder_options.clone().cache_dir;
		let token_model_info = TextEmbedding::get_model_info(&embedder_options.model_name);
		let token_model_path = token_model_repo.join(&token_model_info.model_file);
		let tokenizer_source = llm::TokenizerSource::HuggingFaceTokenizerFile(token_model_path);
		drop(embedding_model);
		let model = llm::load_dynamic(
			Some(model_architecture),
			&model_path,
			tokenizer_source.clone(),
			Default::default(),
			llm::load_progress_callback_stdout,
		)
		.expect("Failed to load model");

		let mut session = model.start_session(Default::default());

		let user_name = "### Human";
		let assistant_name = "### Assistant";
		let persona = "A chat between a human and an assistant.";
		let history = String::new();
		session
			.feed_prompt(
				model.as_ref(),
				format!("{persona}\n{history}").as_str(),
				&mut Default::default(),
				llm::feed_prompt_callback(|resp| match resp {
					InferenceResponse::PromptToken(_t) | InferenceResponse::InferredToken(_t) =>
						Ok::<llm::InferenceFeedback, Infallible>(llm::InferenceFeedback::Continue),
					_ => Ok(llm::InferenceFeedback::Continue),
				}),
			)
			.map_err(|e| LLMError::OtherError(e.to_string()))
			.expect("Failed to feed prompt");

		Self {
			options: CallOptions::default(),
			model,
			user_name: user_name.to_string(),
			assistant_name: assistant_name.to_string(),
			session: Mutex::new(session),
		}
	}

	pub fn with_options(mut self, options: CallOptions) -> Self {
		self.options = options;
		self
	}
}

#[async_trait]
impl LLM for TransformersLLM {
	async fn generate(&self, prompt: &[Message]) -> Result<GenerateResult, LLMError> {
		let mut rng = rand::rngs::StdRng::from_entropy();
		let inference_parameters = InferenceParameters::default();
		let user_name = self.user_name.clone();
		let assistant_name = self.assistant_name.clone();
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
						prompt: format!("{user_name}: {prompt_text}\n{assistant_name}:")
							.as_str()
							.into(),
						parameters: &inference_parameters,
						play_back_previous_tokens: false,
						maximum_token_count: None,
					},
					&mut Default::default(),
					|r| {
						match r {
							InferenceResponse::InferredToken(t) => text.push_str(&t),
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
