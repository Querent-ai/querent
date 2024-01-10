use std::convert::Infallible;

use actors::{AskError, MessageBus, Observe};
use common::IndexingStatistics;
use querent::{ObservePipeline, PipelineErrors, SemanticService, SemanticServiceCounters};
use warp::{reject::Rejection, Filter};

use crate::{extract_format_from_qs, make_json_api_response, serve::require};

#[derive(utoipa::OpenApi)]
#[openapi(paths(semantic_endpoint, describe_pipeline))]
pub struct SemanticApi;

#[utoipa::path(
    get,
    tag = "Semantic Service",
    path = "/semantics",
    responses(
        (status = 200, description = "Successfully observed semantic pipelines.", body = SemanticServiceCounters)
    ),
)]
/// Observe Service Pipeline
async fn semantic_endpoint(
	semantic_service_mailbox: MessageBus<SemanticService>,
) -> Result<SemanticServiceCounters, AskError<Infallible>> {
	let counters = semantic_service_mailbox.ask(Observe).await?;
	semantic_service_mailbox.ask(Observe).await?;
	Ok(counters)
}

#[utoipa::path(
	get,
	tag = "Semantic Service",
	path = "/semantics/{pipeline_id}",
	responses(
		(status = 200, description = "Successfully observed semantic pipelines.", body = IndexingStatistics)
	),
	params(
		("pipeline_id" = String, Path, description = "The pipeline id running semantic loop to describe.")
	)
)]

/// Observes semantic pipelines.
async fn describe_pipeline(
	pipeline_id: String,
	semantic_service_mailbox: MessageBus<SemanticService>,
) -> Result<IndexingStatistics, PipelineErrors> {
	let counters = semantic_service_mailbox.ask(ObservePipeline { pipeline_id }).await;
	counters.unwrap()
}

pub fn pipelines_get_all_handler(
	semantic_service_bus: Option<MessageBus<SemanticService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics")
		.and(warp::get())
		.and(require(semantic_service_bus))
		.then(semantic_endpoint)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}
fn pipelines_get_filter() -> impl Filter<Extract = (), Error = Rejection> + Clone {
	warp::path!("semantics").and(warp::get())
}

pub fn pipelines_get_handler(
	semantic_service_bus: Option<MessageBus<SemanticService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	pipelines_get_filter()
		.and(require(semantic_service_bus))
		.then(semantic_endpoint)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}

pub fn observe_pipeline_get_handler(
	semantic_service_bus: Option<MessageBus<SemanticService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics" / String / "describe")
		.and(warp::get())
		.and(require(semantic_service_bus))
		.then(describe_pipeline)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}

// #[utoipa::path(
//     post,
//     tag = "Semantic Service",
//     path = "/semantics",
//     request_body = SemanticPipelineRequest,
//     responses(
//         // We return `VersionedIndexMetadata` as it's the serialized model view.
//         (status = 200, description = "Successfully started semantic pipeline.", body = SemanticPipelineResponse)
//     ),
// )]
// Starts a new semantic pipeline
// async fn start_pipeline(
// 	semantic_service_mailbox: MessageBus<SemanticService>,
// 	request: SemanticPipelineRequest,
// ) -> Result<String, PipelineErrors> {
// 	let new_uuid = uuid::Uuid::new_v4();
// 	let pipeline_settings = PipelineSettings { qflow_id: new_uuid.to_string(), ..request.workflow.config };
// 	let pipeline_rest = semantic_service_mailbox
// 		.ask(SpawnPipeline { settings: pipeline_settings, pipeline_id: new_uuid.to_string() })
// 		.await;
// 	pipeline_rest.unwrap()
// }
