use async_trait::async_trait;
use insights::InsightError;
use proto::{error::convert_to_grpc_result, insights::insight_service_server as grpc};
use std::sync::Arc;
use tracing::instrument;

use crate::insight_api::insights_service::InsightService;

#[derive(Clone)]
pub struct InsightAdapter(Arc<dyn InsightService>);

impl From<Arc<dyn InsightService>> for InsightAdapter {
	fn from(service: Arc<dyn InsightService>) -> Self {
		InsightAdapter(service)
	}
}

#[async_trait]
impl grpc::InsightService for InsightAdapter {
	#[instrument(skip(self, request))]
	async fn create_insight_session(
		&self,
		request: tonic::Request<proto::insights::InsightAnalystRequest>,
	) -> Result<tonic::Response<proto::insights::InsightAnalystResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::InsightAnalystResponse, InsightError> =
			self.0.start_insight_session(req).await;
		convert_to_grpc_result(res)
	}

	#[instrument(skip(self, request))]
	async fn provide_insight_input(
		&self,
		request: tonic::Request<proto::insights::InsightQuery>,
	) -> Result<tonic::Response<proto::insights::InsightQueryResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::InsightQueryResponse, InsightError> = self.0.send_input(req).await;
		convert_to_grpc_result(res)
	}

	#[instrument(skip(self, request))]
	async fn stop_insight_session(
		&self,
		request: tonic::Request<proto::insights::StopInsightSessionRequest>,
	) -> Result<tonic::Response<proto::insights::StopInsightSessionResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::StopInsightSessionResponse, InsightError> =
			self.0.stop_insight_session(req).await;
		convert_to_grpc_result(res)
	}
}
