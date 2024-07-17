use serde::{Deserialize, Serialize};

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
