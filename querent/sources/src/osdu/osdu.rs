use reqwest::{header::HeaderMap, Client as HttpClient, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
	pub access_token: String,
	pub data_partition_id: String,
	pub x_collaboration: String,
}

impl BaseHttpClient {
	pub fn new(
		base_api_url: &str,
		access_token: &str,
		data_partition_id: &str,
		x_collaboration: &str,
	) -> BaseHttpClient {
		BaseHttpClient {
			base_api_url: base_api_url.to_string(),
			access_token: access_token.to_string(),
			data_partition_id: data_partition_id.to_string(),
			x_collaboration: x_collaboration.to_string(),
		}
	}

	pub fn construct_headers(&self) -> HeaderMap {
		let mut headers = HeaderMap::new();
		headers.insert("AUTHORIZATION", format!("Bearer {}", self.access_token).parse().unwrap());
		headers.insert("CONTENT_TYPE", "application/json".parse().unwrap());
		headers.insert("ACCEPT", "application/json".parse().unwrap());
		headers.insert("data-partition-id", self.data_partition_id.parse().unwrap());
		headers.insert("x-collaboration", self.x_collaboration.parse().unwrap());
		headers
	}

	pub async fn get_request(
		&self,
		service_path: &str,
		param: Param,
	) -> Result<Response, reqwest::Error> {
		let request_url =
			format!("{}{}/{}", self.base_api_url, service_path, get_url_params(param));
		let client = HttpClient::new();
		client.get(&request_url).headers(self.construct_headers()).send().await
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
