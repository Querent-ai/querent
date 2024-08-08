use candle_core::{DType, Device, Result, Tensor};
use candle_nn::{embedding, Embedding, Module, VarBuilder};
use candle_transformers::models::with_tracing::{layer_norm, linear, LayerNorm, Linear};
use serde::Deserialize;
use std::{collections::HashMap, sync::RwLock};

use crate::transformers::modelling_outputs::TokenClassifierOutput;

pub const DTYPE: DType = DType::F32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HiddenAct {
	Gelu,
	GeluApproximate,
	Relu,
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
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum PositionEmbeddingType {
	#[default]
	Absolute,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BertConfig {
	pub vocab_size: usize,
	pub hidden_size: usize,
	num_hidden_layers: usize,
	num_attention_heads: usize,
	intermediate_size: usize,
	pub hidden_act: HiddenAct,
	hidden_dropout_prob: f64,
	max_position_embeddings: usize,
	type_vocab_size: usize,
	initializer_range: f64,
	layer_norm_eps: f64,
	pad_token_id: usize,
	#[serde(default)]
	position_embedding_type: PositionEmbeddingType,
	#[serde(default)]
	use_cache: bool,
	pub classifier_dropout: Option<f64>,
	model_type: Option<String>,
	problem_type: Option<String>,
	pub _num_labels: Option<usize>,
	pub id2label: Option<HashMap<String, String>>,
	pub label2id: Option<HashMap<String, usize>>,
}

impl Default for BertConfig {
	fn default() -> Self {
		Self {
			vocab_size: 30522,
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
			pad_token_id: 0,
			position_embedding_type: PositionEmbeddingType::Absolute,
			use_cache: true,
			classifier_dropout: None,
			model_type: Some("bert".to_string()),
			problem_type: None,
			_num_labels: Some(9),
			id2label: None,
			label2id: None,
		}
	}
}

impl BertConfig {
	fn _all_mini_lm_l6_v2() -> Self {
		Self {
			vocab_size: 30522,
			hidden_size: 384,
			num_hidden_layers: 6,
			num_attention_heads: 12,
			intermediate_size: 1536,
			hidden_act: HiddenAct::Gelu,
			hidden_dropout_prob: 0.1,
			max_position_embeddings: 512,
			type_vocab_size: 2,
			initializer_range: 0.02,
			layer_norm_eps: 1e-12,
			pad_token_id: 0,
			position_embedding_type: PositionEmbeddingType::Absolute,
			use_cache: true,
			classifier_dropout: None,
			model_type: Some("bert".to_string()),
			problem_type: None,
			_num_labels: None,
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
		0 =>
			for i in 0..rows {
				for j in 1..cols {
					result[i][j] += result[i][j - 1];
				}
			},
		1 =>
			for j in 0..cols {
				for i in 1..rows {
					result[i][j] += result[i - 1][j];
				}
			},
		_ => panic!("Dimension not supported"),
	}
	let result = Tensor::new(result, &device)?;
	Ok(result)
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

struct BertEmbeddings {
	word_embeddings: Embedding,
	position_embeddings: Option<Embedding>,
	token_type_embeddings: Embedding,
	layer_norm: LayerNorm,
	dropout: Dropout,
	pub padding_idx: u32,
	span: tracing::Span,
}

impl BertEmbeddings {
	fn load(vb: VarBuilder, config: &BertConfig) -> Result<Self> {
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
			span: tracing::span!(tracing::Level::TRACE, "embeddings"),
		})
	}

	fn forward(
		&self,
		input_ids: &Tensor,
		token_type_ids: &Tensor,
		position_ids: Option<&Tensor>,
		inputs_embeds: Option<&Tensor>,
	) -> Result<Tensor> {
		let _enter = self.span.enter();
		let position_ids = match position_ids {
			Some(ids) => ids.to_owned(),
			None =>
				if Option::is_some(&inputs_embeds) {
					self.create_position_ids_from_input_embeds(inputs_embeds.unwrap())?
				} else {
					create_position_ids_from_input_ids(input_ids, self.padding_idx, 1)?
				},
		};
		let inputs_embeds: Tensor = match inputs_embeds {
			Some(embeds) => embeds.to_owned(),
			None => self.word_embeddings.forward(input_ids)?,
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

	fn create_position_ids_from_input_embeds(&self, input_embeds: &Tensor) -> Result<Tensor> {
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

struct BertSelfAttention {
	query: Linear,
	key: Linear,
	value: Linear,
	dropout: Dropout,
	num_attention_heads: usize,
	attention_head_size: usize,
	span: tracing::Span,
	span_softmax: tracing::Span,
	attention_probs: RwLock<Option<Tensor>>,
}

impl BertSelfAttention {
	fn load(vb: VarBuilder, config: &BertConfig) -> Result<Self> {
		let attention_head_size = config.hidden_size / config.num_attention_heads;
		let all_head_size = config.num_attention_heads * attention_head_size;
		let query = linear(config.hidden_size, all_head_size, vb.pp("query"))?;
		let key = linear(config.hidden_size, all_head_size, vb.pp("key"))?;
		let value = linear(config.hidden_size, all_head_size, vb.pp("value"))?;
		Ok(Self {
			query,
			key,
			value,
			dropout: Dropout::new(config.hidden_dropout_prob),
			num_attention_heads: config.num_attention_heads,
			attention_head_size,
			span: tracing::span!(tracing::Level::TRACE, "self-attn"),
			span_softmax: tracing::span!(tracing::Level::TRACE, "softmax"),
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
		let _enter = self.span.enter();
		let query_layer = self.query.forward(hidden_states)?;
		let key_layer = self.key.forward(hidden_states)?;
		let value_layer = self.value.forward(hidden_states)?;
		let query_layer = self.transpose_for_scores(&query_layer)?;
		let key_layer = self.transpose_for_scores(&key_layer)?;
		let value_layer = self.transpose_for_scores(&value_layer)?;
		let attention_scores = query_layer.matmul(&key_layer.t()?)?;
		let attention_scores = (attention_scores / (self.attention_head_size as f64).sqrt())?;
		let attention_probs = {
			let _enter_sm = self.span_softmax.enter();
			candle_nn::ops::softmax(&attention_scores, candle_core::D::Minus1)?
		};
		let attention_probs = self.dropout.forward(&attention_probs)?;
		*self.attention_probs.write().unwrap() = Some(attention_probs.clone());
		let context_layer = attention_probs.matmul(&value_layer)?;
		let context_layer = context_layer.transpose(1, 2)?.contiguous()?;
		let context_layer = context_layer.flatten_from(candle_core::D::Minus2)?;
		Ok(context_layer)
	}
}

struct BertSelfOutput {
	dense: Linear,
	layer_norm: LayerNorm,
	dropout: Dropout,
	span: tracing::Span,
}

impl BertSelfOutput {
	fn load(vb: VarBuilder, config: &BertConfig) -> Result<Self> {
		let dense = linear(config.hidden_size, config.hidden_size, vb.pp("dense"))?;
		let layer_norm = layer_norm(config.hidden_size, config.layer_norm_eps, vb.pp("LayerNorm"))?;
		let dropout = Dropout::new(config.hidden_dropout_prob);
		Ok(Self {
			dense,
			layer_norm,
			dropout,
			span: tracing::span!(tracing::Level::TRACE, "self-out"),
		})
	}

	fn forward(&self, hidden_states: &Tensor, input_tensor: &Tensor) -> Result<Tensor> {
		let _enter = self.span.enter();
		let hidden_states = self.dense.forward(hidden_states)?;
		let hidden_states = self.dropout.forward(&hidden_states)?;
		self.layer_norm.forward(&(hidden_states + input_tensor)?)
	}
}

struct BertAttention {
	self_attention: BertSelfAttention,
	self_output: BertSelfOutput,
	span: tracing::Span,
}

impl BertAttention {
	fn load(vb: VarBuilder, config: &BertConfig) -> Result<Self> {
		let self_attention = BertSelfAttention::load(vb.pp("self"), config)?;
		let self_output = BertSelfOutput::load(vb.pp("output"), config)?;
		Ok(Self {
			self_attention,
			self_output,
			span: tracing::span!(tracing::Level::TRACE, "attn"),
		})
	}

	fn forward(&self, hidden_states: &Tensor) -> Result<Tensor> {
		let _enter = self.span.enter();
		let self_outputs = self.self_attention.forward(hidden_states)?;
		let attention_output = self.self_output.forward(&self_outputs, hidden_states)?;
		Ok(attention_output)
	}
}

struct BertIntermediate {
	dense: Linear,
	intermediate_act: HiddenActLayer,
	span: tracing::Span,
}

impl BertIntermediate {
	fn load(vb: VarBuilder, config: &BertConfig) -> Result<Self> {
		let dense = linear(config.hidden_size, config.intermediate_size, vb.pp("dense"))?;
		Ok(Self {
			dense,
			intermediate_act: HiddenActLayer::new(config.hidden_act),
			span: tracing::span!(tracing::Level::TRACE, "inter"),
		})
	}

	fn forward(&self, hidden_states: &Tensor) -> Result<Tensor> {
		let _enter = self.span.enter();
		let hidden_states = self.dense.forward(hidden_states)?;
		let ys = self.intermediate_act.forward(&hidden_states)?;
		Ok(ys)
	}
}

struct BertOutput {
	dense: Linear,
	layer_norm: LayerNorm,
	dropout: Dropout,
	span: tracing::Span,
}

impl BertOutput {
	fn load(vb: VarBuilder, config: &BertConfig) -> Result<Self> {
		let dense = linear(config.intermediate_size, config.hidden_size, vb.pp("dense"))?;
		let layer_norm = layer_norm(config.hidden_size, config.layer_norm_eps, vb.pp("LayerNorm"))?;
		let dropout = Dropout::new(config.hidden_dropout_prob);
		Ok(Self { dense, layer_norm, dropout, span: tracing::span!(tracing::Level::TRACE, "out") })
	}

	fn forward(&self, hidden_states: &Tensor, input_tensor: &Tensor) -> Result<Tensor> {
		let _enter = self.span.enter();
		let hidden_states = self.dense.forward(hidden_states)?;
		let hidden_states = self.dropout.forward(&hidden_states)?;
		self.layer_norm.forward(&(hidden_states + input_tensor)?)
	}
}

struct BertLayer {
	attention: BertAttention,
	intermediate: BertIntermediate,
	output: BertOutput,
	span: tracing::Span,
}

impl BertLayer {
	fn load(vb: VarBuilder, config: &BertConfig) -> Result<Self> {
		let attention = BertAttention::load(vb.pp("attention"), config)?;
		let intermediate = BertIntermediate::load(vb.pp("intermediate"), config)?;
		let output = BertOutput::load(vb.pp("output"), config)?;
		Ok(Self {
			attention,
			intermediate,
			output,
			span: tracing::span!(tracing::Level::TRACE, "layer"),
		})
	}

	fn forward(&self, hidden_states: &Tensor) -> Result<Tensor> {
		let _enter = self.span.enter();
		let attention_output = self.attention.forward(hidden_states)?;
		let intermediate_output = self.intermediate.forward(&attention_output)?;
		let layer_output = self.output.forward(&intermediate_output, &attention_output)?;
		Ok(layer_output)
	}
}

struct BertEncoder {
	layers: Vec<BertLayer>,
	span: tracing::Span,
}

impl BertEncoder {
	fn load(vb: VarBuilder, config: &BertConfig) -> Result<Self> {
		let layers = (0..config.num_hidden_layers)
			.map(|index| BertLayer::load(vb.pp(&format!("layer.{index}")), config))
			.collect::<Result<Vec<_>>>()?;
		let span = tracing::span!(tracing::Level::TRACE, "encoder");
		Ok(BertEncoder { layers, span })
	}

	fn forward(&self, hidden_states: &Tensor) -> Result<Tensor> {
		let _enter = self.span.enter();
		let mut hidden_states = hidden_states.clone();
		for layer in self.layers.iter() {
			hidden_states = layer.forward(&hidden_states)?;
		}
		Ok(hidden_states)
	}
}

pub struct BertModel {
	embeddings: BertEmbeddings,
	encoder: BertEncoder,
	pub device: Device,
	span: tracing::Span,
}

impl BertModel {
	pub fn load(vb: VarBuilder, config: &BertConfig) -> Result<Self> {
		let (embeddings, encoder) = match (
			BertEmbeddings::load(vb.pp("embeddings"), config),
			BertEncoder::load(vb.pp("encoder"), config),
		) {
			(Ok(embeddings), Ok(encoder)) => (embeddings, encoder),
			(Err(err), _) | (_, Err(err)) =>
				if let Some(model_type) = &config.model_type {
					if let (Ok(embeddings), Ok(encoder)) = (
						BertEmbeddings::load(vb.pp(&format!("{model_type}.embeddings")), config),
						BertEncoder::load(vb.pp(&format!("{model_type}.encoder")), config),
					) {
						(embeddings, encoder)
					} else {
						return Err(err);
					}
				} else {
					return Err(err);
				},
		};
		Ok(Self {
			embeddings,
			encoder,
			device: vb.device().clone(),
			span: tracing::span!(tracing::Level::TRACE, "model"),
		})
	}

	pub fn forward(&self, input_ids: &Tensor, token_type_ids: &Tensor) -> Result<Tensor> {
		let _enter = self.span.enter();
		let embedding_output = self.embeddings.forward(input_ids, token_type_ids, None, None)?;
		let sequence_output = self.encoder.forward(&embedding_output)?;
		Ok(sequence_output)
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
}

pub struct BertForTokenClassification {
	bert: BertModel,
	dropout: Dropout,
	classifier: Linear,
	pub device: Device,
	pub config: BertConfig,
}

impl BertForTokenClassification {
	pub fn load(vb: VarBuilder, config: &BertConfig) -> Result<Self> {
		let classifier_dropout = config.classifier_dropout;
		let (bert, classifier) = match (
			BertModel::load(vb.pp("bert"), config),
			if let Some(num_labels) = config._num_labels {
				linear(config.hidden_size, num_labels, vb.pp("classifier"))
			} else if let Some(id2label) = &config.id2label {
				linear(config.hidden_size, id2label.len(), vb.pp("classifier"))
			} else {
				candle_core::bail!("cannot find the number of classes to map to")
			},
		) {
			(Ok(bert), Ok(classifier)) => (bert, classifier),
			(Err(err), _) | (_, Err(err)) => return Err(err),
		};
		Ok(Self {
			bert,
			dropout: Dropout::new(classifier_dropout.unwrap_or(0.2)),
			classifier,
			device: vb.device().clone(),
			config: config.clone(),
		})
	}

	pub fn forward(
		&self,
		input_ids: &Tensor,
		token_type_ids: &Tensor,
		labels: Option<&Tensor>,
	) -> Result<TokenClassifierOutput> {
		let outputs = self.bert.forward(input_ids, token_type_ids)?;
		let outputs = self.dropout.forward(&outputs)?;
		let logits = self.classifier.forward(&outputs)?;
		let mut loss: Tensor = Tensor::new(vec![0.0], &self.device)?;
		if let Some(labels) = labels {
			loss = candle_nn::loss::cross_entropy(&logits.flatten_to(1)?, &labels.flatten_to(1)?)?;
		}
		Ok(TokenClassifierOutput {
			loss: Some(loss),
			logits,
			hidden_states: None,
			attentions: None,
		})
	}
}
