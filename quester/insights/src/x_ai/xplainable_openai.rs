use crate::{
	ConfigCallbackResponse, CustomInsightOption, Insight, InsightConfig, InsightCustomOptionValue,
	InsightError, InsightErrorKind, InsightInfo, InsightInput, InsightOutput, InsightResult,
	InsightRunner,
};
use async_stream::stream;
use async_trait::async_trait;
use futures::{pin_mut, Stream, StreamExt};
use llms::{OpenAI, OpenAIConfig, LLM};
use serde_json::Value;
use std::{collections::HashMap, pin::Pin, sync::Arc};

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
		Self {
			info: InsightInfo {
				id: "querent.insights.x_ai.openai".to_string(),
				name: "Querent xAI with GPT".to_string(),
				description: "xAI utilizes generative models to perform a directed traversal in Querent's attention data fabric.".to_string(),
				version: "1.0.0".to_string(),
				author: "Querent AI".to_string(),
				license: "Apache-2.0".to_string(),
				icon: &[], // Add your icon bytes here.
				additional_options,
				conversational: true,
			},
		}
	}
}

pub struct XAIRunner {
	pub config: InsightConfig,
	pub llm: Arc<dyn LLM>,
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
			_ =>
				return Err(InsightError::new(
					InsightErrorKind::Unauthorized,
					anyhow::anyhow!("OpenAI API Key is required").into(),
				)),
		};
		let default_openai_config: OpenAIConfig =
			OpenAIConfig::default().with_api_key(openai_api_key);
		let openai_llm = OpenAI::new(default_openai_config);
		Ok(Arc::new(XAIRunner { config: config.clone(), llm: Arc::new(openai_llm) }))
	}
}

#[async_trait]
impl InsightRunner for XAIRunner {
	async fn run(&self, input: InsightInput) -> InsightResult<InsightOutput> {
		// Placeholder explanation logic.
		let explanation = format!("Explanation for input: {:?}", input.data);
		Ok(InsightOutput { data: Value::String(explanation) })
	}

	async fn run_stream(
		&self,
		input: Pin<Box<dyn Stream<Item = InsightInput> + Send + 'static>>,
	) -> InsightResult<Pin<Box<dyn Stream<Item = InsightResult<InsightOutput>> + Send + 'static>>>
	{
		let stream = stream! {
			pin_mut!(input);
			while let Some(input) = input.next().await {
				log::info!("Processing input: {:?}", input.data);
				let output = InsightOutput {
					data: Value::String(format!("Explanation for input: {:?}", input.data)),
				};
				yield Ok(output);
			}
		};
		Ok(Box::pin(stream))
	}
}
