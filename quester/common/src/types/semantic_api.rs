use querent_synapse::comm::IngestedTokens;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, utoipa::ToSchema)]
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

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PipelinesMetadata {
	pub pipelines: Vec<PipelineMetadata>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PipelineMetadata {
	pub pipeline_id: String,
	pub name: String,
	pub import: String,
	pub attr: String,
}

#[derive(Debug, Clone)]
pub struct SendIngestedTokens {
	pub pipeline_id: String,
	pub tokens: Vec<IngestedTokens>,
}
