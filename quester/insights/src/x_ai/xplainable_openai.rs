use crate::{
	ConfigCallbackResponse, CustomInsightOption, Insight, InsightConfig, InsightCustomOptionValue,
	InsightInfo, InsightResult, InsightRunner,
};
use async_stream::stream;
use async_trait::async_trait;
use futures::{pin_mut, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, pin::Pin, sync::Arc};

/// XAI Insight struct.
pub struct XAI {
	info: InsightInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XAIInput {
	pub query: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XAIOutput {
	pub explanation: String,
}

pub struct XAIRunner {
	_config: InsightConfig,
}

#[async_trait]
impl Insight for XAI {
	type Input = XAIInput;
	type Output = XAIOutput;

	async fn new() -> Box<Self> {
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
		Box::new(Self {
			info: InsightInfo {
				id: "querent.insights.x_ai.openai".to_string(),
				name: "Querent xAI with GPT".to_string(),
				description: "xAI utilizes generative models to perform a directed traversal in Querent's attention data fabric.".to_string(),
				version: "1.0.0".to_string(),
				author: "Querent AI".to_string(),
				license: "Apache-2.0".to_string(),
				icon: &[], // Add your icon bytes here.
				additional_options: HashMap::new(),
				conversational: true,
			},
		})
	}

	async fn info(&self) -> InsightInfo {
		self.info.clone()
	}

	fn supports_streaming(&self) -> bool {
		true
	}

	fn config_callback(&mut self, _name: &str, _config: Value) -> ConfigCallbackResponse {
		ConfigCallbackResponse::Empty
	}

	fn get_runner(
		&mut self,
		config: &InsightConfig,
	) -> InsightResult<Arc<dyn InsightRunner<Input = Self::Input, Output = Self::Output>>> {
		Ok(Arc::new(XAIRunner { _config: config.clone() }))
	}
}

#[async_trait]
impl InsightRunner for XAIRunner {
	type Input = XAIInput;
	type Output = XAIOutput;

	async fn run(&self, input: Self::Input) -> InsightResult<Self::Output> {
		// Placeholder explanation logic.
		let explanation = format!("Explanation for input: {:?}", input.query);
		Ok(XAIOutput { explanation })
	}

	async fn run_stream(
		&self,
		input: Pin<Box<dyn Stream<Item = Self::Input> + Send + 'static>>,
	) -> InsightResult<Pin<Box<dyn Stream<Item = InsightResult<Self::Output>> + Send + 'static>>> {
		let stream = stream! {
			pin_mut!(input);
			while let Some(input) = input.next().await {
				log::info!("Processing input: {:?}", input.query);
				let output = XAIOutput {
					explanation: format!("Explanation for input: {:?}", input.query),
				};
				yield Ok(output);
			}
		};

		Ok(Box::pin(stream))
	}
}
