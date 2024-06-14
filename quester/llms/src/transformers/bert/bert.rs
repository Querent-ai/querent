use async_trait::async_trait;
use candle_core::{DType, Tensor};
use candle_nn::VarBuilder;
// use candle_transformers::models::bert::{BertModel as CandleBertModel, Config, DTYPE};
use crate::transformers::bert::bert_model_functions::{BertModel as CandleBertModel, BertConfig, DTYPE};
use crate::transformers::modelling_outputs::TokenClassifierOutput;
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::path::PathBuf;
use std::{collections::HashMap, sync::Arc};
use tokenizers::{PaddingParams, Tokenizer};

use crate::{LLMError, LLMErrorKind, LLMResult, LLM};

use crate::transformers::DistributionShift;

use super::BertForTokenClassification;

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
	local_dir: Option<String>,
}

impl EmbedderOptions {
	pub fn new() -> Self {
		Self { model: "BAAI/bge-base-en-v1.5".to_string(), revision: None, distribution: None, local_dir: None }
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
	
		let (config_filename, tokenizer_filename, weights_filename, weight_source) = if let Some(local_dir) = options.local_dir.clone() {
			// Read from local directory
			let config_filename = PathBuf::from(format!("{}/config.json", local_dir));
			let tokenizer_filename = PathBuf::from(format!("{}/tokenizer.json", local_dir));
			let (weights_filename, weight_source) = {
				let safetensors_path = PathBuf::from(format!("{}/model.safetensors", local_dir));
				let pytorch_path = PathBuf::from(format!("{}/pytorch_model.bin", local_dir));
	
				if safetensors_path.exists() {
					(safetensors_path, WeightSource::Safetensors)
				} else if pytorch_path.exists() {
					(pytorch_path, WeightSource::Pytorch)
				} else {
					return Err(LLMError::new(
						LLMErrorKind::Io,
						Arc::new(anyhow::anyhow!("could not find model weights in local directory")),
					));
				}
			};
			(config_filename, tokenizer_filename, weights_filename, weight_source)
		} else {
			// Fetch from Hugging Face API
			let repo = match options.revision.clone() {
				Some(revision) => Repo::with_revision(options.model.clone(), RepoType::Model, revision),
				None => Repo::model(options.model.clone()),
			};
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
		println!("BERT Config......{:?}", config);
	
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
		let token_classification_model = if config._num_labels.is_some() || config.id2label.is_some() {
			Some(BertForTokenClassification::load(vb.clone(), &config).map_err(|e| {
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
	
		let this = Self { model, token_classification_model, tokenizer, options, device };
	
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

    async fn model_input(&self, tokenized_sequence: Vec<i32>) -> Result<HashMap<String, Tensor>, LLMError> {
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
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?;
		let token_type_ids = Tensor::zeros(shape.clone(), DType::I64, &self.device)
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?;
		let attention_mask = Tensor::ones(shape, DType::I64, &self.device)
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?;
	
		let mut input_map = HashMap::new();
		input_map.insert("input_ids".to_string(), input_ids);
		input_map.insert("token_type_ids".to_string(), token_type_ids);
		input_map.insert("attention_mask".to_string(), attention_mask);
	
		Ok(input_map)
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

	async fn inference_attention(&self, model_input: HashMap<String, Tensor>) ->Result<Tensor, LLMError> { 
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
        512
    }

	async fn tokens_to_words(&self, tokens: &[i32]) -> Vec<String> {
		let words = tokens.iter().map(|&token| {
			let token_u32 = token as u32;
			self.tokenizer.decode(&[token_u32], false).unwrap_or_else(|_| "[UNK]".to_string())
		}).collect::<Vec<String>>();
		words
	}
	

	async fn process_attention_weights(&self, attention_weights: &Tensor) -> Result<Vec<Vec<f32>>, LLMError> {
		let attention_weights_2d = attention_weights.squeeze(0)
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?
			.to_vec2::<f32>()
			.map_err(|e| LLMError::new(LLMErrorKind::ModelError, Arc::new(e.into())))?;

		// Remove first and last rows and columns (corresponding to [CLS] and [SEP])
		let trimmed_attention_weights: Vec<Vec<f32>> = attention_weights_2d[1..attention_weights_2d.len()-1]
			.iter()
			.map(|row| row[1..row.len()-1].to_vec())
			.collect();
	
		Ok(trimmed_attention_weights)
	}


	// async fn token_classification(
	// 	&self,
	// 	model_input: HashMap<String, Tensor>,
	// 	labels: Option<&Tensor>,
	// ) -> Result<Vec<(String, String)>, LLMError> {
	// 	if let Some(token_classification_model) = &self.token_classification_model {
	// 		let input_ids = model_input.get("input_ids").ok_or_else(|| {
	// 			LLMError::new(
	// 				LLMErrorKind::ModelError,
	// 				Arc::new(anyhow::anyhow!("missing input_ids in token classification")),
	// 			)
	// 		})?;
	// 		let token_type_ids = model_input.get("token_type_ids").ok_or_else(|| {
	// 			LLMError::new(
	// 				LLMErrorKind::ModelError,
	// 				Arc::new(anyhow::anyhow!("missing token_type_ids in token classification")),
	// 			)
	// 		})?;
	// 		let output = token_classification_model
	// 			.forward(input_ids, token_type_ids, labels)
	// 			.map_err(|e| {
	// 				LLMError::new(
	// 					LLMErrorKind::ModelError,
	// 					Arc::new(anyhow::anyhow!("token classification forward pass failed: {}", e)),
	// 				)
	// 			})?;
	
	// 		// Calculate softmax probabilities
	// 		let logits = output.logits;
	// 		let probabilities = candle_nn::ops::softmax(&logits, candle_core::D::Minus1).unwrap().to_vec3::<f32>().unwrap();

	// 		// Get tokens
	// 		let input_ids_vec = input_ids.to_vec1::<i64>().unwrap();
	// 		let input_ids_u32: Vec<u32> = input_ids_vec.iter().map(|&id| id as u32).collect();
	// 		let tokens_string = self.tokenizer.decode(&input_ids_u32, false).unwrap();
    //     	let tokens: Vec<String> = tokens_string.split_whitespace().map(String::from).collect();
	
	// 		// Decode logits to get predicted labels
	// 		let config = &self.token_classification_model.as_ref().unwrap().config;
	// 		let id2label = match &config.id2label {
	// 			Some(map) => map,
	// 			None => panic!("id2label not found in model config"),
	// 		};
	
	// 		// Map tokens to their predicted labels
	// 		let mut entity_predictions = Vec::new();
	// 		let default_label = "O".to_string();
	
	// 		for (token, probs) in tokens.iter().zip(probabilities[0].iter()) {
	// 			let max_prob = probs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
	// 			let label_idx = probs.iter().position(|&p| p == max_prob).unwrap() as i64;
	// 			let label = id2label.get(&label_idx.to_string()).unwrap_or(&default_label);
	// 			entity_predictions.push((token.clone(), label.clone()));
	// 		}
	
	// 		Ok(entity_predictions)
	// 	} else {
	// 		Err(LLMError::new(
	// 			LLMErrorKind::ModelError,
	// 			Arc::new(anyhow::anyhow!("token classification model not initialized")),
	// 		))
	// 	}
	// }
	
	async fn token_classification(
        &self,
        model_input: HashMap<String, Tensor>,
        labels: Option<&Tensor>,
    ) -> Result<TokenClassifierOutput, LLMError> {
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
                .forward(input_ids, token_type_ids, labels)
                .map_err(|e| {
                    LLMError::new(
                        LLMErrorKind::ModelError,
                        Arc::new(anyhow::anyhow!("token classification forward pass failed: {}", e)),
                    )
                })?;
            Ok(output)
        } else {
            Err(LLMError::new(
                LLMErrorKind::ModelError,
                Arc::new(anyhow::anyhow!("token classification model not initialized")),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_process_attention_weights() {
        let options = EmbedderOptions {
            // model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
			// local_dir: None,
			model: "/home/nishantg/querent-main/local models/geobert_files".to_string(),
			local_dir : Some("/home/nishantg/querent-main/local models/geobert_files".to_string()),
            revision: None,
            distribution: None,
			
        };
        let embedder = BertLLM::new(options).unwrap();

        // Tokenize input text
		//let input_text = "Joel lives in Delhi".to_string();
        let input_text = "The tectonic movements in the Jurassic era are not common.";
		let tokens = embedder.tokenize(&input_text).await;
        println!("These are the Tokens: {:?}", tokens);
		let encoding = embedder.tokenizer.encode(input_text, true).unwrap();
        // Prepare model input
        let model_input1 = embedder.model_input(tokens.clone()).await;
		// let model_input = embedder.model_input(tokens.clone()).await;
		// let model_input = match model_input {
        //     Ok(ref input) => {
        //         println!("Model Input: {:?}", input);

        //         // Perform inference to get attention weights
        //         match embedder.inference_attention(input.clone()).await {
        //             Ok(tensor) => {
        //                 println!("Output Tensor: {:?}", tensor);
        //                 Ok(tensor)
        //             }
        //             Err(e) => {
        //                 println!("Failed to perform inference: {:?}", e);
        //                 Err(e)
        //             }
        //         }
        //     }
        //     Err(ref e) => {
        //         println!("Failed to create model input: {:?}", e);
        //         return;
        //     }
        // };


		// // Handle the Result type properly
		// let output_tensor = match model_input {
		// 	Ok(tensor) => tensor,
		// 	Err(e) => {
		// 		println!("Failed to get output tensor: {:?}", e);
		// 		return;
		// 	}
		// };
		
		


		// // Process the attention weights to remove CLS and SEP tokens
		// let processed_attention_weights = match embedder.process_attention_weights(&output_tensor).await {
		// 	Ok(weights) => weights,
		// 	Err(e) => {
		// 		println!("Failed to process attention weights: {:?}", e);
		// 		return;
		// 	}
		// };
		// println!("Processed Attention Weights: {:?}", processed_attention_weights);	
        // Map tokens to words
        let words = embedder.tokens_to_words(&tokens).await;
        for (token, word) in tokens.iter().zip(words.iter()) {
            println!("Token {} corresponds to word '{}'", token, word);
        }

		// Perform token classification
        let token_classification_output = match embedder.token_classification(model_input1.unwrap().clone(), None).await {
            Ok(output) => output,
            Err(e) => {
                println!("Failed to perform token classification: {:?}", e);
                return;
            }
        };
        println!("Token Classification Logits: {:?}", token_classification_output.logits);
		let logits = token_classification_output.logits;

		// Calculate softmax probabilities
        let probabilities = candle_nn::ops::softmax(&logits, candle_core::D::Minus1).unwrap().to_vec3::<f32>().unwrap();

        // Get tokens
        let tokens = encoding.get_tokens().iter().map(|s| s.to_string()).collect::<Vec<String>>();

        // Decode logits to get predicted labels
        let config = &embedder.token_classification_model.as_ref().unwrap().config;
        let id2label = match &config.id2label {
            Some(map) => map,
            None => panic!("id2label not found in model config"),
        };

        // Map tokens to their predicted labels
        let mut entity_predictions = Vec::new();
        let default_label = "O".to_string();

        for (token, probs) in tokens.iter().zip(probabilities[0].iter()) {
            let max_prob = probs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let label_idx = probs.iter().position(|&p| p == max_prob).unwrap() as i64;
            let label = id2label.get(&label_idx.to_string()).unwrap_or(&default_label);
            entity_predictions.push((token.clone(), label.clone()));
        }

        // Print tokens and their predicted labels
        for (token, label) in entity_predictions.iter() {
            println!("Token: {}, Label: {}", token, label);
        }

    }
}
