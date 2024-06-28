use actors::{MessageBus, Observe};
use async_trait::async_trait;
use common::EventType;
use proto::semantics::{semantics_service_grpc_server as grpc, BooleanResponse, IngestedTokens};
use querent::{PipelineErrors, SemanticService};
use std::{collections::HashMap, sync::Arc};
use storage::Storage;
use tracing::instrument;

use crate::{
	delete_collectors, describe_pipeline, get_pipelines_metadata, ingest_tokens, restart_pipeline,
	set_collectors, start_pipeline, stop_pipeline,
};

#[derive(Debug, Clone)]
pub struct SemanticsGrpcAdapter {
	semantic_service_mailbox: MessageBus<SemanticService>,
	event_storages: HashMap<EventType, Vec<Arc<dyn Storage>>>,
	index_storages: Vec<Arc<dyn Storage>>,
	secret_store: Arc<dyn Storage>,
}

impl SemanticsGrpcAdapter {
	pub fn new(
		semantic_service_mailbox: MessageBus<SemanticService>,
		event_storages: HashMap<EventType, Vec<Arc<dyn storage::Storage>>>,
		index_storages: Vec<Arc<dyn storage::Storage>>,
		secret_store: Arc<dyn storage::Storage>,
	) -> Self {
		Self { semantic_service_mailbox, event_storages, index_storages, secret_store }
	}
}

pub type GrpcResult<T, E> = std::result::Result<T, E>;

#[async_trait]
impl grpc::SemanticsServiceGrpc for SemanticsGrpcAdapter {
	#[instrument(skip(self, request))]
	async fn start_pipeline(
		&self,
		request: tonic::Request<proto::semantics::SemanticPipelineRequest>,
	) -> GrpcResult<tonic::Response<proto::semantics::SemanticPipelineResponse>, tonic::Status> {
		let req = request.into_inner();
		let response = start_pipeline(
			req,
			self.semantic_service_mailbox.clone(),
			self.event_storages.clone(),
			self.index_storages.clone(),
			self.secret_store.clone(),
		)
		.await;
		match response {
			Ok(response) => Ok(tonic::Response::new(response)),
			Err(err) => Err(tonic::Status::from(err)),
		}
	}

	#[instrument(skip(self, request))]

	async fn observe_pipeline(
		&self,
		request: tonic::Request<proto::semantics::EmptyObserve>,
	) -> GrpcResult<tonic::Response<proto::semantics::SemanticServiceCounters>, tonic::Status> {
		let _req = request.into_inner();
		let response = self.semantic_service_mailbox.ask(Observe).await;
		match response {
			Ok(response) => {
				let result = proto::semantics::SemanticServiceCounters {
					num_failed_pipelines: response.num_failed_pipelines as i32,
					num_running_pipelines: response.num_running_pipelines as i32,
					num_successful_pipelines: response.num_successful_pipelines as i32,
				};
				Ok(tonic::Response::new(result))
			},
			Err(err) => Err(tonic::Status::from(PipelineErrors::UnknownError(err.to_string()))),
		}
	}

	#[instrument(skip(self, request))]
	async fn get_pipelines_metadata(
		&self,
		request: tonic::Request<proto::semantics::EmptyGetPipelinesMetadata>,
	) -> GrpcResult<tonic::Response<proto::semantics::PipelinesMetadata>, tonic::Status> {
		let _req = request.into_inner();
		let response = get_pipelines_metadata(self.semantic_service_mailbox.clone()).await;
		match response {
			Ok(response) => Ok(tonic::Response::new(response)),
			Err(err) => Err(tonic::Status::from(PipelineErrors::UnknownError(err.to_string()))),
		}
	}

	#[instrument(skip(self, request))]
	async fn stop_pipeline(
		&self,
		request: tonic::Request<proto::semantics::StopPipelineRequest>,
	) -> GrpcResult<tonic::Response<proto::semantics::BooleanResponse>, tonic::Status> {
		let stop_request = request.into_inner();
		let response =
			stop_pipeline(stop_request.pipeline_id, self.semantic_service_mailbox.clone()).await;
		match response {
			Ok(response) => Ok(tonic::Response::new(BooleanResponse { response })),
			Err(err) => Err(tonic::Status::from(err)),
		}
	}

	#[instrument(skip(self, request))]
	async fn describe_pipeline(
		&self,
		request: tonic::Request<proto::semantics::DescribePipelineRequest>,
	) -> GrpcResult<tonic::Response<proto::semantics::IndexingStatistics>, tonic::Status> {
		let req = request.into_inner();
		let response =
			describe_pipeline(req.pipeline_id, self.semantic_service_mailbox.clone()).await;
		match response {
			Ok(response) => Ok(tonic::Response::new(response)),
			Err(err) => Err(tonic::Status::from(err)),
		}
	}

	#[instrument(skip(self, request))]
	async fn ingest_tokens(
		&self,
		request: tonic::Request<proto::semantics::SendIngestedTokens>,
	) -> GrpcResult<tonic::Response<proto::semantics::BooleanResponse>, tonic::Status> {
		let tokens_request = request.into_inner();
		let pipeline_id = tokens_request.pipeline_id;
		let tokens = tokens_request.tokens;
		let mut synapse_tokens = Vec::new();
		tokens.into_iter().for_each(|token| {
			let synapse_token = IngestedTokens {
				file: token.file,
				data: token.data.into(),
				is_token_stream: token.is_token_stream,
				doc_source: token.doc_source.into(),
			};
			synapse_tokens.push(synapse_token);
		});

		let response =
			ingest_tokens(pipeline_id, synapse_tokens, self.semantic_service_mailbox.clone()).await;
		match response {
			Ok(response) => Ok(tonic::Response::new(BooleanResponse { response })),
			Err(err) => Err(tonic::Status::from(err)),
		}
	}

	#[instrument(skip(self, request))]
	async fn restart_pipeline(
		&self,
		request: tonic::Request<proto::semantics::RestartPipelineRequest>,
	) -> GrpcResult<tonic::Response<proto::semantics::BooleanResponse>, tonic::Status> {
		let req = request.into_inner();
		let response =
			restart_pipeline(req.pipeline_id, self.semantic_service_mailbox.clone()).await;
		match response {
			Ok(response) => Ok(tonic::Response::new(BooleanResponse { response })),
			Err(err) => Err(tonic::Status::from(err)),
		}
	}

	#[instrument(skip(self, request))]
	async fn post_collectors(
		&self,
		request: tonic::Request<proto::semantics::CollectorConfig>,
	) -> GrpcResult<tonic::Response<proto::semantics::CollectorConfigResponse>, tonic::Status> {
		let req = request.into_inner();
		let response = set_collectors(req, self.secret_store.clone()).await;
		match response {
			Ok(response) => Ok(tonic::Response::new(response)),
			Err(err) => Err(tonic::Status::from(err)),
		}
	}

	#[instrument(skip(self, request))]
	async fn delete_collectors(
		&self,
		request: tonic::Request<proto::semantics::DeleteCollectorRequest>,
	) -> GrpcResult<tonic::Response<proto::semantics::DeleteCollectorResponse>, tonic::Status> {
		let req = request.into_inner();
		let response = delete_collectors(req, self.secret_store.clone()).await;
		match response {
			Ok(response) => Ok(tonic::Response::new(response)),
			Err(err) => Err(tonic::Status::from(err)),
		}
	}
}
