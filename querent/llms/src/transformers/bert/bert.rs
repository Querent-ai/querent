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
	transformers::bert::bert_model_functions::{BertConfig, BertModel as CandleBertModel, DTYPE},
	GenerateResult, LLMError, LLMErrorKind, LLMResult, Message, LLM,
};
use async_trait::async_trait;
use candle_core::Tensor;
use candle_nn::VarBuilder;
use hf_hub::{api::sync::ApiBuilder, Repo, RepoType};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokenizers::{PaddingParams, Tokenizer};

use super::BertForTokenClassification;
use crate::transformers::DistributionShift;
use common::get_querent_data_path;

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
	pub local_dir: Option<String>,
}

impl EmbedderOptions {
	pub fn new() -> Self {
		Self {
			model: "BAAI/bge-base-en-v1.5".to_string(),
			revision: None,
			distribution: None,
			local_dir: None,
		}
	}
}

impl Default for EmbedderOptions {
	fn default() -> Self {
		Self::new()
	}
}

/// Perform embedding of documents and queries
pub struct BertLLM {
	model: Option<CandleBertModel>,
	token_classification_model: Option<BertForTokenClassification>,
	tokenizer: Tokenizer,
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

		let (config_filename, tokenizer_filename, weights_filename, weight_source) =
			if let Some(local_dir) = &options.local_dir {
				// Read from local directory
				let config_filename = PathBuf::from(format!("{}/config.json", local_dir));
				let tokenizer_filename = PathBuf::from(format!("{}/tokenizer.json", local_dir));
				let (weights_filename, weight_source) = {
					let safetensors_path =
						PathBuf::from(format!("{}/model.safetensors", local_dir));
					let pytorch_path = PathBuf::from(format!("{}/pytorch_model.bin", local_dir));

					if safetensors_path.exists() {
						(safetensors_path, WeightSource::Safetensors)
					} else if pytorch_path.exists() {
						(pytorch_path, WeightSource::Pytorch)
					} else {
						return Err(LLMError::new(
							LLMErrorKind::Io,
							Arc::new(anyhow::anyhow!(
								"could not find model weights in local directory"
							)),
						));
					}
				};
				(config_filename, tokenizer_filename, weights_filename, weight_source)
			} else {
				// Fetch from Hugging Face API
				let repo = match &options.revision {
					Some(revision) =>
						Repo::with_revision(options.model, RepoType::Model, revision.to_string()),
					None => Repo::model(options.model),
				};
				let cache_dir = get_querent_data_path();
				let api = ApiBuilder::new().with_cache_dir(cache_dir).build().map_err(|e| {
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
						.map(|filename| (PathBuf::from(filename), WeightSource::Safetensors))
						.or_else(|_| {
							api.get("pytorch_model.bin")
								.map(|filename| (PathBuf::from(filename), WeightSource::Pytorch))
						})
						.map_err(|e| {
							LLMError::new(
								LLMErrorKind::Io,
								Arc::new(anyhow::anyhow!("could not fetch model weights: {}", e)),
							)
						})?
				};
				(PathBuf::from(config), PathBuf::from(tokenizer), weights, source)
			};

		let config = std::fs::read_to_string(&config_filename).map_err(|inner| {
			LLMError::new(
				LLMErrorKind::Io,
				Arc::new(anyhow::anyhow!("could not read config.json: {}", inner)),
			)
		})?;
		let config: BertConfig = serde_json::from_str(&config).map_err(|inner| {
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

		let model = if config.id2label.is_none() && config.label2id.is_none() {
			Some(CandleBertModel::load(vb.clone(), &config).map_err(|e| {
				LLMError::new(
					LLMErrorKind::PyTorch,
					Arc::new(anyhow::anyhow!("could not load model: {}", e)),
				)
			})?)
		} else {
			None
		};
		let token_classification_model = if config._num_labels.is_some() ||
			config.id2label.is_some()
		{
			Some(BertForTokenClassification::load(vb, &config).map_err(|e| {
				LLMError::new(
					LLMErrorKind::PyTorch,
					Arc::new(anyhow::anyhow!("could not load token classification model: {}", e)),
				)
			})?)
		} else {
			None
		};

		if let Some(pp) = tokenizer.get_padding_mut() {
			pp.strategy = tokenizers::PaddingStrategy::BatchLongest
		} else {
			let pp = PaddingParams {
				strategy: tokenizers::PaddingStrategy::BatchLongest,
				..Default::default()
			};
			tokenizer.with_padding(Some(pp));
		}

		let this = Self { model, token_classification_model, tokenizer, device };

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

	async fn model_input(
		&self,
		tokenized_sequence: Vec<i32>,
	) -> Result<HashMap<String, Tensor>, LLMError> {
		let cls_token_id = 101; // [CLS]
		let sep_token_id = 102; // [SEP]

		let tokenized_sequence =
			vec![vec![cls_token_id], tokenized_sequence, vec![sep_token_id]].concat();
		let tokenized_sequence_u32: Vec<u32> =
			tokenized_sequence.iter().map(|&x| x as u32).collect();
		let tokenized_sequence_slice: &[u32] = &tokenized_sequence_u32;
		let input_ids = Tensor::new(tokenized_sequence_slice, &self.device)
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?
			.reshape((1, tokenized_sequence_u32.len()))
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?;
		let token_type_ids = input_ids
			.zeros_like()
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?;
		let attention_mask = input_ids
			.ones_like()
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?;
		let mut input_map = HashMap::new();
		input_map.insert("input_ids".to_string(), input_ids);
		input_map.insert("token_type_ids".to_string(), token_type_ids);
		input_map.insert("attention_mask".to_string(), attention_mask);

		Ok(input_map)
	}

	async fn tokenize(&self, word: &str) -> Result<Vec<i32>, LLMError> {
		self.tokenizer
			.encode(word, false)
			.map_err(|e| {
				LLMError::new(
					LLMErrorKind::ModelError,
					Arc::new(anyhow::anyhow!("token encoding failed: {}", e)),
				)
			})
			.map(|encoding| encoding.get_ids().iter().map(|&id| id as i32).collect())
	}

	async fn inference_attention(
		&self,
		model_input: HashMap<String, Tensor>,
	) -> Result<Tensor, LLMError> {
		let input_ids = model_input.get("input_ids").ok_or_else(|| {
			LLMError::new(
				LLMErrorKind::ModelError,
				Arc::new(anyhow::anyhow!("missing input_ids in model input")),
			)
		})?;

		let token_type_ids = model_input.get("token_type_ids").ok_or_else(|| {
			LLMError::new(
				LLMErrorKind::ModelError,
				Arc::new(anyhow::anyhow!("missing token_type_ids in model input")),
			)
		})?;

		let model = self.model.as_ref().ok_or_else(|| {
			LLMError::new(
				LLMErrorKind::ModelError,
				Arc::new(anyhow::anyhow!("model is not initialized")),
			)
		})?;

		let _ = model.forward(input_ids, token_type_ids).map_err(|e| {
			LLMError::new(
				LLMErrorKind::ModelError,
				Arc::new(anyhow::anyhow!("model forward pass failed: {}", e)),
			)
		})?;

		if let Some(attention_probs) = model.get_last_attention_probs() {
			let mean_attention_probs = attention_probs.mean(1).map_err(|e| {
				LLMError::new(
					LLMErrorKind::ModelError,
					Arc::new(anyhow::anyhow!("failed to calculate mean attention probs: {}", e)),
				)
			})?;

			Ok(mean_attention_probs)
		} else {
			Err(LLMError::new(
				LLMErrorKind::ModelError,
				Arc::new(anyhow::anyhow!("could not retrieve attention weights")),
			))
		}
	}

	async fn maximum_tokens(&self) -> usize {
		255
	}

	async fn tokens_to_words(&self, tokens: &[i32]) -> Vec<String> {
		let words = tokens
			.iter()
			.map(|&token| {
				let token_u32 = token as u32;
				match self.tokenizer.decode(&[token_u32], false) {
					Ok(word) => word,
					Err(e) => {
						log::error!("Failed to decode token {}: {:?}", token, e);
						"[UNK]".to_string()
					},
				}
			})
			.collect::<Vec<String>>();
		words
	}

	async fn attention_tensor_to_2d_vector(
		&self,
		attention_weights: &Tensor,
	) -> Result<Vec<Vec<f32>>, LLMError> {
		let attention_weights_2d = attention_weights
			.squeeze(0)
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?
			.to_vec2::<f32>()
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?;

		// Remove first and last rows and columns (corresponding to [CLS] and [SEP])
		let trimmed_attention_weights: Vec<Vec<f32>> = attention_weights_2d
			[1..attention_weights_2d.len() - 1]
			.iter()
			.map(|row| row[1..row.len() - 1].to_vec())
			.collect();
		Ok(trimmed_attention_weights)
	}

	async fn token_classification(
		&self,
		model_input: HashMap<String, Tensor>,
		_labels: Option<&Tensor>,
	) -> Result<Vec<(String, String)>, LLMError> {
		if let Some(token_classification_model) = &self.token_classification_model {
			let input_ids = model_input.get("input_ids").ok_or_else(|| {
				LLMError::new(
					LLMErrorKind::ModelError,
					Arc::new(anyhow::anyhow!("missing input_ids in token classification")),
				)
			})?;
			let token_type_ids = model_input.get("token_type_ids").ok_or_else(|| {
				LLMError::new(
					LLMErrorKind::ModelError,
					Arc::new(anyhow::anyhow!("missing token_type_ids in token classification")),
				)
			})?;
			let output = token_classification_model
				.forward(&input_ids, &token_type_ids, None)
				.map_err(|e| {
					LLMError::new(
						LLMErrorKind::ModelError,
						Arc::new(anyhow::anyhow!(
							"token classification forward pass failed: {}",
							e
						)),
					)
				})?;

			// Calculate softmax probabilities
			let logits = output.logits;
			let _probabilities = candle_nn::ops::softmax(&logits, candle_core::D::Minus1)
				.map_err(|e| {
					LLMError::new(
						LLMErrorKind::ModelError,
						Arc::new(anyhow::anyhow!("softmax calculation failed: {}", e)),
					)
				})?
				.to_vec3::<f32>()
				.map_err(|e| {
					LLMError::new(
						LLMErrorKind::ModelError,
						Arc::new(anyhow::anyhow!("conversion to Vec3 failed: {}", e)),
					)
				})?;
			let final_classification_logits = logits.to_vec3::<f32>().map_err(|e| {
				LLMError::new(
					LLMErrorKind::ModelError,
					Arc::new(anyhow::anyhow!("conversion to Vec3 failed: {}", e)),
				)
			})?;
			let input_ids_vec_2d = input_ids.to_vec2::<u32>().map_err(|e| {
				LLMError::new(
					LLMErrorKind::ModelError,
					Arc::new(anyhow::anyhow!("conversion to Vec2 failed: {}", e)),
				)
			})?;
			let input_ids_vec: Vec<u32> = input_ids_vec_2d.into_iter().flatten().collect();
			let input_ids_u32: Vec<u32> = input_ids_vec.iter().map(|&id| id as u32).collect();
			let token_string: Result<Vec<String>, LLMError> = input_ids_u32
				.iter()
				.map(|&id| {
					self.tokenizer.decode(&[id], false).map_err(|e| {
						LLMError::new(
							LLMErrorKind::ModelError,
							Arc::new(anyhow::anyhow!("token decoding failed: {}", e)),
						)
					})
				})
				.collect();
			let tokens: Vec<String> = token_string?;
			let config = &self
				.token_classification_model
				.as_ref()
				.ok_or_else(|| {
					LLMError::new(
						LLMErrorKind::ModelError,
						Arc::new(anyhow::anyhow!("token classification model config not found")),
					)
				})?
				.config;
			let id2label = match &config.id2label {
				Some(map) => map,
				None => {
					return Err(LLMError::new(
						LLMErrorKind::ModelError,
						Arc::new(anyhow::anyhow!("id2label not found in model config")),
					));
				},
			};
			// Map tokens to their predicted labels
			let mut entity_predictions = Vec::new();
			let default_label = "O".to_string();

			for (token, probs) in tokens.iter().zip(final_classification_logits[0].iter()) {
				let max_prob = probs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
				let label_idx = probs.iter().position(|&p| p == max_prob).ok_or_else(|| {
					LLMError::new(
						LLMErrorKind::ModelError,
						Arc::new(anyhow::anyhow!("max probability not found")),
					)
				})? as i64;
				let label = match id2label.get(&label_idx.to_string()) {
					Some(label) => label,
					None => &default_label,
				};
				entity_predictions.push((token.clone(), label.clone()));
			}

			Ok(entity_predictions)
		} else {
			Err(LLMError::new(
				LLMErrorKind::ModelError,
				Arc::new(anyhow::anyhow!("token classification model not initialized")),
			))
		}
	}

	async fn generate(&self, _messages: &[Message]) -> LLMResult<GenerateResult> {
		// Not supported
		Err(LLMError::new(
			LLMErrorKind::ModelError,
			Arc::new(anyhow::anyhow!("generation not supported")),
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::transformers::roberta::roberta::RobertaLLM;
	use tokio::test;

	#[test]
	async fn test_inference_and_attention_processing() {
		rustls::crypto::ring::default_provider()
			.install_default()
			.expect("Failed to install ring as the default crypto provider");
		let options = EmbedderOptions {
			model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
			local_dir: None,
			revision: None,
			distribution: None,
		};
		let embedder = match BertLLM::new(options) {
			Ok(embedder) => embedder,
			Err(e) => {
				eprintln!("Failed to create BertLLM: {:?}", e);
				return;
			},
		};

		let entities = vec![
			"oil".to_string(),
			"gas".to_string(),
			"porosity".to_string(),
			"joel".to_string(),
			"india".to_string(),
			"microsoft".to_string(),
			"nitrogen gas".to_string(),
			"deposition".to_string(),
		];
		// let entities: Vec<String> = vec![];

		let ner_llm: Option<Arc<dyn LLM>> = if entities.is_empty() {
			let ner_options = EmbedderOptions {
				model: "Davlan/xlm-roberta-base-wikiann-ner".to_string(),
				local_dir: None,
				revision: None,
				distribution: None,
			};

			match RobertaLLM::new(ner_options) {
				Ok(llm) => Some(Arc::new(llm) as Arc<dyn LLM>),
				Err(e) => {
					eprintln!("Failed to create RobertaLLM: {:?}", e);
					return;
				},
			}
		} else {
			None
		};

		let input_text = "Joel lives in India and works for the company microsoft.";
		let tokens = match embedder.tokenize(&input_text).await {
			Ok(tokens) => tokens,
			Err(e) => {
				eprintln!("Tokenization failed: {:?}", e);
				return;
			},
		};

		let model_input = match embedder.model_input(tokens.clone()).await {
			Ok(model_input) => model_input,
			Err(e) => {
				eprintln!("Model input creation failed: {:?}", e);
				return;
			},
		};

		if let Some(ner_llm) = ner_llm {
			let tokens = match ner_llm.tokenize(&input_text).await {
				Ok(tokens) => tokens,
				Err(e) => {
					eprintln!("Tokenization failed: {:?}", e);
					return;
				},
			};

			let model_input = match ner_llm.model_input(tokens.clone()).await {
				Ok(model_input) => model_input,
				Err(e) => {
					eprintln!("Model input creation failed: {:?}", e);
					return;
				},
			};

			let ner_results = ner_llm.token_classification(model_input, None).await;
			match ner_results {
				Ok(results) => {
					let contains_joel_per =
						results.iter().any(|(token, entity)| token == "Joel" && entity == "B-PER");
					assert!(
						contains_joel_per,
						"Assertion failed: Expected 'Joel' to be identified as 'B-PER'"
					);
				},
				Err(e) => {
					eprintln!("NER model returned an error: {:?}", e);
					return;
				},
			}
		}

		// Perform inference to get attention weights
		match embedder.inference_attention(model_input).await {
			Ok(tensor) => {
				// Process the attention weights to remove CLS and SEP tokens
				match embedder.attention_tensor_to_2d_vector(&tensor).await {
					Ok(weights) => tracing::info!("Processed Attention Weights: {:?}", weights),
					Err(e) => tracing::error!("Failed to process attention weights: {:?}", e),
				}
			},
			Err(e) => tracing::error!("Failed to perform inference: {:?}", e),
		}
	}
}
