// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use actors::{AskError, MessageBus, Observe};
use common::{get_querent_data_path, EventType};
use engines::agn::AttentionTensorsEngine;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use futures_util::StreamExt;
use llms::{
	transformers::{
		bert::{BertLLM, EmbedderOptions},
		roberta::roberta::RobertaLLM,
	},
	LLM,
};
use proto::{
	config::StorageConfigs,
	semantics::{
		AzureCollectorConfig, Backend, CollectorConfig, CollectorConfigResponse,
		DeleteCollectorRequest, DeleteCollectorResponse, DropBoxCollectorConfig,
		EmailCollectorConfig, EmptyGetPipelinesMetadata, FileCollectorConfig, FixedEntities,
		GcsCollectorConfig, GithubCollectorConfig, GoogleDriveCollectorConfig, IndexingStatistics,
		JiraCollectorConfig, ListCollectorConfig, ListCollectorRequest, Neo4jConfig,
		NewsCollectorConfig, NotionConfig, OneDriveConfig, OsduServiceConfig, PipelineMetadata,
		PipelineRequestInfo, PipelineRequestInfoList, PipelinesMetadata, PostgresConfig,
		RecordKind, S3CollectorConfig, SampleEntities, SemanticPipelineRequest,
		SemanticPipelineResponse, SendIngestedTokens, SlackCollectorConfig, StorageConfig,
		StorageType,
	},
};
use serde_json::from_str;

use proto::semantics::IngestedTokens;
use rian_core::{
	create_dynamic_sources, ObservePipeline, PipelineErrors, PipelineSettings, RestartPipeline,
	SemanticService, SemanticServiceCounters, ShutdownPipeline, SpawnPipeline,
};
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use tracing::{error, warn};
use warp::{filters::ws::WebSocket, reject::Rejection, Filter};

use crate::{extract_format_from_qs, make_json_api_response, serve::require, Model};

#[derive(utoipa::OpenApi)]
#[openapi(
	paths(
		semantic_endpoint,
		describe_pipeline,
		start_pipeline,
		get_pipelines_metadata,
		stop_pipeline,
		ingest_tokens,
		restart_pipeline,
		set_collectors,
		delete_collectors,
		list_collectors,
		get_pipelines_history,
	),
	components(schemas(
		SemanticPipelineRequest,
		Backend,
		SemanticPipelineResponse,
		SemanticServiceCounters,
		IndexingStatistics,
		CollectorConfig,
		IngestedTokens,
		PipelinesMetadata,
		PipelineMetadata,
		GcsCollectorConfig,
		S3CollectorConfig,
		JiraCollectorConfig,
		GoogleDriveCollectorConfig,
		DropBoxCollectorConfig,
		GithubCollectorConfig,
		NewsCollectorConfig,
		FileCollectorConfig,
		OneDriveConfig,
		StorageConfigs,
		StorageConfig,
		StorageType,
		PostgresConfig,
		Neo4jConfig,
		FixedEntities,
		SampleEntities,
		AzureCollectorConfig,
		EmailCollectorConfig,
		SlackCollectorConfig,
		CollectorConfigResponse,
		DeleteCollectorRequest,
		DeleteCollectorResponse,
		ListCollectorRequest,
		ListCollectorConfig,
		PipelineRequestInfoList,
		PipelineRequestInfo,
		NotionConfig,
		OsduServiceConfig,
		RecordKind,
		SalesForceConfig,
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
	path = "/semantics/list",
	responses(
		(status = 200, description = "Get pipelines metadata", body = PipelineRequestInfoList)
	),
)]

/// Get pipelines metadata
pub async fn get_pipelines_history(
	metadata_store: Arc<dyn storage::MetaStorage>,
) -> Result<proto::semantics::PipelineRequestInfoList, PipelineErrors> {
	let pipelines = metadata_store
		.get_all_pipelines()
		.await
		.map_err(|e| PipelineErrors::UnknownError(e.to_string()))?;
	let mut requests = Vec::new();
	for (pipeline_id, pipeline) in pipelines {
		let pipeline_metadata = proto::semantics::PipelineRequestInfo {
			pipeline_id: pipeline_id.clone(),
			request: Some(pipeline.clone()),
		};
		requests.push(pipeline_metadata);
	}
	Ok(proto::semantics::PipelineRequestInfoList { requests })
}

