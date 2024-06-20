use std::path::Path;

use anyhow::{anyhow, Result};
use candle_core::{DType, Device};
use candle_nn::VarBuilder;
use tokenizers::Tokenizer;

use hf_hub::{api::sync::Api, Repo, RepoType};

use crate::transformers::{
	bert::bert_model_functions::{BertConfig, BertForTokenClassification},
	roberta::roberta_model_functions::{
		RobertaConfig, RobertaForQuestionAnswering, RobertaForSequenceClassification,
		RobertaForTokenClassification, RobertaModel,
	},
};

pub enum ModelType {
	RobertaModel { model: RobertaModel },
	RobertaForSequenceClassification { model: RobertaForSequenceClassification },
	RobertaForTokenClassification { model: RobertaForTokenClassification },
	RobertaForQuestionAnswering { model: RobertaForQuestionAnswering },
	BertForTokenClassification { model: BertForTokenClassification },
}

pub fn round_to_decimal_places(n: f32, places: u32) -> f32 {
	let multiplier: f32 = 10f32.powi(places as i32);
	(n * multiplier).round() / multiplier
}

pub fn build_roberta_model_and_tokenizer(
	model_name_or_path: impl Into<String>,
	offline: bool,
	model_type: &str,
) -> Result<(ModelType, Tokenizer)> {
	let device = Device::Cpu;
	let model_dir = model_name_or_path.into();

	let (config_filename, tokenizer_filename, weights_filename) = if offline {
		let config_filename = Path::new(&model_dir).join("config.json");
		let tokenizer_filename = Path::new(&model_dir).join("tokenizer.json");
		let weights_filename = Path::new(&model_dir).join("model.safetensors");

		if !config_filename.exists() {
			return Err(anyhow!("Missing config file in directory"));
		}
		if !tokenizer_filename.exists() {
			return Err(anyhow!("Missing tokenizer file in directory"));
		}
		if !weights_filename.exists() {
			return Err(anyhow!("Missing weights file in directory"));
		}

		(config_filename, tokenizer_filename, weights_filename)
	} else {
		let (model_id, revision) = (model_dir, "main".to_string());
		let repo = Repo::with_revision(model_id, RepoType::Model, revision);
		let api = Api::new()?;
		let api = api.repo(repo);

		let tokenizer_filename =
			api.get("tokenizer.json").map_err(|_| anyhow!("Missing tokenizer file"))?;
		(api.get("config.json")?, tokenizer_filename, api.get("model.safetensors")?)
	};

	let config_content = std::fs::read_to_string(&config_filename)
		.map_err(|e| anyhow!("Failed to read config file: {}", e))?;

	let config: Result<Box<dyn std::any::Any>, _> = match model_type {
		"BertForTokenClassification" => serde_json::from_str::<BertConfig>(&config_content)
			.map(|cfg| Box::new(cfg) as Box<dyn std::any::Any>),
		_ => serde_json::from_str::<RobertaConfig>(&config_content)
			.map(|cfg| Box::new(cfg) as Box<dyn std::any::Any>),
	}
	.map_err(|e| anyhow!("Failed to parse config JSON: {}", e));

	let config = config?;

	// Load the tokenizer
	let tokenizer = Tokenizer::from_file(&tokenizer_filename)
		.map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

	let vb =
		unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], DType::F32, &device)? };

	let model = match model_type {
		"RobertaModel" => {
			let config = config.downcast_ref::<RobertaConfig>().unwrap();
			let model = RobertaModel::load(vb, config)?;
			ModelType::RobertaModel { model }
		},
		"RobertaForSequenceClassification" => {
			let config = config.downcast_ref::<RobertaConfig>().unwrap();
			let model = RobertaForSequenceClassification::load(vb, config)?;
			ModelType::RobertaForSequenceClassification { model }
		},
		"RobertaForTokenClassification" => {
			let config = config.downcast_ref::<RobertaConfig>().unwrap();
			let model = RobertaForTokenClassification::load(vb, config)?;
			ModelType::RobertaForTokenClassification { model }
		},
		"BertForTokenClassification" => {
			let config = config.downcast_ref::<BertConfig>().unwrap();
			let model = BertForTokenClassification::load(vb, config)?;
			ModelType::BertForTokenClassification { model }
		},
		"RobertaForQuestionAnswering" => {
			let config = config.downcast_ref::<RobertaConfig>().unwrap();
			let model = RobertaForQuestionAnswering::load(vb, config)?;
			ModelType::RobertaForQuestionAnswering { model }
		},
		_ => return Err(anyhow!("Invalid model_type: {}", model_type)),
	};

	Ok((model, tokenizer))
}
