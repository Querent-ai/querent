use candle_core::{quantized::gguf_file, Device, Tensor};
use candle_transformers::{
	generation::LogitsProcessor, models::quantized_llama::ModelWeights, utils::apply_repeat_penalty,
};
use hf_hub::{
	api::sync::{Api, ApiRepo},
	Repo, RepoType,
};
use rand::SeedableRng;
use std::{
	convert::Infallible,
	fmt::{Display, Formatter},
	path::PathBuf,
	pin::Pin,
};
use tokenizers::{AddedToken, PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};
use tracing::info;

use async_trait::async_trait;
use fastembed::{read_file_to_bytes, EmbeddingModel, InitOptions, TextEmbedding, TokenizerFiles};
use futures::Stream;
use tokio::{sync::Mutex, time::Instant};

use crate::{
	language_models::{llm::LLM, options::CallOptions, GenerateResult, LLMError, TokenUsage},
	schemas::{messages::Message, StreamData},
};

use super::token_output_stream::TokenOutputStream;

pub struct LLama {
	options: CallOptions,
	model: ModelWeights,
	tokenizer: Tokenizer,
}

const TOKENIZER_REPO: &str = "Qdrant/all-MiniLM-L6-v2-onnx";
const TOKENIZER: &str = "tokenizer.json";

fn build_repo(repo: &str) -> Result<ApiRepo, LLMError> {
	let api = Api::new().map_err(|e| LLMError::OtherError(e.to_string()))?;
	Ok(api.repo(Repo::with_revision(repo.into(), RepoType::Model, "main".into())))
}

#[derive(Debug)]
pub struct TokenizerFile(PathBuf);
impl TokenizerFile {
	pub fn download() -> Result<TokenizerFile, LLMError> {
		let repo = build_repo(TOKENIZER_REPO)?;
		let filename = repo.get(TOKENIZER).map_err(|e| LLMError::OtherError(e.to_string()))?;
		Ok(Self(filename))
	}

	pub fn tokenizer(self) -> Result<Tokenizer, LLMError> {
		Tokenizer::from_file(self.0).map_err(|e| LLMError::OtherError(e.to_string()))
	}
}

impl Display for TokenizerFile {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{:?}", self.0)
	}
}

fn load_gguf_model_weights(model_weights_path: String) -> Result<ModelWeights, LLMError> {
	let mut file: std::fs::File =
		std::fs::File::open(model_weights_path.clone()).map_err(|_| {
			LLMError::OtherError(format!(
				"Failed to open model weights file: {}",
				model_weights_path
			))
		})?;

	let model_content = gguf_file::Content::read(&mut file)
		.map_err(|_| LLMError::OtherError("Failed to read model weights".into()))?;
	info!("loaded {:?} tensors", model_content.tensor_infos.len(),);
	let model = ModelWeights::from_gguf(model_content, &mut file, &candle_core::Device::Cpu)
		.map_err(|e| LLMError::OtherError(format!("Failed to load model weights: {}", e)))?;
	Ok(model)
}

impl LLama {
	pub async fn try_new(model_path: PathBuf) -> Self {
		let model = load_gguf_model_weights(model_path.to_str().unwrap().to_string())
			.expect("Failed to load model weights");
		let tokenizer = TokenizerFile::download().expect("Failed to download tokenizer file");
		Self {
			options: Default::default(),
			model,
			tokenizer: tokenizer.tokenizer().expect("Failed to load tokenizer"),
		}
	}

	pub fn with_options(mut self, options: CallOptions) -> Self {
		self.options = options;
		self
	}
}

#[async_trait]
impl LLM for LLama {
	async fn generate(&self, prompt: &[Message]) -> Result<GenerateResult, LLMError> {
		let prompt_text =
			prompt.iter().map(|m| m.content.clone()).collect::<Vec<String>>().join(" ");

		let mut tos: TokenOutputStream = TokenOutputStream::new(self.tokenizer.clone());
		let mut tokens = tos
			.tokenizer()
			.encode(prompt_text, true)
			.map_err(|e| LLMError::OtherError(e.to_string()))?;
		let prompt_tokens = tokens.get_ids().to_vec();
		let eos: u32 = *self
			.tokenizer
			.get_vocab(true)
			.get("</s>")
			.ok_or_else(|| LLMError::OtherError("Failed to get </s> token".into()))?;
		// Prepare the device and logits processor
		let device = Device::Cpu;
		let mut logits_processor = LogitsProcessor::new(10074, Some(0.8), None);
		let mut generated_tokens = 0;

		// Timing the generation process
		let start = Instant::now();

		Err(LLMError::OtherError("Not implemented".into()))
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
		println!("{:?}", result);
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
