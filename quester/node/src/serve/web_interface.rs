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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_path_regex() {
		let path_ptn = Regex::new(PATH_PATTERN).unwrap();

		assert!(path_ptn.is_match("manifest.json"));
		assert!(path_ptn.is_match("favicon.ico"));
		assert!(path_ptn.is_match("static/js/main.df380554.js.map"));
		assert!(path_ptn.is_match("android-chrome-192x192.png"));
		assert!(!path_ptn.is_match("search"));
		assert!(!path_ptn.is_match(""));
	}
}