pub fn get_pipelines_history_handler(
	metadata_store: Arc<dyn storage::MetaStorage>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics" / "list")
		.and(warp::get())
		.and(require(Some(metadata_store)))
		.then(get_pipelines_history)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
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
pub async fn describe_pipeline(
	pipeline_id: String,
	semantic_service_mailbox: MessageBus<SemanticService>,
) -> Result<IndexingStatistics, PipelineErrors> {
	let counters: Result<Result<IndexingStatistics, PipelineErrors>, AskError<Infallible>> =
		semantic_service_mailbox.ask(ObservePipeline { pipeline_id }).await;
	match counters {
		Ok(counters) => counters,
		Err(_e) => {
			println!("Error in describe pipeline: {:?}", _e);
			return Ok(IndexingStatistics::default());
		},
	}
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
		.boxed()
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
		.boxed()
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
		.boxed()
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

/// Start semantic pipeline by providing a `SemanticPipelineRequest`.
pub async fn start_pipeline(
	request: SemanticPipelineRequest,
	semantic_service_mailbox: MessageBus<SemanticService>,
	event_storages: HashMap<EventType, Vec<Arc<dyn storage::Storage>>>,
	index_storages: Vec<Arc<dyn storage::Storage>>,
	secret_store: Arc<dyn storage::SecretStorage>,
	metadata_store: Arc<dyn storage::MetaStorage>,
) -> Result<SemanticPipelineResponse, PipelineErrors> {
	let new_uuid = uuid::Uuid::new_v4().to_string().replace("-", "");
	// Extract entities from request.fixed_entities or use a default
	let entities = match &request.fixed_entities {
		Some(fixed_entities) => fixed_entities.entities.clone(),
		_ => Vec::new(),
	};

	// Extract entities from request.fixed_entities or use a default
	let sample_entities = match &request.sample_entities {
		Some(sample_entities) => sample_entities.entities.clone(),
		_ => Vec::new(),
	};

	// Extract and handle the model parameter
	let (model_string, model_type) = match request.model.and_then(Model::from_i32) {
		Some(Model::English) =>
			("Davlan/xlm-roberta-base-wikiann-ner".to_string(), "Roberta".to_string()),
		Some(Model::Geology) => ("botryan96/GeoBERT".to_string(), "Bert".to_string()),
		_ => ("Davlan/xlm-roberta-base-wikiann-ner".to_string(), "Roberta".to_string()), // Default to option 1
	};

	// Initialize NER model only if fixed_entities is not defined or empty
	let ner_llm: Option<Arc<dyn LLM>> = if entities.is_empty() {
		let ner_options = EmbedderOptions {
			model: model_string.to_string(),
			local_dir: None,
			revision: None,
			distribution: None,
		};
		if model_type == "Roberta".to_string() {
			Some(Arc::new(
				RobertaLLM::new(ner_options)
					.map_err(|e| PipelineErrors::UnknownError(e.to_string()))?,
			) as Arc<dyn LLM>)
		} else if model_type == "Bert".to_string() {
			Some(Arc::new(
				BertLLM::new(ner_options)
					.map_err(|e| PipelineErrors::UnknownError(e.to_string()))?,
			) as Arc<dyn LLM>)
		} else {
			None
		}
	} else {
		None // Some(Arc::new(DummyLLM) as Arc<dyn LLM>)
	};

	let mut collectors_configs = Vec::new();
	for collector_id in request.clone().collectors {
		let config_value = secret_store.get_secret(&collector_id).await.map_err(|e| {
			PipelineErrors::InvalidParams(anyhow::anyhow!("Failed to create sources: {:?}", e))
		})?;
		if let Some(value) = config_value {
			let collector_config_value: CollectorConfig =
				serde_json::from_str(&value).map_err(|e| {
					PipelineErrors::InvalidParams(anyhow::anyhow!(
						"Failed to create sources: {:?}",
						e
					))
				})?;
			collectors_configs.push(collector_config_value);
		}
	}
	let mut _license_key = None;
	#[cfg(feature = "license-check")]
	{
		_license_key = secret_store.get_rian_api_key().await.map_err(|e| {
			PipelineErrors::InvalidParams(anyhow::anyhow!("Failed to create sources: {:?}", e))
		})?;
	};
	let data_sources = create_dynamic_sources(_license_key, collectors_configs).await?;

	let options = EmbedderOptions {
		model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
		local_dir: None,
		revision: None,
		distribution: None,
	};
	let embedder =
		Arc::new(BertLLM::new(options).map_err(|e| PipelineErrors::UnknownError(e.to_string()))?);

	let model_details: InitOptions = InitOptions::new(EmbeddingModel::AllMiniLML6V2)
		.with_cache_dir(get_querent_data_path())
		.with_show_download_progress(true);
	let embedding_model = TextEmbedding::try_new(model_details)
		.map_err(|e| PipelineErrors::UnknownError(e.to_string()))?;

	let engine = Arc::new(AttentionTensorsEngine::new(
		embedder,
		entities,
		sample_entities,
		Some(embedding_model),
		ner_llm,
	));

	let pipeline_settings =
		PipelineSettings { engine, event_storages, index_storages, secret_store, data_sources };

	let pipeline_rest = semantic_service_mailbox
		.ask(SpawnPipeline { settings: pipeline_settings, pipeline_id: new_uuid.clone() })
		.await;
	let pipeline_id = pipeline_rest.unwrap_or(Ok("".to_string()));
	if pipeline_id.is_err() {
		return Err(PipelineErrors::UnknownError(pipeline_id.unwrap_err().to_string()).into());
	}
	let id: String = pipeline_id.unwrap_or_default();
	let result_pipe_obs = describe_pipeline(id.clone(), semantic_service_mailbox.clone()).await;
	if result_pipe_obs.is_err() {
		return Err(PipelineErrors::InvalidParams(anyhow::anyhow!(
			"Failed to describe pipeline: {:?}",
			result_pipe_obs.unwrap_err()
		)));
	}
	metadata_store
		.set_pipeline(&id.clone(), request)
		.await
		.map_err(|e| PipelineErrors::UnknownError(e.to_string()))?;
	//call secret store and get the credentials
	Ok(SemanticPipelineResponse { pipeline_id: id })
}

pub fn start_pipeline_post_handler(
	semantic_service_bus: Option<MessageBus<SemanticService>>,
	event_storages: HashMap<EventType, Vec<Arc<dyn storage::Storage>>>,
	index_storages: Vec<Arc<dyn storage::Storage>>,
	secret_store: Arc<dyn storage::SecretStorage>,
	metadata_store: Arc<dyn storage::MetaStorage>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics")
		.and(warp::body::json())
		.and(warp::post())
		.and(require(semantic_service_bus))
		.and(require(Some(event_storages)))
		.and(require(Some(index_storages)))
		.and(require(Some(secret_store)))
		.and(require(Some(metadata_store)))
		.then(start_pipeline)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
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
pub async fn get_pipelines_metadata(
	semantic_service_mailbox: MessageBus<SemanticService>,
) -> Result<PipelinesMetadata, AskError<Infallible>> {
	let pipelines = semantic_service_mailbox.ask(EmptyGetPipelinesMetadata {}).await?;
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
		.boxed()
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

/// Stop semantic pipeline by providing a pipeline id.
pub async fn stop_pipeline(
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
	request_body = Vec<IngestedTokens>,
	responses(
		(status = 200, description = "Successfully ingested tokens.", body = bool)
	),
	params(
		("pipeline_id" = String, Path, description = "The pipeline id running semantic loop to ingest tokens.")
	)
)]

/// Send tokens to semantic pipeline by providing `IngestedTokens`.
pub async fn ingest_tokens(
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
		.boxed()
}

/// restart semantic pipeline by providing a pipeline id.
#[utoipa::path(
	post,
	tag = "Semantic Service",
	path = "/semantics/{pipeline_id}/restart",
	responses(
		(status = 200, description = "Successfully restarted semantic pipeline.", body = SemanticPipelineResponse)
	),
	params(
		("pipeline_id" = String, Path, description = "The pipeline id running semantic loop to restart.")
	)
)]
/// Restart semantic pipeline by providing a pipeline id.
pub async fn restart_pipeline(
	pipeline_id: String,
	semantic_service_mailbox: MessageBus<SemanticService>,
) -> Result<bool, PipelineErrors> {
	let pipeline_rest = semantic_service_mailbox.ask(RestartPipeline { pipeline_id }).await;
	match pipeline_rest {
		Ok(_) => Ok(true),
		Err(e) => Err(PipelineErrors::UnknownError(e.to_string()).into()),
	}
}

pub fn restart_pipeline_post_handler(
	semantic_service_bus: Option<MessageBus<SemanticService>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics" / String / "restart")
		.and(warp::post())
		.and(require(semantic_service_bus))
		.then(restart_pipeline)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}

pub fn set_collectors_post_handler(
	secret_store: Arc<dyn storage::SecretStorage>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics" / "collectors")
		.and(warp::body::json())
		.and(warp::post())
		.and(require(Some(secret_store)))
		.then(set_collectors)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}

#[utoipa::path(
    post,
    tag = "Semantic Service",
    path = "/semantics/collectors",
    request_body = CollectorConfig,
    responses(
        (status = 200, description = "Set the collectors successfully", body = CollectorConfigResponse)
    ),
)]
pub async fn set_collectors(
	collector: CollectorConfig,
	secret_store: Arc<dyn storage::SecretStorage>,
) -> Result<CollectorConfigResponse, PipelineErrors> {
	let collector_string = serde_json::to_string(&collector).map_err(|e| {
		PipelineErrors::InvalidParams(anyhow::anyhow!("Unable to convert to json string: {:?}", e))
	})?;
	let collector_json: &serde_json::Value =
		&serde_json::from_str(&collector_string).map_err(|e| {
			PipelineErrors::InvalidParams(anyhow::anyhow!("Unable to convert to json {:?}", e))
		})?;
	let id = collector_json["backend"]
		.as_object()
		.and_then(|backend| {
			backend.values().find_map(|backend_value| {
				backend_value.as_object()?.get("id")?.as_str().map(|s| s.to_string())
			})
		})
		.ok_or_else(|| anyhow::anyhow!("Failed to extract id from CollectorConfig"))
		.map_err(|e| {
			PipelineErrors::InvalidParams(anyhow::anyhow!(
				"Failed to extract id from CollectorConfig {:?}",
				e
			))
		})?;

	secret_store.store_secret(&id, &collector_string).await.map_err(|e| {
		PipelineErrors::InvalidParams(anyhow::anyhow!("Failed to store key: {:?}", e))
	})?;

	Ok(CollectorConfigResponse { id })
}

