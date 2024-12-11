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

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use proto::SpanContextInterceptor;
use std::{fmt, net::SocketAddr, sync::Arc};
use tonic::{codegen::InterceptedService, transport::Channel};
use tower::timeout::Timeout;

use crate::insight_api::insights_service::InsightService;

#[derive(Clone)]
enum InsightServiceClientImpl {
	Rest(Arc<dyn InsightService>),
	Grpc(
		proto::insights::insight_service_client::InsightServiceClient<
			InterceptedService<Timeout<Channel>, SpanContextInterceptor>,
		>,
	),
}

#[derive(Clone)]
pub struct InsightServiceClient {
	client_impl: InsightServiceClientImpl,
	grpc_addr: SocketAddr,
}

impl fmt::Debug for InsightServiceClient {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		match &self.client_impl {
			InsightServiceClientImpl::Rest(_service) => {
				write!(formatter, "Rest({:?})", self.grpc_addr)
			},
			InsightServiceClientImpl::Grpc(_grpc_client) => {
				write!(formatter, "Grpc({:?})", self.grpc_addr)
			},
		}
	}
}

impl InsightServiceClient {
	/// Create a discovery service client instance given a gRPC client and gRPC address.
	pub fn from_grpc_client(
		client: proto::insights::insight_service_client::InsightServiceClient<
			InterceptedService<Timeout<Channel>, SpanContextInterceptor>,
		>,
		grpc_addr: SocketAddr,
	) -> Self {
		InsightServiceClient { client_impl: InsightServiceClientImpl::Grpc(client), grpc_addr }
	}

	/// Create a search service client instance given a search service and gRPC address.
	pub fn from_service(service: Arc<dyn InsightService>, grpc_addr: SocketAddr) -> Self {
		InsightServiceClient { client_impl: InsightServiceClientImpl::Rest(service), grpc_addr }
	}

	/// Return the grpc_addr the underlying client connects to.
	pub fn grpc_addr(&self) -> SocketAddr {
		self.grpc_addr
	}

	/// Returns whether the underlying client is rest or remote.
	#[cfg(any(test, feature = "testsuite"))]
	pub fn is_local(&self) -> bool {
		matches!(self.client_impl, InsightServiceClientImpl::Rest(_))
	}
}
