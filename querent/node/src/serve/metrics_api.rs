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

#[derive(utoipa::OpenApi)]
#[openapi(paths(metrics_handler))]
/// Endpoints which are weirdly tied to another crate with no
/// other bits of information attached.
///
/// If a crate plans to encompass different schemas, handlers, etc...
/// Then it should have it's own specific API group.
pub struct MetricsApi;

#[utoipa::path(
    get,
    tag = "Get Metrics",
    path = "/",
    responses(
        (status = 200, description = "Successfully fetched metrics.", body = String),
    ),
)]
/// Get Node Metrics
///
/// These are in the form of prometheus metrics.
pub fn metrics_handler() -> impl warp::Reply {
	common::metrics::metrics_text_payload()
}
