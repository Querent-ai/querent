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

use candle_core::{DType, Device, IndexOp, Result, Tensor};
use candle_nn::{Embedding, Module, VarBuilder};
use candle_transformers::models::with_tracing::{layer_norm, linear, LayerNorm, Linear};
use std::collections::HashMap;

use crate::transformers::modelling_outputs::{
	QuestionAnsweringModelOutput, SequenceClassifierOutput, TokenClassifierOutput,
};
use serde::Deserialize;
use std::sync::RwLock;
pub const FLOATING_DTYPE: DType = DType::F32;
pub const LONG_DTYPE: DType = DType::I64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HiddenAct {
	Gelu,
	GeluApproximate,
	Relu,
	Tanh,
}

struct HiddenActLayer {
	act: HiddenAct,
	span: tracing::Span,
}

impl HiddenActLayer {
	fn new(act: HiddenAct) -> Self {
		let span = tracing::span!(tracing::Level::TRACE, "hidden-act");
		Self { act, span }
	}

	fn forward(&self, xs: &Tensor) -> candle_core::Result<Tensor> {
		let _enter = self.span.enter();
		match self.act {
			HiddenAct::Gelu => xs.gelu_erf(),
			HiddenAct::GeluApproximate => xs.gelu(),
			HiddenAct::Relu => xs.relu(),
			HiddenAct::Tanh => xs.tanh(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PositionEmbeddingType {
	#[default]
	Absolute,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RobertaConfig {
	vocab_size: usize,
	hidden_size: usize,
	num_hidden_layers: usize,
	num_attention_heads: usize,
	intermediate_size: usize,
	hidden_act: HiddenAct,
	hidden_dropout_prob: f64,
	max_position_embeddings: usize,
	type_vocab_size: usize,
	initializer_range: f64,
	layer_norm_eps: f64,
	pad_token_id: usize,
	bos_token_id: usize,
	eos_token_id: usize,
	#[serde(default)]
	position_embedding_type: PositionEmbeddingType,
	#[serde(default)]
	use_cache: bool,
	classifier_dropout: Option<f64>,
	model_type: Option<String>,
	problem_type: Option<String>,
	pub _num_labels: Option<usize>,
	pub id2label: Option<HashMap<String, String>>,
	pub label2id: Option<HashMap<String, usize>>,
}

impl Default for RobertaConfig {
	fn default() -> Self {
		Self {
			vocab_size: 50265,
			hidden_size: 768,
			num_hidden_layers: 12,
			num_attention_heads: 12,
			intermediate_size: 3072,
			hidden_act: HiddenAct::Gelu,
			hidden_dropout_prob: 0.1,
			max_position_embeddings: 512,
			type_vocab_size: 2,
			initializer_range: 0.02,
			layer_norm_eps: 1e-12,
			pad_token_id: 1,
			bos_token_id: 0,
			eos_token_id: 2,
			position_embedding_type: PositionEmbeddingType::Absolute,
			use_cache: true,
			classifier_dropout: None,
			model_type: Some("roberta".to_string()),
			problem_type: None,
			_num_labels: Some(3),
			id2label: None,
			label2id: None,
		}
	}
}

struct Dropout {
	#[allow(dead_code)]
	pr: f64,
}

impl Dropout {
	fn new(pr: f64) -> Self {
		Self { pr }
	}
}

impl Module for Dropout {
	fn forward(&self, x: &Tensor) -> Result<Tensor> {
		Ok(x.clone())
	}
}

fn cumsum_2d(mask: &Tensor, dim: u8, device: &Device) -> Result<Tensor> {
	let mask = mask.to_vec2::<u8>()?;

	let rows = mask.len();
	let cols = mask[0].len();

	let mut result = mask.clone();

	match dim {
		0 => {
			// Cumulative sum along rows
			for i in 0..rows {
				for j in 1..cols {
					result[i][j] += result[i][j - 1];
				}
			}
		},
		1 => {
			// Cumulative sum along columns
			for j in 0..cols {
				for i in 1..rows {
					result[i][j] += result[i - 1][j];
				}
			}
		},
		_ => panic!("Dimension not supported"),
	}

	let result = Tensor::new(result, &device)?;

	Ok(result)
}

pub fn sigmoid(xs: &Tensor) -> Result<Tensor> {
	// TODO: Should we have a specialized op for this?
	(xs.neg()?.exp()? + 1.0)?.recip()
}

pub fn binary_cross_entropy_with_logit(inp: &Tensor, target: &Tensor) -> Result<Tensor> {
	let inp = sigmoid(inp)?;

	let left_side = target * inp.log()?;
	let right_side = (target.affine(-1., 1.))? * inp.affine(-1., 1.)?.log()?;

	let loss = left_side? + right_side?;
	let loss = loss?.neg()?.mean_all()?;

	Ok(loss)
}

pub fn create_position_ids_from_input_ids(
	input_ids: &Tensor,
	padding_idx: u32,
	past_key_values_length: u8,
) -> Result<Tensor> {
	let mask = input_ids.ne(padding_idx)?;
	let incremental_indices = cumsum_2d(&mask, 0, input_ids.device())?;

	let incremental_indices = incremental_indices
		.broadcast_add(&Tensor::new(&[past_key_values_length], input_ids.device())?)?;

	Ok(incremental_indices)
}

fn embedding(vocab_size: usize, hidden_size: usize, vb: VarBuilder) -> Result<Embedding> {
	let embeddings = vb.get((vocab_size, hidden_size), "weight")?;
	Ok(Embedding::new(embeddings, hidden_size))
}

pub struct RobertaEmbeddings {
	word_embeddings: Embedding,
	position_embeddings: Option<Embedding>,
	token_type_embeddings: Embedding,
	layer_norm: LayerNorm,
	dropout: Dropout,
	pub padding_idx: u32,
}

impl RobertaEmbeddings {
	pub fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let word_embeddings =
			embedding(config.vocab_size, config.hidden_size, vb.pp("word_embeddings"))?;
		let position_embeddings = embedding(
			config.max_position_embeddings,
			config.hidden_size,
			vb.pp("position_embeddings"),
		)?;
		let token_type_embeddings =
			embedding(config.type_vocab_size, config.hidden_size, vb.pp("token_type_embeddings"))?;
		let layer_norm = layer_norm(config.hidden_size, config.layer_norm_eps, vb.pp("LayerNorm"))?;
		let padding_idx = config.pad_token_id as u32;

		Ok(Self {
			word_embeddings,
			position_embeddings: Some(position_embeddings),
			token_type_embeddings,
			layer_norm,
			dropout: Dropout::new(config.hidden_dropout_prob),
			padding_idx,
		})
	}

	pub fn forward(
		&self,
		input_ids: &Tensor,
		token_type_ids: &Tensor,
		position_ids: Option<&Tensor>,
		inputs_embeds: Option<&Tensor>,
	) -> Result<Tensor> {
		let position_ids = match position_ids {
			Some(ids) => ids.to_owned(),
			None =>
				if Option::is_some(&inputs_embeds) {
					let position_ids =
						self.create_position_ids_from_input_embeds(inputs_embeds.unwrap())?;
					position_ids
				} else {
					let position_ids =
						create_position_ids_from_input_ids(input_ids, self.padding_idx, 1)?;
					position_ids
				},
		};

		let inputs_embeds: Tensor = match inputs_embeds {
			Some(embeds) => embeds.to_owned(),
			None => {
				let embeds = self.word_embeddings.forward(input_ids)?;
				embeds
			},
		};

		let token_type_embeddings = self.token_type_embeddings.forward(token_type_ids)?;
		let mut embeddings = (inputs_embeds + token_type_embeddings)?;

		if let Some(position_embeddings) = &self.position_embeddings {
			embeddings = embeddings.broadcast_add(&position_embeddings.forward(&position_ids)?)?
		}

		let embeddings = self.layer_norm.forward(&embeddings)?;
		let embeddings = self.dropout.forward(&embeddings)?;

		Ok(embeddings)
	}

	pub fn create_position_ids_from_input_embeds(&self, input_embeds: &Tensor) -> Result<Tensor> {
		let input_shape = input_embeds.dims3()?;
		let seq_length = input_shape.1;
		let mut position_ids = Tensor::arange(
			self.padding_idx + 1,
			seq_length as u32 + self.padding_idx + 1,
			&Device::Cpu,
		)?;

		position_ids = position_ids.unsqueeze(0)?.expand((input_shape.0, input_shape.1))?;
		Ok(position_ids)
	}
}

struct RobertaSelfAttention {
	query: Linear,
	key: Linear,
	value: Linear,
	dropout: Dropout,
	num_attention_heads: usize,
	attention_head_size: usize,
	attention_probs: RwLock<Option<Tensor>>,
}

impl RobertaSelfAttention {
	fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let attention_head_size = config.hidden_size / config.num_attention_heads;
		let all_head_size = config.num_attention_heads * attention_head_size;
		let dropout = Dropout::new(config.hidden_dropout_prob);
		let hidden_size = config.hidden_size;
		let query = linear(hidden_size, all_head_size, vb.pp("query"))?;
		let value = linear(hidden_size, all_head_size, vb.pp("value"))?;
		let key = linear(hidden_size, all_head_size, vb.pp("key"))?;
		Ok(Self {
			query,
			key,
			value,
			dropout,
			num_attention_heads: config.num_attention_heads,
			attention_head_size,
			attention_probs: RwLock::new(None),
		})
	}

	fn transpose_for_scores(&self, xs: &Tensor) -> Result<Tensor> {
		let mut new_x_shape = xs.dims().to_vec();
		new_x_shape.pop();
		new_x_shape.push(self.num_attention_heads);
		new_x_shape.push(self.attention_head_size);
		let xs = xs.reshape(new_x_shape.as_slice())?.transpose(1, 2)?;
		xs.contiguous()
	}

	fn forward(&self, hidden_states: &Tensor) -> Result<Tensor> {
		let query_layer = self.query.forward(hidden_states)?;
		let key_layer = self.key.forward(hidden_states)?;
		let value_layer = self.value.forward(hidden_states)?;

		let query_layer = self.transpose_for_scores(&query_layer)?;
		let key_layer = self.transpose_for_scores(&key_layer)?;
		let value_layer = self.transpose_for_scores(&value_layer)?;

		let attention_scores = query_layer.matmul(&key_layer.t()?)?;
		let attention_scores = (attention_scores / (self.attention_head_size as f64).sqrt())?;
		let attention_probs =
			{ candle_nn::ops::softmax(&attention_scores, candle_core::D::Minus1)? };
		let attention_probs = self.dropout.forward(&attention_probs)?;
		*self.attention_probs.write().unwrap() = Some(attention_probs.clone());
		let context_layer = attention_probs.matmul(&value_layer)?;
		let context_layer = context_layer.transpose(1, 2)?.contiguous()?;
		let context_layer = context_layer.flatten_from(candle_core::D::Minus2)?;
		Ok(context_layer)
	}
}

struct RobertaSelfOutput {
	dense: Linear,
	layer_norm: LayerNorm,
	dropout: Dropout,
}

impl RobertaSelfOutput {
	fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let dense = linear(config.hidden_size, config.hidden_size, vb.pp("dense"))?;
		let layer_norm = layer_norm(config.hidden_size, config.layer_norm_eps, vb.pp("LayerNorm"))?;
		let dropout = Dropout::new(config.hidden_dropout_prob);
		Ok(Self { dense, layer_norm, dropout })
	}

	fn forward(&self, hidden_states: &Tensor, input_tensor: &Tensor) -> Result<Tensor> {
		let hidden_states = self.dense.forward(hidden_states)?;
		let hidden_states = self.dropout.forward(&hidden_states)?;
		self.layer_norm.forward(&(hidden_states + input_tensor)?)
	}
}

struct RobertaAttention {
	self_attention: RobertaSelfAttention,
	self_output: RobertaSelfOutput,
}

impl RobertaAttention {
	fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let self_attention = RobertaSelfAttention::load(vb.pp("self"), config)?;
		let self_output = RobertaSelfOutput::load(vb.pp("output"), config)?;
		Ok(Self { self_attention, self_output })
	}

	fn forward(&self, hidden_states: &Tensor) -> Result<Tensor> {
		let self_outputs = self.self_attention.forward(hidden_states)?;
		let attention_output = self.self_output.forward(&self_outputs, hidden_states)?;
		Ok(attention_output)
	}
}

struct RobertaIntermediate {
	dense: Linear,
	intermediate_act: HiddenActLayer,
}

impl RobertaIntermediate {
	fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let dense = linear(config.hidden_size, config.intermediate_size, vb.pp("dense"))?;
		Ok(Self { dense, intermediate_act: HiddenActLayer::new(config.hidden_act) })
	}

