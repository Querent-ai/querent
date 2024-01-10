use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Eq, PartialEq, Default, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowConfig {
	pub version: String,
	pub collectors: Vec<CollectorConfig>,
	pub engines: Vec<EngineConfig>,
}

#[derive(Deserialize, Debug, Eq, PartialEq, utoipa::ToSchema)]
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

#[derive(Deserialize, Debug, Eq, PartialEq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub enum SupportedBackend {
	#[serde(rename = "llama2_v1")]
	KnowledgeGraphLlama2V1,
	#[serde(rename = "openai")]
	KnowledgeGraphOpenAI,
}

#[derive(Deserialize, Debug, Eq, PartialEq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EngineConfig {
	pub config: HashMap<String, String>,
	pub backend: SupportedBackend,
}

#[derive(Deserialize, Debug, Eq, PartialEq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CollectorConfig {
	pub name: String,
	pub config: HashMap<String, String>,
	pub backend: SupportedSources,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Default, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SemanticPipelineRequest {
	pub name: String,
	pub import: String,
	pub attr: String,
	pub code: Option<String>,
	pub config: WorkflowConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SemanticPipelineResponse {
	pub pipeline_id: String,
}
