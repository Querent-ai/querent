use crate::{
	ConfigCallbackResponse, Insight, InsightConfig, InsightError, InsightErrorKind, InsightInfo,
	InsightResult, InsightRunner,
};
use async_trait::async_trait;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

/// XAI Insight struct.
pub struct CDSV1 {
	info: InsightInfo,
}

impl CDSV1 {
	pub fn new() -> Self {
		let additional_options = HashMap::new();
		Self {
			info: InsightInfo {
				id: "querent.insights.cds.cdsv1".to_string(),
				name: "Querent Cross Document Summarization".to_string(),
				description: "Cross Document Summarization involves processing information from multiple documents to create a cohesive summary, capturing key insights and themes across diverse sources.".to_string(),
				version: "0.0.1-dev".to_string(),
				author: "Querent AI".to_string(),
				license: "BSL-1.0".to_string(),
				iconify_icon: "fluent:document-bullet-list-cube-24-regular".to_string(),
				additional_options,
				conversational: false,
				premium: true,
			},
		}
	}
}

#[async_trait]
impl Insight for CDSV1 {
	async fn info(&self) -> InsightInfo {
		self.info.clone()
	}

	fn supports_streaming(&self) -> bool {
		true
	}

	fn config_callback(&mut self, _name: &str, _config: Value) -> ConfigCallbackResponse {
		ConfigCallbackResponse::Empty
	}

	fn get_runner(&self, _config: &InsightConfig) -> InsightResult<Arc<dyn InsightRunner>> {
		Err(InsightError::new(
			InsightErrorKind::NotFound,
			anyhow::anyhow!("Not Implemented!").into(),
		))
	}
}
