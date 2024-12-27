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

use std::sync::Arc;

use crate::discovery_api::discovery_service::{error::DiscoveryError, DiscoveryService};
use async_trait::async_trait;
use proto::{discovery::discovery_server as grpc, error::convert_to_grpc_result};
use tracing::instrument;

#[derive(Clone)]
pub struct DiscoveryAdapter(Arc<dyn DiscoveryService>);

impl From<Arc<dyn DiscoveryService>> for DiscoveryAdapter {
	fn from(service: Arc<dyn DiscoveryService>) -> Self {
		DiscoveryAdapter(service)
	}
}

#[async_trait]
impl grpc::Discovery for DiscoveryAdapter {
	#[instrument(skip(self, request))]
	async fn start_discovery_session(
		&self,
		request: tonic::Request<proto::discovery::DiscoverySessionRequest>,
	) -> Result<tonic::Response<proto::discovery::DiscoverySessionResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::DiscoverySessionResponse, DiscoveryError> =
			self.0.start_discovery_session(req).await;
		convert_to_grpc_result(res)
	}

	#[instrument(skip(self, request))]
	async fn discover_insights(
		&self,
		request: tonic::Request<proto::discovery::DiscoveryRequest>,
	) -> Result<tonic::Response<proto::discovery::DiscoveryResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::DiscoveryResponse, DiscoveryError> =
			self.0.discover_insights(req).await;
		convert_to_grpc_result(res)
	}

	#[instrument(skip(self, request))]
	async fn stop_discovery_session(
		&self,
		request: tonic::Request<proto::discovery::StopDiscoverySessionRequest>,
	) -> Result<tonic::Response<proto::discovery::StopDiscoverySessionResponse>, tonic::Status> {
		let req = request.into_inner();
		let res: Result<proto::StopDiscoverySessionResponse, DiscoveryError> =
			self.0.stop_discovery_session(req).await;
		convert_to_grpc_result(res)
	}

	#[instrument(skip(self, empty))]
	async fn list_discovery_sessions(
		&self,
		empty: tonic::Request<proto::discovery::Empty>,
	) -> Result<tonic::Response<proto::discovery::DiscoverySessionRequestInfoList>, tonic::Status>
	{
		let _req = empty.into_inner();
		let res: Result<proto::DiscoverySessionRequestInfoList, DiscoveryError> =
			self.0.get_discovery_session_list().await;
		convert_to_grpc_result(res)
	}
}