pub async fn set_collectors_all(
	collectors: Vec<CollectorConfig>,
	secret_store: Arc<dyn storage::SecretStorage>,
) -> Result<CollectorConfigResponse, PipelineErrors> {
	for collector in collectors {
		let collector_string = serde_json::to_string(&collector).map_err(|e| {
			PipelineErrors::InvalidParams(anyhow::anyhow!(
				"Unable to convert to json string: {:?}",
				e
			))
		})?;
		let collector_json: &serde_json::Value =
			&serde_json::from_str(&collector_string).map_err(|e| {
				PipelineErrors::InvalidParams(anyhow::anyhow!("Unable to convert to json {:?}", e))
			})?;
		let id = collector_json["backend"]
			.as_object()
			.and_then(|backend| {
				backend.values().find_map(|backend_value| {
					backend_value.as_object()?.get("id")?.as_str().map(|s| s.to_string())
				})
			})
			.ok_or_else(|| anyhow::anyhow!("Failed to extract id from CollectorConfig"))
			.map_err(|e| {
				PipelineErrors::InvalidParams(anyhow::anyhow!(
					"Failed to extract id from CollectorConfig {:?}",
					e
				))
			})?;

		secret_store.store_secret(&id, &collector_string).await.map_err(|e| {
			PipelineErrors::InvalidParams(anyhow::anyhow!("Failed to store key: {:?}", e))
		})?;
	}
	Ok(CollectorConfigResponse { id: "".to_string() })
}