	fn forward(&self, hidden_states: &Tensor) -> Result<Tensor> {
		let hidden_states = self.dense.forward(hidden_states)?;
		let ys = self.intermediate_act.forward(&hidden_states)?;
		Ok(ys)
	}
}

struct RobertaOutput {
	dense: Linear,
	layer_norm: LayerNorm,
	dropout: Dropout,
}

impl RobertaOutput {
	fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let dense = linear(config.intermediate_size, config.hidden_size, vb.pp("dense"))?;
		let layer_norm = layer_norm(config.hidden_size, config.layer_norm_eps, vb.pp("LayerNorm"))?;
		let dropout = Dropout::new(config.hidden_dropout_prob);
		Ok(Self { dense, layer_norm, dropout })
	}

	fn forward(&self, hidden_states: &Tensor, input_tensor: &Tensor) -> Result<Tensor> {
		let hidden_states = self.dense.forward(hidden_states)?;
		let hidden_states = self.dropout.forward(&hidden_states)?;
		self.layer_norm.forward(&(hidden_states + input_tensor)?)
	}
}

struct RobertaLayer {
	attention: RobertaAttention,
	intermediate: RobertaIntermediate,
	output: RobertaOutput,
}

impl RobertaLayer {
	fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let attention = RobertaAttention::load(vb.pp("attention"), config)?;
		let intermediate = RobertaIntermediate::load(vb.pp("intermediate"), config)?;
		let output = RobertaOutput::load(vb.pp("output"), config)?;
		Ok(Self { attention, intermediate, output })
	}

	fn forward(&self, hidden_states: &Tensor) -> Result<Tensor> {
		let attention_output = self.attention.forward(hidden_states)?;

		let intermediate_output = self.intermediate.forward(&attention_output)?;
		let layer_output = self.output.forward(&intermediate_output, &attention_output)?;
		Ok(layer_output)
	}
}

