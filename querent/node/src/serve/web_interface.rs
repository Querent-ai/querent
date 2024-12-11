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

use hyper::header::HeaderValue;
use once_cell::sync::Lazy;
use regex::Regex;
use rust_embed::RustEmbed;
use warp::{path::Tail, reply::Response, Filter, Rejection};

/// Regular expression to identify which path should serve an asset file.
/// If not matched, the server serves the `index.html` file.
const PATH_PATTERN: &str = r"(^static|\.(png|json|txt|ico|js|map)$)";

const UI_INDEX_FILE_NAME: &str = "index.html";

#[derive(RustEmbed)]
#[folder = "../web/build/"]
struct Asset;

pub fn ui_handler() -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
	warp::path("ui").and(warp::path::tail()).and_then(serve_file)
}

async fn serve_file(path: Tail) -> Result<impl warp::Reply, Rejection> {
	serve_impl(path.as_str()).await
}

async fn serve_impl(path: &str) -> Result<impl warp::Reply, Rejection> {
	static PATH_PTN: Lazy<Regex> = Lazy::new(|| Regex::new(PATH_PATTERN).unwrap());
	let path_to_file = if PATH_PTN.is_match(path) { path } else { UI_INDEX_FILE_NAME };
	let asset = Asset::get(path_to_file).ok_or_else(warp::reject::not_found)?;
	let mime = mime_guess::from_path(path_to_file).first_or_octet_stream();

	let mut res = Response::new(asset.data.into());
	res.headers_mut()
		.insert("content-type", HeaderValue::from_str(mime.as_ref()).unwrap());
	Ok(res)
}
