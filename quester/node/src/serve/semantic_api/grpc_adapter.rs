use actors::MessageBus;
use async_trait::async_trait;
use common::semantic_api::SemanticPipelineRequest;
use proto::{convert_to_grpc_result, semantics::semantics_service_grpc_server as grpc};
use querent::SemanticService;
use querent_synapse::callbacks::EventType;
use std::{collections::HashMap, sync::Arc};
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct SemanticsGrpcAdapter {
	semantic_service_mailbox: Arc<MessageBus<SemanticService>>,
	event_storages: HashMap<EventType, Arc<dyn storage::Storage>>,
	index_storages: Vec<Arc<dyn storage::Storage>>,
}

impl SemanticsGrpcAdapter {
	pub fn new(
		semantic_service_mailbox: Arc<MessageBus<SemanticService>>,
		event_storages: HashMap<EventType, Arc<dyn storage::Storage>>,
		index_storages: Vec<Arc<dyn storage::Storage>>,
	) -> Self {
		Self { semantic_service_mailbox, event_storages, index_storages }
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
		)
		.await;
		convert_to_grpc_result(response)
	}

	#[instrument(skip(self, request))]

	async fn observe_pipeline(
		&self,
		request: tonic::Request<proto::semantics::EmptyObserve>,
	) -> GrpcResult<tonic::Response<proto::semantics::SemanticServiceCounters>, tonic::Status> {
	}

	#[instrument(skip(self, request))]
	async fn get_pipelines_metadata(
		&self,
		request: tonic::Request<proto::semantics::EmptyGetPipelinesMetadata>,
	) -> GrpcResult<tonic::Response<proto::semantics::PipelinesMetadata>, tonic::Status> {
	}

	#[instrument(skip(self, request))]
	async fn stop_pipeline(
		&self,
		request: tonic::Request<proto::semantics::StopPipelineRequest>,
	) -> GrpcResult<tonic::Response<proto::semantics::BooleanResponse>, tonic::Status> {
	}

	#[instrument(skip(self, request))]
	async fn describe_pipeline(
		&self,
		request: tonic::Request<proto::semantics::DescribePipelineRequest>,
	) -> GrpcResult<tonic::Response<proto::semantics::IndexingStatistics>, tonic::Status> {
	}

	#[instrument(skip(self, request))]
	async fn ingest_tokens(
		&self,
		request: tonic::Request<proto::semantics::IngestedTokens>,
	) -> GrpcResult<tonic::Response<proto::semantics::BooleanResponse>, tonic::Status> {
	}

	#[instrument(skip(self, request))]
	async fn restart_pipeline(
		&self,
		request: tonic::Request<proto::semantics::RestartPipelineRequest>,
	) -> GrpcResult<tonic::Response<proto::semantics::BooleanResponse>, tonic::Status> {
	}
}