struct RobertaEncoder {
	layers: Vec<RobertaLayer>,
}

impl RobertaEncoder {
	fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let layers = (0..config.num_hidden_layers)
			.map(|index| RobertaLayer::load(vb.pp(&format!("layer.{index}")), config))
			.collect::<Result<Vec<_>>>()?;
		Ok(RobertaEncoder { layers })
	}

	fn forward(&self, hidden_states: &Tensor) -> Result<Tensor> {
		let mut hidden_states = hidden_states.clone();
		for layer in self.layers.iter() {
			hidden_states = layer.forward(&hidden_states)?
		}
		Ok(hidden_states)
	}
}

pub struct RobertaPooler {
	dense: Linear,
	activation: HiddenActLayer,
}

impl RobertaPooler {
	pub fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let dense = linear(config.hidden_size, config.hidden_size, vb.pp("dense"))?;
		Ok(Self { dense, activation: HiddenActLayer::new(HiddenAct::Tanh) })
	}

	pub fn forward(&self, hidden_states: &Tensor) -> Result<Tensor> {
		// We "pool" the model by simply taking the hidden state corresponding
		// to the first token.

		let first_token_sensor = hidden_states.i((.., 0))?;
		let pooled_output = self.dense.forward(&first_token_sensor)?;
		let pooled_output = self.activation.forward(&pooled_output)?;

		Ok(pooled_output)
	}
}

