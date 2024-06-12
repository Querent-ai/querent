use async_trait::async_trait;
use candle_core::{DType, IndexOp, Tensor};
use candle_core::Module;
use candle_nn::VarBuilder;
// use candle_transformers::models::bert::{BertModel as CandleBertModel, Config, DTYPE};
use crate::transformers::bertattention::{BertModel as CandleBertModel, Config, DTYPE};
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
        let cls_token_id = 101; // [CLS]
        let sep_token_id = 102; // [SEP]

        let tokenized_sequence = vec![
            vec![cls_token_id],
            tokenized_sequence,
            vec![sep_token_id],
        ]
        .concat();

        // Convert tokenized_sequence to Vec<i64>
        let tokenized_sequence_i64: Vec<i64> = tokenized_sequence.iter().map(|&x| x as i64).collect();

        // Define the shape as a Vec<usize>
        let shape: Vec<usize> = vec![1, tokenized_sequence_i64.len()];

        // Create the input_ids tensor with the correct shape
        let input_ids = Tensor::from_vec(tokenized_sequence_i64, shape.clone(), &self.device)
            .unwrap();
        let token_type_ids = Tensor::zeros(shape.clone(), DType::I64, &self.device).unwrap();
        let attention_mask = Tensor::ones(shape, DType::I64, &self.device).unwrap();

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

	async fn inference_attention(&self, model_input: HashMap<String, Tensor>) -> Tensor {
		let input_ids = model_input.get("input_ids").unwrap();
        let token_type_ids = model_input.get("token_type_ids").unwrap();

        self.model.forward(input_ids, token_type_ids).unwrap();
		if let Some(attention_probs) = self.model.get_last_attention_probs() {
			println!("Attention weights from the last layer: {:?}", attention_probs);
	
			// Calculate mean over the head dimension
			let mean_attention_probs = attention_probs.mean(1).unwrap();
			println!("Mean attention weights from the last layer: {:?}", mean_attention_probs);
	
			// Print shapes to verify
			let dims = attention_probs.dims();
			println!("Output Shape------------------------ {:?}", dims);
			let mean_dims = mean_attention_probs.dims();
			println!("Mean Shape------------------------------------------- {:?}", mean_dims);
	
			// Return the first element similar to the Python implementation
			let first_mean = mean_attention_probs.i(0).unwrap();
			println!("First element of Mean attention weights--------------------------- {:?}", first_mean);
			mean_attention_probs
			// first_mean
		} else {
			println!("Could not retrieve attention weights.");
			self.model.forward(input_ids, token_type_ids).unwrap()
		}
    }

    async fn maximum_tokens(&self) -> usize {
        512
    }

	async fn tokens_to_words(&self, tokens: &[i32]) -> Vec<String> {
		let words = tokens.iter().map(|&token| {
			let token_u32 = token as u32;
			self.tokenizer.decode(&[token_u32], false).unwrap_or_else(|_| "[UNK]".to_string())
		}).collect::<Vec<String>>();
		println!("Tokens to words mapping: {:?}", words);
		words
	}
	

	async fn process_attention_weights(&self, attention_weights: &Tensor) -> Result<Vec<Vec<f32>>, LLMError> {
		let attention_weights_2d = attention_weights.squeeze(0)
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?
			.to_vec2::<f32>()
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?;
	
		println!("Attention weights 2D: {:?}", attention_weights_2d);
	
		// Remove first and last rows and columns (corresponding to [CLS] and [SEP])
		let trimmed_attention_weights: Vec<Vec<f32>> = attention_weights_2d[1..attention_weights_2d.len()-1]
			.iter()
			.map(|row| row[1..row.len()-1].to_vec())
			.collect();
	
		println!("Trimmed attention weights 2D: {:?}", trimmed_attention_weights);
	
		Ok(trimmed_attention_weights)
	}
	
	
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_process_attention_weights() {
        let options = EmbedderOptions {
            model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            revision: None,
            distribution: None,
        };
        let embedder = BertLLM::new(options).unwrap();

        // Tokenize input text
        let tokens = embedder.tokenize("Joel lives and works in Delhi that is the capital of India").await;
        println!("These are the Tokens: {:?}", tokens);

        // Prepare model input
        let model_input = embedder.model_input(tokens.clone()).await;
        println!("Model Input: {:?}", model_input);

        // Perform inference to get attention weights
        let output_tensor = embedder.inference_attention(model_input).await;
        println!("Output Tensor: {:?}", output_tensor);

        // Process the attention weights to remove CLS and SEP tokens
        let processed_attention_weights = embedder.process_attention_weights(&output_tensor).await.unwrap();
        println!("Processed Attention Weights: {:?}", processed_attention_weights);

        // Map tokens to words
        let words = embedder.tokens_to_words(&tokens).await;
        for (token, word) in tokens.iter().zip(words.iter()) {
            println!("Token {} corresponds to word '{}'", token, word);
        }
    }
}
