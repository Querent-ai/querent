use crate::{
	ConfigCallbackResponse, CustomInsightOption, Insight, InsightConfig, InsightCustomOptionValue,
	InsightError, InsightErrorKind, InsightInfo, InsightResult, InsightRunner,
};
use async_trait::async_trait;
use common::get_querent_data_path;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use llms::Claude;
use serde_json::Value;
use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

use super::x_ai_runner::XAIRunner;
/// XAI Insight struct.
pub struct XAIClaude {
	info: InsightInfo,
}

impl XAIClaude {
	pub fn new() -> Self {
		let mut additional_options = HashMap::new();
		additional_options.insert(
			"claude_api_key".to_string(),
			CustomInsightOption {
				id: "claude_api_key".to_string(),
				label: "Calude API Key".to_string(),
				default_value: Some(InsightCustomOptionValue::String {
					value: "".to_string(),
					hidden: Some(false),
				}),
				value: InsightCustomOptionValue::String {
					value: "".to_string(),
					hidden: Some(false),
				},
				tooltip: Some("Claude API Key".to_string()),
			},
		);
		additional_options.insert(
			"prompt".to_string(),
			CustomInsightOption {
				id: "prompt".to_string(),
				label: "Prompt".to_string(),
				default_value: Some(InsightCustomOptionValue::String {
					value: "".to_string(),
					hidden: Some(false),
				}),
				value: InsightCustomOptionValue::String {
					value: "You are a knowledgeable assistant responsible for generating a comprehensive summary of the data provided below. \
        Given below is a user query and its cosine based similarity results, which have sentences from various documents. \
        Please concatenate all of these into a single, comprehensive description that answers the user's query making sure to use information collected from all the sentences. \
        If the results are contradictory, please resolve the contradictions and provide a single, coherent summary (approximately 300 - 500 words). \
        Make sure it is written in third person, and make sure we have the full context.".to_string(),
					hidden: Some(false),
				},
				tooltip: Some("Custom prompt for generating insights. If provided, this prompt will be used instead of the default prompt to generate a summary of results using Calude AI. This free tool does not parse the actual documents but materializes responses using data fabric as a guardrails for llm.".to_string()),

			},
		);
		Self {
			info: InsightInfo {
				id: "querent.insights.x_ai.claude".to_string(),
				name: "Querent xAI with Claude3pus20240229".to_string(),
				description: "xAI utilizes generative models to perform a directed traversal in R!AN's attention data fabric.".to_string(),
				version: "1.0.0".to_string(),
				author: "Querent AI".to_string(),
				license: "Apache-2.0".to_string(),
				iconify_icon: "game-icons:laser-burst".to_string(),
				additional_options,
				conversational: true,
				premium: false,
			},
		}
	}
}

#[async_trait]
impl Insight for XAIClaude {
	async fn info(&self) -> InsightInfo {
		self.info.clone()
	}

	fn supports_streaming(&self) -> bool {
		true
	}

	fn config_callback(&mut self, _name: &str, _config: Value) -> ConfigCallbackResponse {
		ConfigCallbackResponse::Empty
	}

	fn get_runner(&self, config: &InsightConfig) -> InsightResult<Arc<dyn InsightRunner>> {
		let claude_api_key = config.get_custom_option("claude_api_key");
		if claude_api_key.is_none() {
			return Err(InsightError::new(
				InsightErrorKind::Unauthorized,
				anyhow::anyhow!("Claude API Key is required").into(),
			));
		}
		let claude_api_key = claude_api_key.unwrap().value.clone();
		let claude_api_key = match claude_api_key {
			InsightCustomOptionValue::String { value, .. } => value,
			_ => {
				return Err(InsightError::new(
					InsightErrorKind::Unauthorized,
					anyhow::anyhow!("Claude API Key is required").into(),
				));
			},
		};
		let claude_llm = Claude::new().with_api_key(claude_api_key.clone());

		let prompt_option = config.get_custom_option("prompt");
		if prompt_option.is_none() {
			tracing::info!("No prompt provided.");
		}
		let prompt = prompt_option.map_or_else(
			|| "".to_string(),
			|opt| match opt.value.clone() {
				InsightCustomOptionValue::String { value, .. } => value,
				_ => "".to_string(),
			},
		);
		let model_details = InitOptions::new(EmbeddingModel::AllMiniLML6V2)
			.with_cache_dir(get_querent_data_path())
			.with_show_download_progress(true);
		let embedding_model = TextEmbedding::try_new(model_details)
			.map_err(|e| InsightError::new(InsightErrorKind::Internal, e.into()))?;

		Ok(Arc::new(XAIRunner {
			config: config.clone(),
			llm: Arc::new(claude_llm),
			embedding_model: Some(embedding_model),
			previous_query_results: RwLock::new(String::new()),
			previous_filtered_results: RwLock::new(Vec::new()),
			previous_session_id: RwLock::new(String::new()),
			prompt,
		}))
	}
}