pub struct RobertaModel {
	embeddings: RobertaEmbeddings,
	encoder: RobertaEncoder,
	pub device: Device,
}

impl RobertaModel {
	pub fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let (embeddings, encoder) = match (
			RobertaEmbeddings::load(vb.pp("embeddings"), config),
			RobertaEncoder::load(vb.pp("encoder"), config),
		) {
			(Ok(embeddings), Ok(encoder)) => (embeddings, encoder),
			(Err(err), _) | (_, Err(err)) =>
				if let Some(model_type) = &config.model_type {
					if let (Ok(embeddings), Ok(encoder)) = (
						RobertaEmbeddings::load(vb.pp(&format!("{model_type}.embeddings")), config),
						RobertaEncoder::load(vb.pp(&format!("{model_type}.encoder")), config),
					) {
						(embeddings, encoder)
					} else {
						return Err(err);
					}
				} else {
					return Err(err);
				},
		};
		Ok(Self { embeddings, encoder, device: vb.device().clone() })
	}

	pub fn get_last_attention_probs(&self) -> Option<Tensor> {
		self.encoder
			.layers
			.last()?
			.attention
			.self_attention
			.attention_probs
			.read()
			.unwrap()
			.clone()
	}

	pub fn forward(&self, input_ids: &Tensor, token_type_ids: &Tensor) -> Result<Tensor> {
		let embedding_output = self.embeddings.forward(input_ids, token_type_ids, None, None)?;
		let sequence_output = self.encoder.forward(&embedding_output)?;
		Ok(sequence_output)
	}
}

