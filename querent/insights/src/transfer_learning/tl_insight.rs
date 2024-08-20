use crate::{
	ConfigCallbackResponse, Insight, InsightConfig, InsightError, InsightErrorKind, InsightInfo,
	InsightResult, InsightRunner,
};
use async_trait::async_trait;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

/// XAI Insight struct.
pub struct TLV1 {
	info: InsightInfo,
}

impl TLV1 {
	pub fn new() -> Self {
		let additional_options = HashMap::new();
		Self {
			info: InsightInfo {
				id: "querent.insights.tl.tlv1".to_string(),
				name: "Querent Transfer Learning".to_string(),
				description: "Transfer Learning is a research problem in machine learning that focuses on storing knowledge gained while solving one problem and applying it to a different but related problem.".to_string(),
				version: "0.0.1-dev".to_string(),
				author: "Querent AI".to_string(),
				license: "BSL-1.0".to_string(),
				iconify_icon: "solar:text-field-focus-broken".to_string(),
				additional_options,
				conversational: false,
				premium: true,
			},
		}
	}
}

#[async_trait]
impl Insight for TLV1 {
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
