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

use std::convert::Infallible;

use cluster::{Cluster, ClusterSnapshot, NodeIdSchema};
use warp::{Filter, Rejection};

use crate::{format::extract_format_from_qs, json_api_response::make_json_api_response};

#[derive(utoipa::OpenApi)]
#[openapi(paths(get_cluster), components(schemas(ClusterSnapshot, NodeIdSchema,)))]
pub struct ClusterApi;

/// Cluster handler.
pub fn cluster_handler(
	cluster: Cluster,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path!("cluster")
		.and(warp::path::end())
		.and(warp::get())
		.and(warp::path::end().map(move || cluster.clone()))
		.then(get_cluster)
		.and(extract_format_from_qs())
		.map(make_json_api_response)
		.boxed()
}

#[utoipa::path(
    get,
    tag = "Cluster Info",
    path = "/cluster",
    responses(
        (status = 200, description = "Successfully fetched cluster information.", body = ClusterSnapshot)
    )
)]

/// Get cluster information.
async fn get_cluster(cluster: Cluster) -> Result<ClusterSnapshot, Infallible> {
	let snapshot = cluster.snapshot().await;
	Ok(snapshot)
}
