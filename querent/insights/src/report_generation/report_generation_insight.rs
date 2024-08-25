use crate::{
	ConfigCallbackResponse, Insight, InsightConfig, InsightError, InsightErrorKind, InsightInfo,
	InsightResult, InsightRunner,
};
use async_trait::async_trait;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

/// XAI Insight struct.
pub struct RGV1 {
	info: InsightInfo,
}

impl RGV1 {
	pub fn new() -> Self {
		let additional_options = HashMap::new();
		Self {
			info: InsightInfo {
				id: "querent.insights.rg.rgv1".to_string(),
				name: "Querent Report Generation".to_string(),
				description: "Report generation processes information from the semantic data fabric to produce comprehensive reports that highlight key insights, trends, and patterns across the data landscape.".to_string(),
				version: "0.0.1-dev".to_string(),
				author: "Querent AI".to_string(),
				license: "BSL-1.0".to_string(),
				iconify_icon: "carbon:report".to_string(),
				additional_options,
				conversational: false,
				premium: true,
			},
		}
	}
}

#[async_trait]
impl Insight for RGV1 {
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
