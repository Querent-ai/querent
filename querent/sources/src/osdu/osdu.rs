use reqwest::{header::HeaderMap, Client as HttpClient, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use yup_oauth2::{
	authenticator::Authenticator, hyper_rustls::HttpsConnector, parse_service_account_key,
	AccessToken, ServiceAccountAuthenticator,
};

// These are the main OSDU data types:

// Reference Data - These are the standard naming for the data values. For example, the reference value for measured depth is MD and for elevation is ELEV. Whenever these values are being used, the reference data must be first loaded in the OSDU platform. There are 3 governance levels for the reference data:
// Fixed - Pre-determined by agreement in OSDU forum and shall not be changed. This allows interoperability between companies.
// Open - Agreed by OSDU forum but companies may extend with custom values. Custom values shall not conflict with Forum values. This allows some level of interoperability between companies.
// Local - OSDU forum makes no declaration about the values and companies need to create their own list. This list does not benefit much from interoperability and agreed-upon values are hard to come by.
// Master Data - A record of the information about business objects that we manage in the OSDU record catalog. For example, a list of field names with well names and their associated wellbore names.
// Work Product - A record that ties together a set of work product components such as a group of well logs inside a wellbore.
// Work Product Components - A record that describes the business content of a single well log, such as the log data information, top, bottom depth of the well log.
// Here is the list of the supported bulk standards in OSDU.
// File - A record that describes the metadata about the digital files, but does not describe the business content of the file, such as the file size, checksum of a well log.

pub struct BaseHttpClient {
	pub base_api_url: String,
	pub data_partition_id: String,
	pub x_collaboration: String,
	pub correlation_id: String,
	pub scopes: Vec<String>,
	access_token: Option<AccessToken>,
	auth: Authenticator<HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>,
}

impl BaseHttpClient {
	pub async fn new(
		base_api_url: &str,
		data_partition_id: &str,
		x_collaboration: &str,
		correlation_id: &str,
		svc_access_key: &str,
		scopes: Vec<String>,
	) -> BaseHttpClient {
		let service_account_key = parse_service_account_key(svc_access_key).unwrap();
		let auth: Authenticator<
			HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
		> = ServiceAccountAuthenticator::builder(service_account_key)
			.build()
			.await
			.expect("Failed to create authenticator");
		BaseHttpClient {
			base_api_url: base_api_url.to_string(),
			data_partition_id: data_partition_id.to_string(),
			x_collaboration: x_collaboration.to_string(),
			correlation_id: correlation_id.to_string(),
			auth,
			scopes,
			access_token: None,
		}
	}

	pub async fn construct_headers(&mut self) -> HeaderMap {
		// Note: many unwraps here assuming tokens work
		let mut headers = HeaderMap::new();
		if self.access_token.is_none() || self.access_token.as_ref().unwrap().is_expired() {
			self.access_token = Some(self.auth.token(self.scopes.as_slice()).await.unwrap());
		}
		headers.insert(
			"AUTHORIZATION",
			format!("Bearer {}", self.access_token.as_ref().unwrap().token().unwrap())
				.parse()
				.unwrap(),
		);
		headers.insert("CONTENT_TYPE", "application/json".parse().unwrap());
		headers.insert("ACCEPT", "application/json".parse().unwrap());
		headers.insert("data-partition-id", self.data_partition_id.parse().unwrap());
		headers.insert("x-collaboration", self.x_collaboration.parse().unwrap());
		headers.insert("Correlation-Id", self.correlation_id.parse().unwrap());
		headers
	}

	pub async fn get_request(
		&mut self,
		service_path: &str,
		param: Param,
	) -> Result<Response, reqwest::Error> {
		let request_url =
			format!("{}{}/{}", self.base_api_url, service_path, get_url_params(param));
		let client = HttpClient::new();
		client.get(&request_url).headers(self.construct_headers().await).send().await
	}
}

#[derive(Deserialize)]
pub struct RecordBase {
	pub id: String,
	pub version: u32,
	pub kind: String,
	pub acl: Acl,
	pub legal: Legal,
	pub data: serde_json::Value,
}

#[derive(Deserialize)]
pub struct Acl {
	pub viewers: Vec<String>,
	pub owners: Vec<String>,
}

#[derive(Deserialize)]
pub struct Legal {
	pub status: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StoreRecordResponse {
	pub record_count: i16,
	pub record_ids: Vec<String>,
}

pub enum Param {
	Path(String),
	Query(HashMap<String, String>),
}

fn get_url_params(param: Param) -> String {
	match param {
		Param::Path(path_param) => path_param,
		Param::Query(query_params) => query_params
			.into_iter()
			.map(|(key, value)| format!("{}={}", key, value))
			.collect::<Vec<_>>()
			.join("&"),
	}
}