pub struct RobertaModelWithPooler {
	embeddings: RobertaEmbeddings,
	encoder: RobertaEncoder,
	pooler: RobertaPooler,
	pub device: Device,
}

impl RobertaModelWithPooler {
	pub fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let (embeddings, encoder, pooler) = match (
			RobertaEmbeddings::load(vb.pp("embeddings"), config),
			RobertaEncoder::load(vb.pp("encoder"), config),
			RobertaPooler::load(vb.pp("pooler"), config),
		) {
			(Ok(embeddings), Ok(encoder), Ok(pooler)) => (embeddings, encoder, pooler),
			(Err(err), _, _) | (_, Err(err), _) | (_, _, Err(err)) => {
				if let Some(model_type) = &config.model_type {
					if let (Ok(embeddings), Ok(encoder), Ok(pooler)) = (
						RobertaEmbeddings::load(vb.pp(&format!("{model_type}.embeddings")), config),
						RobertaEncoder::load(vb.pp(&format!("{model_type}.encoder")), config),
						RobertaPooler::load(vb.pp(&format!("{model_type}.pooler")), config),
					) {
						(embeddings, encoder, pooler)
					} else {
						return Err(err);
					}
				} else {
					return Err(err);
				}
			},
		};
		Ok(Self { embeddings, encoder, pooler, device: vb.device().clone() })
	}

	pub fn forward(&self, input_ids: &Tensor, token_type_ids: &Tensor) -> Result<Tensor> {
		let embedding_output = self.embeddings.forward(input_ids, token_type_ids, None, None)?;
		let sequence_output = self.encoder.forward(&embedding_output)?;
		let pooled_output = self.pooler.forward(&sequence_output)?;
		Ok(pooled_output)
	}
}

