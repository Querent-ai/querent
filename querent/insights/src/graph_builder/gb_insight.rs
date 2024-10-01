use crate::{
	ConfigCallbackResponse, CustomInsightOption, Insight, InsightConfig, InsightCustomOptionValue, InsightError, InsightErrorKind, InsightInfo, InsightResult, InsightRunner
};
use async_trait::async_trait;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

use super::gb_runner::GraphBuilderRunner;

/// Graph Builder Insight struct.
pub struct GBV1 {
	info: InsightInfo,
}

impl GBV1 {
	pub fn new() -> Self {
		let mut additional_options = HashMap::new();
		additional_options.insert(
			"neo4j_instance_url".to_string(),
			CustomInsightOption {
				id: "neo4j_instance_url".to_string(),
				label: "Neo4j Instance URL".to_string(),
				default_value: Some(InsightCustomOptionValue::String {
					value: "bolt://localhost:7687".to_string(),
					hidden: Some(false),
				}),
				value: InsightCustomOptionValue::String {
					value: "bolt://localhost:7687".to_string(),
					hidden: Some(false),
				},
				tooltip: Some("The URL of your Neo4j instance (e.g., bolt://localhost:7687)".to_string()),
			},
		);
		additional_options.insert(
			"neo4j_username".to_string(),
			CustomInsightOption {
				id: "neo4j_username".to_string(),
				label: "Neo4j Username".to_string(),
				default_value: Some(InsightCustomOptionValue::String {
					value: "".to_string(),
					hidden: Some(false),
				}),
				value: InsightCustomOptionValue::String {
					value: "".to_string(),
					hidden: Some(false),
				},
				tooltip: Some("Your Neo4j username".to_string()),
			},
		);
		additional_options.insert(
			"neo4j_password".to_string(),
			CustomInsightOption {
				id: "neo4j_password".to_string(),
				label: "Neo4j Password".to_string(),
				default_value: Some(InsightCustomOptionValue::String {
					value: "".to_string(),
					hidden: Some(true),
				}),
				value: InsightCustomOptionValue::String {
					value: "".to_string(),
					hidden: Some(true),
				},
				tooltip: Some("Your Neo4j password".to_string()),
			},
		);
		additional_options.insert(
			"neo4j_database".to_string(),
			CustomInsightOption {
				id: "neo4j_database".to_string(),
				label: "Neo4j Database".to_string(),
				default_value: Some(InsightCustomOptionValue::String {
					value: "neo4j".to_string(),
					hidden: Some(false),
				}),
				value: InsightCustomOptionValue::String {
					value: "neo4j".to_string(),
					hidden: Some(false),
				},
				tooltip: Some("The name of the Neo4j database (leave empty for default)".to_string()),
			},
		);
		Self {
			info: InsightInfo {
				id: "querent.insights.graph_builder.gbv1".to_string(),
				name: "Querent Graph Builder".to_string(),
				description: "Graph builder will allow user to traverse the datafabrics and plot the results in a neo4j instance.".to_string(),
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
impl Insight for GBV1 {
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
		println!("Trying to Sytart Runner-----");
		let neo4j_instance_url = config.get_custom_option("neo4j_instance_url");
		if neo4j_instance_url.is_none() {
			return Err(InsightError::new(
				InsightErrorKind::Unauthorized,
				anyhow::anyhow!("Neo4j instance URL is required").into(),
			));
		}
		let neo4j_instance_url = match neo4j_instance_url.unwrap().value.clone() {
			InsightCustomOptionValue::String { value, .. } => value,
			_ => {
				return Err(InsightError::new(
					InsightErrorKind::Unauthorized,
					anyhow::anyhow!("Invalid Neo4j instance URL format").into(),
				));
			}
		};
		let neo4j_username = config.get_custom_option("neo4j_username");
		if neo4j_username.is_none() {
			return Err(InsightError::new(
				InsightErrorKind::Unauthorized,
				anyhow::anyhow!("Neo4j username is required").into(),
			));
		}
		let neo4j_username = match neo4j_username.unwrap().value.clone() {
			InsightCustomOptionValue::String { value, .. } => value,
			_ => {
				return Err(InsightError::new(
					InsightErrorKind::Unauthorized,
					anyhow::anyhow!("Invalid Neo4j username format").into(),
				));
			}
		};
		let neo4j_password = config.get_custom_option("neo4j_password");
		if neo4j_password.is_none() {
			return Err(InsightError::new(
				InsightErrorKind::Unauthorized,
				anyhow::anyhow!("Neo4j password is required").into(),
			));
		}
		let neo4j_password = match neo4j_password.unwrap().value.clone() {
			InsightCustomOptionValue::String { value, .. } => value,
			_ => {
				return Err(InsightError::new(
					InsightErrorKind::Unauthorized,
					anyhow::anyhow!("Invalid Neo4j password format").into(),
				));
			}
		};
		let neo4j_database = config.get_custom_option("neo4j_database");
		let neo4j_database = neo4j_database.map_or_else(
			|| None, // Use None if the database is not provided
			|opt| match opt.value.clone() {
				InsightCustomOptionValue::String { value, .. } => Some(value),
				_ => None,
			},
		);
		println!("Trying to Sytart Runner-----222222222222222");
		let graph_builder_runner = GraphBuilderRunner {
			config: config.clone(),
			embedding_model: None,
			neo4j_instance_url,
			neo4j_username,
			neo4j_password,
			neo4j_database,
		};
		Ok(Arc::new(graph_builder_runner))
	}
	
}
