use async_trait::async_trait;
use candle_core::{DType, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::DTYPE;

use candle_transformers::models::bert::{BertModel as CandleBertModel, Config};
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::{collections::HashMap, sync::Arc};
use tokenizers::{PaddingParams, Tokenizer};

use crate::{LLMError, LLMErrorKind, LLMResult, LLM};

use super::DistributionShift;

#[derive(
	Debug, Clone, Copy, Default, Hash, PartialEq, Eq, serde::Deserialize, serde::Serialize,
)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
enum WeightSource {
	#[default]
	Safetensors,
	Pytorch,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct EmbedderOptions {
	pub model: String,
	pub revision: Option<String>,
	pub distribution: Option<DistributionShift>,
}

impl EmbedderOptions {
	pub fn new() -> Self {
		Self { model: "BAAI/bge-base-en-v1.5".to_string(), revision: None, distribution: None }
	}
}

impl Default for EmbedderOptions {
	fn default() -> Self {
		Self::new()
	}
}

/// Perform embedding of documents and queries
pub struct BertLLM {
	model: CandleBertModel,
	tokenizer: Tokenizer,
	options: EmbedderOptions,
	device: candle_core::Device,
}

impl BertLLM {
	pub fn new(options: EmbedderOptions) -> LLMResult<Self> {
		let device = match candle_core::Device::cuda_if_available(0) {
			Ok(device) => device,
			Err(error) => {
				tracing::warn!("could not initialize CUDA device for Hugging Face embedder, defaulting to CPU: {}", error);
				candle_core::Device::Cpu
			},
		};
		let repo = match options.revision.clone() {
			Some(revision) => Repo::with_revision(options.model.clone(), RepoType::Model, revision),
			None => Repo::model(options.model.clone()),
		};
		let (config_filename, tokenizer_filename, weights_filename, weight_source) = {
			let api = Api::new().map_err(|e| {
				LLMError::new(
					LLMErrorKind::Io,
					Arc::new(anyhow::anyhow!("could not initialize Hugging Face API: {}", e)),
				)
			})?;

			let api = api.repo(repo);
			let config = api.get("config.json").map_err(|e| {
				LLMError::new(
					LLMErrorKind::Io,
					Arc::new(anyhow::anyhow!("could not fetch config.json: {}", e)),
				)
			})?;
			let tokenizer = api.get("tokenizer.json").map_err(|e| {
				LLMError::new(
					LLMErrorKind::Io,
					Arc::new(anyhow::anyhow!("could not fetch tokenizer.json: {}", e)),
				)
			})?;
			let (weights, source) = {
				api.get("model.safetensors")
					.map(|filename| (filename, WeightSource::Safetensors))
					.or_else(|_| {
						api.get("pytorch_model.bin")
							.map(|filename| (filename, WeightSource::Pytorch))
					})
					.map_err(|e| {
						LLMError::new(
							LLMErrorKind::Io,
							Arc::new(anyhow::anyhow!("could not fetch model weights: {}", e)),
						)
					})?
			};
			(config, tokenizer, weights, source)
		};

		let config = std::fs::read_to_string(&config_filename).map_err(|inner| {
			LLMError::new(
				LLMErrorKind::Io,
				Arc::new(anyhow::anyhow!("could not read config.json: {}", inner)),
			)
		})?;
		let config: Config = serde_json::from_str(&config).map_err(|inner| {
			LLMError::new(
				LLMErrorKind::Io,
				Arc::new(anyhow::anyhow!("could not parse config.json: {}", inner)),
			)
		})?;
		let mut tokenizer = Tokenizer::from_file(&tokenizer_filename).map_err(|inner| {
			LLMError::new(
				LLMErrorKind::Io,
				Arc::new(anyhow::anyhow!("could not read tokenizer.json: {}", inner)),
			)
		})?;

		let vb = match weight_source {
			WeightSource::Pytorch => VarBuilder::from_pth(&weights_filename, DTYPE, &device)
				.map_err(|e| {
					LLMError::new(
						LLMErrorKind::PyTorch,
						Arc::new(anyhow::anyhow!("could not load PyTorch weights: {}", e)),
					)
				})?,
			WeightSource::Safetensors => unsafe {
				VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device).map_err(
					|e| {
						LLMError::new(
							LLMErrorKind::SafeTensors,
							Arc::new(anyhow::anyhow!("could not load SafeTensors weights: {}", e)),
						)
					},
				)?
			},
		};

		let model = CandleBertModel::load(vb, &config).map_err(|e| {
			LLMError::new(
				LLMErrorKind::PyTorch,
				Arc::new(anyhow::anyhow!("could not load model: {}", e)),
			)
		})?;

		if let Some(pp) = tokenizer.get_padding_mut() {
			pp.strategy = tokenizers::PaddingStrategy::BatchLongest
		} else {
			let pp = PaddingParams {
				strategy: tokenizers::PaddingStrategy::BatchLongest,
				..Default::default()
			};
			tokenizer.with_padding(Some(pp));
		}

		let this = Self { model, tokenizer, options, device };

		Ok(this)
	}
}

#[async_trait]
impl LLM for BertLLM {
	async fn init_token_idx_2_word_doc_idx(&self) -> Vec<(String, i32)> {
		vec![("CLS".to_string(), -1)]
	}

	async fn num_start_tokens(&self) -> usize {
		1
	}

	async fn append_last_token(&self, listing: &mut Vec<(String, i32)>) {
		listing.push(("SEP".to_string(), listing.len() as i32));
	}

	async fn model_input(&self, tokenized_sequence: Vec<i32>) -> HashMap<String, Tensor> {
		let tokenized_sequence = vec![
			self.tokenizer.cls_token_id().unwrap(),
			tokenized_sequence,
			self.tokenizer.sep_token_id().unwrap(),
		]
		.concat();

		let input_ids = Tensor::new(tokenized_sequence.as_slice(), &self.device)
			.unwrap()
			.unsqueeze(0)
			.unwrap();
		let token_type_ids = Tensor::zeros(input_ids.dims(), DType::I64, &self.device).unwrap();
		let attention_mask = Tensor::ones(input_ids.dims(), DType::I64, &self.device).unwrap();

		let mut input_map = HashMap::new();
		input_map.insert("input_ids".to_string(), input_ids);
		input_map.insert("token_type_ids".to_string(), token_type_ids);
		input_map.insert("attention_mask".to_string(), attention_mask);

		input_map
	}

	async fn tokenize(&self, word: &str) -> Vec<i32> {
		self.tokenizer
			.encode(word, false)
			.unwrap()
			.get_ids()
			.iter()
			.map(|&id| id as i32)
			.collect()
	}

	async fn inference_attention(&self, model_input: HashMap<String, Tensor>) -> Vec<Vec<f32>> {
		let input_ids = model_input.get("input_ids").unwrap();
		let token_type_ids = model_input.get("token_type_ids").unwrap();
		let attention_mask = model_input.get("attention_mask").unwrap();

		let output = self.model.forward(input_ids, token_type_ids).unwrap();
		let attentions = output.attentions.unwrap();
		let last_attention_layer = attentions.last().unwrap();
		let mean_attention = last_attention_layer.mean_dim(1);

		mean_attention.to_vec()
	}

	async fn maximum_tokens(&self) -> usize {
		512
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_bert_llm() {
		let options = EmbedderOptions {
			model: "BAAI/bge-base-en-v1.5".to_string(),
			revision: None,
			distribution: None,
		};
		let embedder = BertLLM::new(options).await.unwrap();
		let tokens = embedder.tokenize("hello").await;
		assert_eq!(tokens, vec![101, 7592, 102]);
	}
}
