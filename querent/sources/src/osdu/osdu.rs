use std::{
	collections::HashMap,
	io::Cursor,
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
};

use async_stream::stream;
use async_trait::async_trait;
use bytes::Bytes;
use common::{retry, CollectedBytes};
use futures::Stream;
use jira_query::Issue;
use proto::semantics::JiraCollectorConfig;
use reqwest::Client;
use tokio::io::AsyncRead;

use crate::{DataSource, SendableAsync, SourceError, SourceErrorKind, SourceResult};

#[derive(Debug, Clone)]
pub struct BaseHttpClient {
	pub base_api_url: String,
	pub access_token: String,
	pub data_partition_id: String,
}

enum Param {
	Path(String),
	Query(HashMap<String, String>),
}

impl BaseHttpClient {
	pub fn new(base_api_url: &str, access_token: &str) -> BaseHttpClient {
		BaseHttpClient {
			base_api_url: base_api_url.to_string(),
			access_token: access_token.to_string(),
			data_partition_id: "osdu".to_string(),
		}
	}

	fn construct_headers(&self) -> reqwest::header::HeaderMap {
		let mut headers = reqwest::header::HeaderMap::new();
		headers.insert("AUTHORIZATION", format!("Bearer {}", self.access_token).parse().unwrap());
		headers.insert("CONTENT_TYPE", "application/json".parse().unwrap());
		headers.insert("ACCEPT", "application/json".parse().unwrap());
		headers.insert("data-partition-id", self.data_partition_id.parse().unwrap());
		headers
	}

	async fn get_request(&self, service_path: &str, param: Param) -> reqwest::Response {
		let request_url =
			format!("{}{}/{}", &self.base_api_url, service_path, get_url_params(param));
		println!("{}", request_url);
		let client = reqwest::Client::new();
		let response =
			client.get(request_url).headers(self.construct_headers()).send().await.unwrap();
		response
	}
}

fn get_url_params(param: Param) -> String {
	match param {
		Param::Path(path_param) => return path_param,
		Param::Query(query_params) => {
			let mut url_path = String::new();
			for (key, value) in query_params.into_iter() {
				url_path += &(key + "?" + &value);
			}
			return url_path;
		},
	}
}

#[derive(Clone, Debug)]
pub struct OSDUSource {
	pub service_path: String,
	pub http_client: BaseHttpClient,
}
