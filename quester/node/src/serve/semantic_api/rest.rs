use actors::{AskError, MessageBus, Observe};
use common::{
	CollectorConfig, EngineConfig, GetAllPipelines, IndexingStatistics, PipelineMetadata,
	PipelinesMetadata, SemanticPipelineRequest, SemanticPipelineResponse, SendIngestedTokens,
	SupportedBackend, SupportedSources, WorkflowConfig,
};
use futures_util::StreamExt;
use querent::{
	create_querent_synapose_workflow, ObservePipeline, PipelineErrors, PipelineSettings,
	SemanticService, SemanticServiceCounters, ShutdownPipeline, SpawnPipeline,
};
use querent_synapse::{callbacks::EventType, comm::IngestedTokens};
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use tracing::{error, warn};
use warp::{filters::ws::WebSocket, reject::Rejection, Filter};

use crate::{extract_format_from_qs, make_json_api_response, serve::require};

#[derive(utoipa::OpenApi)]
#[openapi(
	paths(
		semantic_endpoint,
		describe_pipeline,
		start_pipeline,
		get_pipelines_metadata,
		stop_pipeline
	),
	components(schemas(
		SemanticPipelineRequest,
		SemanticPipelineResponse,
		SemanticServiceCounters,
		IndexingStatistics,
		WorkflowConfig,
		CollectorConfig,
		EngineConfig,
		SupportedBackend,
		SupportedSources,
		IngestedTokens,
		PipelinesMetadata,
		PipelineMetadata,
	))
)]
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
	path = "/semantics/{pipeline_id}/describe",
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

#[utoipa::path(
	get,
	tag = "Semantic Service",
	path = "/semantics/pipelines",
	responses(
		(status = 200, description = "Get pipelines metadata", body = PipelinesMetadata)
	),
)]

/// Get pipelines metadata
async fn get_pipelines_metadata(
	semantic_service_mailbox: MessageBus<SemanticService>,
) -> Result<PipelinesMetadata, AskError<Infallible>> {
	let pipelines = semantic_service_mailbox.ask(GetAllPipelines).await?;
	Ok(PipelinesMetadata { pipelines })
}

pub fn get_pipelines_metadata_handler(
	semantic_service_bus: Option<MessageBus<SemanticService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics" / "pipelines")
		.and(warp::get())
		.and(require(semantic_service_bus))
		.then(get_pipelines_metadata)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}

#[utoipa::path(
	delete,
	tag = "Semantic Service",
	path = "/semantics/{pipeline_id}",
	responses(
		(status = 200, description = "Successfully stopped semantic pipeline.", body = SemanticPipelineResponse)
	),
	params(
		("pipeline_id" = String, Path, description = "The pipeline id running semantic loop to stop.")
	)
)]

/// Stop semantic pipeline
async fn stop_pipeline(
	pipeline_id: String,
	semantic_service_mailbox: MessageBus<SemanticService>,
) -> Result<bool, PipelineErrors> {
	let pipeline_rest = semantic_service_mailbox.ask(ShutdownPipeline { pipeline_id }).await;
	match pipeline_rest {
		Ok(_) => Ok(true),
		Err(e) => Err(PipelineErrors::UnknownError(e.to_string()).into()),
	}
}

pub fn stop_pipeline_delete_handler(
	semantic_service_bus: Option<MessageBus<SemanticService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics" / String)
		.and(warp::delete())
		.and(require(semantic_service_bus))
		.then(stop_pipeline)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}

pub fn ingest_token_handler(
	semantic_service_bus: Option<MessageBus<SemanticService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics" / String / "ingest")
		.and(warp::ws())
		.and(require(semantic_service_bus))
		.and_then(|pipeline_id, ws, semantic_service_mailbox| {
			ingest_token(pipeline_id, ws, semantic_service_mailbox)
		})
}

async fn ingest_token(
	pipeline_id: String,
	ws: warp::ws::Ws,
	semantic_service_mailbox: MessageBus<SemanticService>,
) -> Result<impl warp::Reply, Infallible> {
	let ws = ws.on_upgrade(move |socket| {
		ingest_token_ws(socket, pipeline_id.clone(), semantic_service_mailbox.clone())
	});
	Ok(ws)
}

async fn ingest_token_ws(
	socket: WebSocket,
	pipeline_id: String,
	semantic_service_mailbox: MessageBus<SemanticService>,
) {
	let (_tx, mut rx) = socket.split();
	while let Some(result) = rx.next().await {
		match result {
			Ok(msg) =>
				if msg.is_text() {
					if let Ok(text) = msg.to_str() {
						if let Ok(tokens_vec) = serde_json::from_str::<Vec<IngestedTokens>>(text) {
							if let Err(e) = semantic_service_mailbox
								.ask(SendIngestedTokens {
									pipeline_id: pipeline_id.clone(),
									tokens: tokens_vec,
								})
								.await
							{
								error!("Error sending tokens to pipeline: {:?}", e);
							}
						} else {
							warn!("Failed to parse JSON: {:?}", text);
						}
					} else {
						warn!("Failed to convert message to string: {:?}", msg);
					}
				} else {
					warn!("Received non-text message: {:?}", msg);
				},
			Err(e) => {
				error!("Error receiving message: {:?}", e);
				break;
			},
		}
	}
}

#[utoipa::path(
	put,
	tag = "Semantic Service",
	path = "/semantics/{pipeline_id}/ingest",
	responses(
		(status = 200, description = "Successfully ingested tokens.", body = SemanticPipelineResponse)
	),
	params(
		("pipeline_id" = String, Path, description = "The pipeline id running semantic loop to ingest tokens.")
	)
)]
async fn ingest_tokens(
	pipeline_id: String,
	tokens: Vec<IngestedTokens>,
	semantic_service_mailbox: MessageBus<SemanticService>,
) -> Result<bool, PipelineErrors> {
	let pipeline_rest =
		semantic_service_mailbox.ask(SendIngestedTokens { pipeline_id, tokens }).await;
	match pipeline_rest {
		Ok(_) => Ok(true),
		Err(e) => Err(PipelineErrors::UnknownError(e.to_string()).into()),
	}
}

pub fn ingest_tokens_put_handler(
	semantic_service_bus: Option<MessageBus<SemanticService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics" / String / "ingest")
		.and(warp::put())
		.and(warp::body::json())
		.and(require(semantic_service_bus))
		.then(ingest_tokens)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}
