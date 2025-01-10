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

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use crate::{
	ConfigCallbackResponse, CustomInsightOption, Insight, InsightConfig, InsightCustomOptionValue,
	InsightError, InsightErrorKind, InsightInfo, InsightResult, InsightRunner,
};
use async_trait::async_trait;
use common::get_querent_data_path;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use llms::{OpenAI, OpenAIConfig};
use serde_json::Value;
use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

use super::x_ai_runner::XAIRunner;
/// XAI Insight struct.
pub struct XAI {
	info: InsightInfo,
}

impl XAI {
	pub fn new() -> Self {
		let mut additional_options = HashMap::new();
		additional_options.insert(
			"openai_api_key".to_string(),
			CustomInsightOption {
				id: "openai_api_key".to_string(),
				label: "OpenAI API Key".to_string(),
				default_value: Some(InsightCustomOptionValue::String {
					value: "".to_string(),
					hidden: Some(false),
				}),
				value: InsightCustomOptionValue::String {
					value: "".to_string(),
					hidden: Some(false),
				},
				tooltip: Some("OpenAI API Key".to_string()),
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
        Given below is a user query and its graph traversal results, which have sentences from various documents along with the entities identified in the sentence. \
        Please concatenate all of these into a single, comprehensive description that answers the user's query making sure to use information collected from all the sentences. \
        If the provided traversal results are contradictory, please resolve the contradictions and provide a single, coherent summary. \
        Make sure it is written in third person, and make sure we have the full context.".to_string(),
					hidden: Some(false),
				},
				tooltip: Some("Custom prompt for generating insights. If provided, this prompt will be used instead of the default prompt to generate a summary of results using OpenAI.".to_string()),

			},
		);
		Self {
			info: InsightInfo {
				id: "querent.insights.x_ai.openai".to_string(),
				name: "Querent xAI with GPT35 Turbo".to_string(),
				description: "xAI utilizes generative models to perform a directed traversal in R!AN's attention data fabric. This free tool does not parse the actual documents but materializes responses using data fabric as a guardrails for llm. ".to_string(),
				version: "1.0.0".to_string(),
				author: "Querent AI".to_string(),
				license: "Apache-2.0".to_string(),
				iconify_icon: "ri:openai-fill".to_string(),
				additional_options,
				conversational: true,
				premium: false,
			},
		}
	}
}

#[async_trait]
impl Insight for XAI {
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
		let openai_api_key = config.get_custom_option("openai_api_key");
		if openai_api_key.is_none() {
			return Err(InsightError::new(
				InsightErrorKind::Unauthorized,
				anyhow::anyhow!("OpenAI API Key is required").into(),
			));
		}
		let openai_api_key = openai_api_key.unwrap().value.clone();
		let openai_api_key = match openai_api_key {
			InsightCustomOptionValue::String { value, .. } => value,
			_ => {
				return Err(InsightError::new(
					InsightErrorKind::Unauthorized,
					anyhow::anyhow!("OpenAI API Key is required").into(),
				));
			},
		};
		let default_openai_config: OpenAIConfig =
			OpenAIConfig::default().with_api_key(openai_api_key);
		let openai_llm = OpenAI::new(default_openai_config);

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
			llm: Arc::new(openai_llm),
			embedding_model: Some(embedding_model),
			previous_query_results: RwLock::new(String::new()),
			previous_filtered_results: RwLock::new(Vec::new()),
			previous_session_id: RwLock::new(String::new()),
			prompt,
		}))
	}
}
