use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, PartialEq, Default, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowConfig {
	pub version: f32,
	pub collectors: Vec<CollectorConfig>,
	pub engines: Vec<EngineConfig>,
}

#[derive(Deserialize, Debug, PartialEq, utoipa::ToSchema, Clone)]
#[serde(deny_unknown_fields)]
pub enum SupportedSources {
	#[serde(rename = "azure")]
	AzureBlob,
	#[serde(rename = "gcs")]
	GCS,
	#[serde(rename = "s3")]
	S3,
	#[serde(rename = "jira")]
	Jira,
	#[serde(rename = "drive")]
	Drive,
	#[serde(rename = "onedrive")]
	OneDrive,
	#[serde(rename = "email")]
	Email,
	#[serde(rename = "dropbox")]
	Dropbox,
	#[serde(rename = "github")]
	Github,
	#[serde(rename = "slack")]
	Slack,
}

impl Into<String> for SupportedSources {
	fn into(self) -> String {
		match self {
			SupportedSources::AzureBlob => "azure".to_string(),
			SupportedSources::GCS => "gcs".to_string(),
			SupportedSources::S3 => "s3".to_string(),
			SupportedSources::Jira => "jira".to_string(),
			SupportedSources::Drive => "drive".to_string(),
			SupportedSources::OneDrive => "onedrive".to_string(),
			SupportedSources::Email => "email".to_string(),
			SupportedSources::Dropbox => "dropbox".to_string(),
			SupportedSources::Github => "github".to_string(),
			SupportedSources::Slack => "slack".to_string(),
		}
	}
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub enum SupportedBackend {
	#[serde(rename = "llama2_v1")]
	KnowledgeGraphLlama2V1,
	#[serde(rename = "openai")]
	KnowledgeGraphOpenAI,
}

impl Into<String> for SupportedBackend {
	fn into(self) -> String {
		match self {
			SupportedBackend::KnowledgeGraphLlama2V1 => "llama2_v1".to_string(),
			SupportedBackend::KnowledgeGraphOpenAI => "openai".to_string(),
		}
	}
}
#[derive(Deserialize, Debug, Eq, PartialEq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EngineConfig {
	pub config: HashMap<String, String>,
	pub backend: SupportedBackend,
}

#[derive(Deserialize, Debug, PartialEq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CollectorConfig {
	pub name: String,
	pub config: HashMap<String, String>,
	pub backend: SupportedSources,
}

#[derive(Deserialize, Debug, PartialEq, Default, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SemanticPipelineRequest {
	pub name: String,
	pub import: String,
	pub attr: String,
	pub code: Option<String>,
	pub workflow_config: WorkflowConfig,
	pub config: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SemanticPipelineResponse {
	pub pipeline_id: String,
}