pub fn delete_collectors_delete_handler(
	secret_store: Arc<dyn storage::SecretStorage>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics" / "collectors" / "purge")
		.and(warp::body::json())
		.and(warp::delete())
		.and(require(Some(secret_store)))
		.then(delete_collectors)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}

#[utoipa::path(
    delete,
    tag = "Semantic Service",
    path = "/semantics/collectors/purge",
    request_body = DeleteCollectorRequest,
    responses(
        (status = 200, description = "Deleted the collectors successfully", body = DeleteCollectorResponse)
    ),
)]

pub async fn delete_collectors(
	collector: DeleteCollectorRequest,
	secret_store: Arc<dyn storage::SecretStorage>,
) -> Result<DeleteCollectorResponse, PipelineErrors> {
	for key in collector.id.clone() {
		secret_store.delete_secret(&key).await.map_err(|e| {
			PipelineErrors::InvalidParams(anyhow::anyhow!(
				"Failed to delete key {:?}: {:?}",
				key,
				e
			))
		})?;
	}
	Ok(DeleteCollectorResponse { id: collector.id })
}

pub fn list_collectors_list_handler(
	secret_store: Arc<dyn storage::SecretStorage>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("semantics" / "collectors" / "list")
		.and(warp::get())
		.and(require(Some(secret_store)))
		.then(list_collectors)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
}

#[utoipa::path(
    get,
    tag = "Semantic Service",
    path = "/semantics/collectors/list",
    responses(
        (status = 200, description = "Listed the collectors successfully", body = ListCollectorConfig)
    ),
)]

pub async fn list_collectors(
	secret_store: Arc<dyn storage::SecretStorage>,
) -> Result<ListCollectorConfig, PipelineErrors> {
	let collectors = secret_store.get_all_secrets().await.map_err(|e| {
		PipelineErrors::InvalidParams(anyhow::anyhow!("Failed to list sources: {:?}", e))
	})?;

	let mut config_list = Vec::new();

	for (key, collector) in collectors {
		if key == "RIAN_API_KEY" {
			continue;
		}
		let config: CollectorConfig = from_str(&collector).map_err(|e| {
			PipelineErrors::InvalidParams(anyhow::anyhow!("Failed to create sources: {:?}", e))
		})?;
		config_list.push(config);
	}
	Ok(ListCollectorConfig { config: config_list })
}