struct RobertaClassificationHead {
	dense: Linear,
	dropout: Dropout,
	out_proj: Linear,
}

impl RobertaClassificationHead {
	fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let dense = linear(config.hidden_size, config.hidden_size, vb.pp("dense"))?;
		let classifier_dropout = config.classifier_dropout;

		let classifier_dropout: f64 = match classifier_dropout {
			Some(classifier_dropout) => classifier_dropout,
			None => config.hidden_dropout_prob,
		};
		let out_proj = linear(config.hidden_size, config._num_labels.unwrap(), vb.pp("out_proj"))?;

		Ok(Self { dense, dropout: Dropout::new(classifier_dropout), out_proj })
	}

	fn forward(&self, features: &Tensor) -> Result<Tensor> {
		let x = features.i((.., 0))?;
		let x = self.dropout.forward(&x)?;
		let x = self.dense.forward(&x)?;
		let x = x.tanh()?;
		let x = self.dropout.forward(&x)?;
		let x = self.out_proj.forward(&x)?;

		Ok(x)
	}
}

pub struct RobertaForSequenceClassification {
	roberta: RobertaModel,
	classifier: RobertaClassificationHead,
	pub device: Device,
	config: RobertaConfig,
}

impl RobertaForSequenceClassification {
	pub fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let (roberta, classifier) = match (
			RobertaModel::load(vb.pp("roberta"), config),
			RobertaClassificationHead::load(vb.pp("classifier"), config),
		) {
			(Ok(roberta), Ok(classifier)) => (roberta, classifier),
			(Err(err), _) | (_, Err(err)) => {
				return Err(err);
			},
		};
		Ok(Self { roberta, classifier, device: vb.device().clone(), config: config.clone() })
	}

	pub fn forward(
		&self,
		input_ids: &Tensor,
		token_type_ids: &Tensor,
		labels: Option<&Tensor>,
	) -> Result<SequenceClassifierOutput> {
		let outputs = self.roberta.forward(input_ids, token_type_ids)?;
		let mut problem_type: String = String::from("");

		let logits = self.classifier.forward(&outputs)?;
		let mut loss: Tensor = Tensor::new(vec![0.0], &self.device)?;

		match labels {
			Some(labels) => {
				let labels = labels.to_device(&input_ids.device())?;

				if self.config.problem_type == None {
					if self.config._num_labels == Some(1) {
						problem_type = String::from("regression");
					} else if self.config._num_labels > Some(1) &&
						(labels.dtype() == LONG_DTYPE || labels.dtype() == DType::U32)
					{
						problem_type = String::from("single_label_classification");
					} else {
						problem_type = String::from("multi_label_classification");
					}
				}

				if problem_type == String::from("single_label_classification") {
					loss = candle_nn::loss::cross_entropy(
						&logits.flatten_to(1)?,
						&labels.flatten_to(1)?,
					)?;
				} else if problem_type == String::from("multi_label_classification") {
					let labels_logits: Tensor = logits.zeros_like()?;
					let mut label_logits = labels_logits.to_vec2::<f32>()?;

					let label = vec![0, 1, 2, 3, 2];

					for vec_i in 0..label_logits.len() {
						label_logits[vec_i][label[vec_i]] = 1.0;
					}

					let label_logits = Tensor::new(label_logits, &self.device)?;

					loss = binary_cross_entropy_with_logit(&logits, &label_logits)?;
				}
			},

			None => {},
		}

		Ok(SequenceClassifierOutput {
			loss: Some(loss),
			logits,
			hidden_states: None,
			attentions: None,
		})
	}
}

pub struct RobertaForTokenClassification {
	roberta: RobertaModel,
	dropout: Dropout,
	classifier: Linear,
	pub device: Device,
	pub config: RobertaConfig,
}

impl RobertaForTokenClassification {
	pub fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let classifier_dropout = config.classifier_dropout;

