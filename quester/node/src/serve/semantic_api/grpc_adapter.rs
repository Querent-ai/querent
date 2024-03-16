use std::sync::Arc;
use proto::semantics::SemanticsService;
use async_trait::async_trait;
use querent::PipelineErrors;
use tracing::instrument;

#[derive(Clone)]
pub struct SemanticsGrpcAdapter(Arc<dyn SemanticsGrpcService>);

pub type GrpcResult<T> = std::result::Result<T, PipelineErrors>;

#[async_trait]
pub trait SemanticsGrpcService: 'static + Send + Sync {
    async fn start_pipeline(&self, request: proto::semantics::SemanticPipelineRequest) -> GrpcResult<proto::semantics::SemanticPipelineResponse>;
    async fn observe_pipeline(&self, request: proto::semantics::EmptyObserve) -> GrpcResult<proto::semantics::SemanticServiceCounters>;
    async fn stop_pipeline(&self, request: proto::semantics::EmptyStop) -> GrpcResult<proto::semantics::SemanticServiceCounters>;

}

impl From<Arc<dyn SemanticsService>> for SemanticsGrpcAdapter {
    fn from(semantics_service_arc: Arc<dyn SemanticsService>) -> Self {
        SemanticsGrpcAdapter(semantics_service_arc)
    }
}


#[async_trait]
impl SemanticsGrpcService for SemanticsGrpcAdapter {

    #[instrument(skip(self, request))]
    async fn start_pipeline(
        &self,
        request: tonic::Request<proto::semantics::SemanticPipelineRequest>,
    ) -> Result<tonic::Response<proto::semantics::SemanticPipelineResponse>, tonic::Status> {

    }

    #[instrument(skip(self, request))]
    async fn observe_pipeline(
         &self,
        request: tonic::Request<proto::semantics::EmptyObserve>,
    ) -> Result<tonic::Response<proto::semantics::SemanticServiceCounters>, tonic::Status> {

    }
}