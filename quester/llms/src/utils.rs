use anyhow::{anyhow, Error as E, Result};
use candle_core::Device;
use tokenizers::Tokenizer;
use candle_nn::VarBuilder;

use hf_hub::{api::sync::Api, Cache, Repo, RepoType};


use crate::transformers::roberta::{RobertaModel, RobertaConfig,FLOATING_DTYPE};
use crate::transformers::roberta::{RobertaForSequenceClassification, RobertaForTokenClassification, RobertaForQuestionAnswering};

pub enum ModelType {
    RobertaModel {model: RobertaModel},
    RobertaForSequenceClassification {model: RobertaForSequenceClassification},
    RobertaForTokenClassification {model: RobertaForTokenClassification},
    RobertaForQuestionAnswering {model: RobertaForQuestionAnswering},
}

pub fn round_to_decimal_places(n: f32, places: u32) -> f32 {
    let multiplier: f32 = 10f32.powi(places as i32);
    (n * multiplier).round() / multiplier
}

pub fn build_roberta_model_and_tokenizer(model_name_or_path: impl Into<String>, offline: bool, model_type: &str) -> Result<(ModelType, Tokenizer)> {
    let device = Device::Cpu;
    let (model_id, revision) = (model_name_or_path.into(), "main".to_string());
    let repo = Repo::with_revision(model_id, RepoType::Model, revision);

    let (config_filename, tokenizer_filename, weights_filename) = if offline {
        let cache = Cache::default().repo(repo);
        (
            cache
                .get("config.json")
                .ok_or(anyhow!("Missing config file in cache"))?,
            cache
                .get("tokenizer.json")
                .ok_or(anyhow!("Missing tokenizer file in cache"))?,
            cache
                .get("model.safetensors")
                .ok_or(anyhow!("Missing weights file in cache"))?,
        )
    } else {
        let api = Api::new()?;
        let api = api.repo(repo);
        (
            api.get("config.json")?,
            api.get("tokenizer.json")?,
            api.get("model.safetensors")?,
        )
    };

    println!("config_filename: {}", config_filename.display());
    println!("tokenizer_filename: {}", tokenizer_filename.display());
    println!("weights_filename: {}", weights_filename.display());


    let config = std::fs::read_to_string(config_filename)?;
    let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(E::msg)?;

    let vb =
        unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], FLOATING_DTYPE, &device)? };

    let model = match model_type {
        "RobertaModel" => {
            let config: RobertaConfig = serde_json::from_str(&config)?;
            let model = RobertaModel::load(vb, &config)?;
            ModelType::RobertaModel {model}
        }
        "RobertaForSequenceClassification" => {
            let config: RobertaConfig = serde_json::from_str(&config)?;
            let model = RobertaForSequenceClassification::load(vb, &config)?;
            ModelType::RobertaForSequenceClassification {model}
        }
        "RobertaForTokenClassification" => {
            let config: RobertaConfig = serde_json::from_str(&config)?;
            let model = RobertaForTokenClassification::load(vb, &config)?;
            ModelType::RobertaForTokenClassification {model}
        }
        "RobertaForQuestionAnswering" => {
            let config: RobertaConfig = serde_json::from_str(&config)?;
            let model = RobertaForQuestionAnswering::load(vb, &config)?;
            ModelType::RobertaForQuestionAnswering {model}
        }
        
        _ => panic!("Invalid model_type")
    };

    Ok((model, tokenizer))
}