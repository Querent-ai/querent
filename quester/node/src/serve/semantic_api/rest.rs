use std::{collections::HashMap, convert::Infallible, sync::Arc};

use actors::{AskError, MessageBus, Observe};
use common::{IndexingStatistics, SemanticPipelineRequest, SemanticPipelineResponse};
use querent::{
	create_querent_synapose_workflow, ObservePipeline, PipelineErrors, PipelineSettings,
	SemanticService, SemanticServiceCounters, SpawnPipeline,
};
use querent_synapse::callbacks::EventType;
use warp::{reject::Rejection, Filter};

use crate::{extract_format_from_qs, make_json_api_response, serve::require};

#[derive(utoipa::OpenApi)]
#[openapi(paths(semantic_endpoint, describe_pipeline, start_pipeline))]
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

#[utoipa::path(
    post,
    tag = "Semantic Service",
    path = "/semantics",
    request_body = SemanticPipelineRequest,
    responses(
        // We return `VersionedIndexMetadata` as it's the serialized model view.
        (status = 200, description = "Successfully started semantic pipeline.", body = SemanticPipelineResponse)
    ),
)]

async fn start_pipeline(
	request: SemanticPipelineRequest,
	semantic_service_mailbox: MessageBus<SemanticService>,
	event_storages: HashMap<EventType, Arc<dyn storage::Storage>>,
	index_storages: Vec<Arc<dyn storage::Storage>>,
) -> Result<SemanticPipelineResponse, PipelineErrors> {
	let new_uuid = uuid::Uuid::new_v4();
	let qflow: querent_synapse::querent::Workflow =
		create_querent_synapose_workflow(new_uuid.to_string(), &request).await?;
	let pipeline_settings = PipelineSettings {
		qflow_id: new_uuid.to_string(),
		qflow,
		event_storages,
		index_storages,
		semantic_service_bus: semantic_service_mailbox.clone(),
	};

	let pipeline_rest = semantic_service_mailbox
		.ask(SpawnPipeline { settings: pipeline_settings, pipeline_id: new_uuid.to_string() })
		.await;
	let pipeline_id = pipeline_rest.unwrap_or(Ok(new_uuid.to_string()));
	Ok(SemanticPipelineResponse { pipeline_id: pipeline_id.unwrap() })
}

pub fn start_pipeline_post_handler(
	semantic_service_bus: Option<MessageBus<SemanticService>>,
	event_storages: HashMap<EventType, Arc<dyn storage::Storage>>,
	index_storages: Vec<Arc<dyn storage::Storage>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics")
		.and(warp::body::json())
		.and(warp::post())
		.and(require(semantic_service_bus))
		.and(require(Some(event_storages)))
		.and(require(Some(index_storages)))
		.then(start_pipeline)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}
