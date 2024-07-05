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