		let (roberta, classifier) = match (
			RobertaModel::load(vb.pp("roberta"), config),
			if Option::is_some(&config._num_labels) {
				linear(config.hidden_size, config._num_labels.unwrap(), vb.pp("classifier"))
			} else if Option::is_some(&config.id2label) {
				let num_labels = &config.id2label.as_ref().unwrap().len();
				linear(config.hidden_size, num_labels.clone(), vb.pp("classifier"))
			} else {
				candle_core::bail!("cannnot find the number of classes to map to")
			},
		) {
			(Ok(roberta), Ok(classifier)) => (roberta, classifier),
			(Err(err), _) | (_, Err(err)) => {
				return Err(err);
			},
		};
		Ok(Self {
			roberta,
			dropout: Dropout::new(classifier_dropout.unwrap_or_else(|| 0.2)),
			classifier,
			device: vb.device().clone(),
			config: config.clone(), // Add this line to store the configuration
		})
	}

	pub fn forward(
		&self,
		input_ids: &Tensor,
		token_type_ids: &Tensor,
		labels: Option<&Tensor>,
	) -> Result<TokenClassifierOutput> {
		let outputs = self.roberta.forward(input_ids, token_type_ids)?;
		let outputs = self.dropout.forward(&outputs)?;

		let logits = self.classifier.forward(&outputs)?;

		let mut loss: Tensor = Tensor::new(vec![0.0], &self.device)?;

		match labels {
			Some(labels) => {
				loss =
					candle_nn::loss::cross_entropy(&logits.flatten_to(1)?, &labels.flatten_to(1)?)?;
			},
			None => {},
		}

		Ok(TokenClassifierOutput {
			loss: Some(loss),
			logits,
			hidden_states: None,
			attentions: None,
		})
	}
}

pub struct RobertaForQuestionAnswering {
	roberta: RobertaModel,
	dropout: Dropout,
	qa_outputs: Linear,
	pub device: Device,
}

impl RobertaForQuestionAnswering {
	pub fn load(vb: VarBuilder, config: &RobertaConfig) -> Result<Self> {
		let classifier_dropout = config.classifier_dropout;

		let (roberta, qa_outputs) = match (
			RobertaModel::load(vb.pp("roberta"), config),
			linear(config.hidden_size, 2, vb.pp("classifier")),
		) {
			(Ok(roberta), Ok(qa_outputs)) => (roberta, qa_outputs),
			(Err(err), _) | (_, Err(err)) => {
				return Err(err);
			},
		};
		Ok(Self {
			roberta,
			dropout: Dropout::new(classifier_dropout.unwrap_or_else(|| 0.2)),
			qa_outputs,
			device: vb.device().clone(),
		})
	}

	pub fn forward(
		&self,
		input_ids: &Tensor,
		token_type_ids: &Tensor,
		start_positions: Option<&Tensor>,
		end_positions: Option<&Tensor>,
	) -> Result<QuestionAnsweringModelOutput> {
		let outputs = self.roberta.forward(input_ids, token_type_ids)?;
		let outputs = self.dropout.forward(&outputs)?;

		let logits = self.qa_outputs.forward(&outputs)?;

		let start_logits = logits.i((.., 0))?;
		let end_logits = logits.i((.., 1))?;
		let mut loss: Tensor = Tensor::new(vec![0.0], &self.device)?;

		match (start_positions, end_positions) {
			(Some(start_positions), Some(end_positions)) => {
				let start_loss = candle_nn::loss::cross_entropy(
					&start_logits.flatten_to(1)?,
					&start_positions.flatten_to(1)?,
				)?;
				let end_loss = candle_nn::loss::cross_entropy(
					&end_logits.flatten_to(1)?,
					&end_positions.flatten_to(1)?,
				)?;

				loss = ((start_loss + end_loss)? / 2.0)?;
			},
			_ => {},
		}

		Ok(QuestionAnsweringModelOutput {
			loss: Some(loss),
			start_logits,
			end_logits,
			hidden_states: None,
			attentions: None,
		})
	}
}
