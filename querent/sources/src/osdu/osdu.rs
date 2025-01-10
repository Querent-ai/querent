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

use common::retry;
use reqwest::{header::HeaderMap, Client as HttpClient, Response};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug};
use tokio::sync::mpsc;
use yup_oauth2::{
	authenticator::Authenticator, hyper_rustls::HttpsConnector, parse_service_account_key,
	AccessToken, ServiceAccountAuthenticator,
};

use crate::{SourceError, SourceErrorKind};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StoreRecordResponse {
	pub record_count: i16,
	pub record_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RecordQueryResponse {
	pub results: Vec<String>,
	pub cursor: String,
}

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

#[derive(Clone)]
pub struct OSDUClient {
	pub base_api_url: String,
	pub service_path: String,
	pub data_partition_id: String,
	pub x_collaboration: String,
	pub correlation_id: String,
	pub scopes: Vec<String>,
	access_token: Option<AccessToken>,
	auth: Authenticator<HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>,
}

impl Debug for OSDUClient {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "OSDUClient")
	}
}

impl OSDUClient {
	pub async fn new(
		base_api_url: &str,
		service_path: &str,
		data_partition_id: &str,
		x_collaboration: &str,
		correlation_id: &str,
		svc_access_key: &str,
		scopes: Vec<String>,
	) -> anyhow::Result<OSDUClient> {
		let service_account_key =
			parse_service_account_key(svc_access_key).map_err(|e| anyhow::anyhow!(e))?;
		let auth: Authenticator<
			HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
		> = ServiceAccountAuthenticator::builder(service_account_key).build().await?;
		Ok(OSDUClient {
			base_api_url: base_api_url.to_string(),
			service_path: service_path.to_string(),
			data_partition_id: data_partition_id.to_string(),
			x_collaboration: x_collaboration.to_string(),
			correlation_id: correlation_id.to_string(),
			auth,
			scopes,
			access_token: None,
		})
	}

	pub async fn construct_headers(&mut self) -> Result<HeaderMap, SourceError> {
		let mut headers = HeaderMap::new();

		if self.access_token.is_none() ||
			self.access_token.as_ref().map(|token| token.is_expired()).unwrap_or(true)
		{
			self.access_token = Some(self.auth.token(&self.scopes).await.map_err(|e| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Failed to get token: {:?}", e).into(),
				)
			})?);
		}

		if let Some(ref token) = self.access_token {
			headers.insert(
				"AUTHORIZATION",
				format!("Bearer {}", token.token().unwrap_or_default()).parse().map_err(|e| {
					SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Failed to parse token: {:?}", e).into(),
					)
				})?,
			);
		}

		headers.insert(
			"CONTENT_TYPE",
			"application/json".parse().map_err(|e| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Failed to parse token: {:?}", e).into(),
				)
			})?,
		);
		headers.insert(
			"ACCEPT",
			"application/json".parse().map_err(|e| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Failed to parse token: {:?}", e).into(),
				)
			})?,
		);
		headers.insert(
			"data-partition-id",
			self.data_partition_id.parse().map_err(|e| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Failed to parse token: {:?}", e).into(),
				)
			})?,
		);
		headers.insert(
			"x-collaboration",
			self.x_collaboration.parse().map_err(|e| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Failed to parse token: {:?}", e).into(),
				)
			})?,
		);
		headers.insert(
			"Correlation-Id",
			self.correlation_id.parse().map_err(|e| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Failed to parse token: {:?}", e).into(),
				)
			})?,
		);

		Ok(headers)
	}

	pub async fn get_request(&mut self, param: Param) -> Result<Response, SourceError> {
		let request_url =
			format!("{}{}/{}", self.base_api_url, self.service_path, get_url_params(param));
		let client = HttpClient::new();
		let headers = self.construct_headers().await?;
		client.get(&request_url).headers(headers).send().await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while making request to Notion API: {:?}", err).into(),
			)
		})
	}

	// For deployment available public /info endpoint, which provides build and git related information.
	pub async fn get_info(&mut self) -> Result<Response, SourceError> {
		self.get_request(Param::Path("info".to_string())).await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while making request to Notion API: {:?}", err).into(),
			)
		})
	}

	pub async fn fetch_all_kinds(
		&mut self,
		kinds: Vec<String>,
		filters: Option<HashMap<String, String>>,
		retry_params: common::RetryParams,
	) -> Result<mpsc::Receiver<String>, SourceError> {
		let mut offset = 0;
		let page_size = 10;
		let client: HttpClient = HttpClient::new();
		let (tx, rx) = mpsc::channel(100);
		let url = format!("{}{}", self.base_api_url, self.service_path);
		let mut query_params = HashMap::new();

		// Add the filters to the query parameters
		if let Some(f) = filters {
			query_params.extend(f);
		}
		let mut self_clone = self.clone();

		tokio::spawn(async move {
			loop {
				let headers = self_clone.construct_headers().await;
				if let Err(err) = headers {
					eprintln!("Error while constructing OSDU headers: {:?}", err);
					break;
				}
				let headers = headers.unwrap();
				let request_url = format!(
					"{}/schema?offset={}&limit={}&{}",
					url,
					offset,
					page_size,
					get_url_params(Param::Query(query_params.clone()))
				);
				if !kinds.is_empty() {
					// sends kind back to the caller and returns
					for kind in &kinds {
						if tx.send(kind.clone()).await.is_err() {
							break;
						}
					}
					break;
				}
				let response = retry(&retry_params, || async {
					client.get(&request_url).headers(headers.clone()).send().await.map_err(|err| {
						SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Error while making request to Notion API: {:?}", err)
								.into(),
						)
					})
				})
				.await;
				match response {
					Ok(response) => {
						if !response.status().is_success() {
							break;
						}

						match response.json::<SchemaResponse>().await {
							Ok(schema_response) => {
								let count = schema_response.schema_infos.len();
								for schema_info in &schema_response.schema_infos {
									if tx
										.send(schema_info.schema_identity.id.clone())
										.await
										.is_err()
									{
										break;
									}
								}
								if count < page_size {
									break;
								}
							},
							Err(_) => break,
						}
					},
					Err(_) => break,
				}

				offset += page_size;
			}
		});

		Ok(rx)
	}

	pub async fn fetch_record_ids_from_kind(
		&mut self,
		kind_id: &str,
		filters: Option<HashMap<String, String>>,
		retry_params: common::RetryParams,
	) -> Result<mpsc::Receiver<String>, SourceError> {
		let mut cursor = String::new();
		let page_size = 10;
		let client: HttpClient = HttpClient::new();
		let (tx, rx) = mpsc::channel(100);
		let url = format!("{}{}", self.base_api_url, self.service_path);
		let mut query_params = HashMap::new();
		let kind_id = kind_id.to_string();
		// Add the filters to the query parameters
		if let Some(f) = filters {
			query_params.extend(f);
		}
		let mut self_clone = self.clone();

		tokio::spawn(async move {
			loop {
				let headers = self_clone.construct_headers().await;
				if let Err(err) = headers {
					eprintln!("Error while constructing OSDU headers: {:?}", err);
					break;
				}
				let headers = headers.unwrap();
				let request_url = format!(
					"{}/query/records?kind={}&cursor={}&limit={}&{}",
					url,
					kind_id.clone(),
					cursor,
					page_size,
					get_url_params(Param::Query(query_params.clone()))
				);
				let response = retry(&retry_params, || async {
					client.get(&request_url).headers(headers.clone()).send().await.map_err(|err| {
						SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Error while making request to Notion API: {:?}", err)
								.into(),
						)
					})
				})
				.await;
				match response {
					Ok(response) => {
						if !response.status().is_success() {
							continue;
						}

						match response.json::<RecordQueryResponse>().await {
							Ok(record_response) => {
								let count = record_response.results.len();
								for record_id in &record_response.results {
									if tx.send(record_id.clone()).await.is_err() {
										break;
									}
								}
								if count < page_size || record_response.cursor.is_empty() {
									break;
								}
								cursor = record_response.cursor;
							},
							Err(_) => break,
						}
					},
					Err(_) => break,
				}
			}
		});

		Ok(rx)
	}

	pub async fn fetch_records_by_ids(
		&mut self,
		record_ids: Vec<String>,
		attributes: Vec<String>,
		retry_params: common::RetryParams,
	) -> Result<mpsc::Receiver<Record>, SourceError> {
		let client: HttpClient = HttpClient::new();
		let (tx, rx) = mpsc::channel(100);
		let url = format!("{}{}", self.base_api_url, self.service_path);
		let mut self_clone = self.clone();

		tokio::spawn(async move {
			let headers = self_clone.construct_headers().await;
			if let Err(err) = headers {
				eprintln!("Error while constructing OSDU headers: {:?}", err);
				return;
			}
			let headers = headers.unwrap();
			let request_url = format!("{}/query/records", url);
			let payload = FetchRecordsRequest { records: record_ids, attributes };
			// Post the request
			let response = retry(&retry_params, || async {
				client
					.post(&request_url)
					.headers(headers.clone())
					.json(&payload)
					.send()
					.await
					.map_err(|err| {
						SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Error while making request to Notion API: {:?}", err)
								.into(),
						)
					})
			})
			.await;
			match response {
				Ok(response) => {
					if !response.status().is_success() {
						return;
					}

					match response.json::<FetchRecordsResponse>().await {
						Ok(record_response) =>
							for record in record_response.records {
								if tx.send(record).await.is_err() {
									break;
								}
							},
						Err(_) => return,
					}
				},
				Err(_) => return,
			}
		});
		Ok(rx)
	}
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

#[derive(Debug, Deserialize, Clone)]
pub struct SchemaIdentity {
	pub authority: String,
	pub source: String,
	pub entity_type: String,
	pub schema_version_major: u32,
	pub schema_version_minor: u32,
	pub schema_version_patch: u32,
	pub id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SchemaInfo {
	pub schema_identity: SchemaIdentity,
	pub created_by: String,
	pub date_created: String,
	pub status: String,
	pub scope: String,
	pub superseded_by: Option<SchemaIdentity>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SchemaResponse {
	pub schema_infos: Vec<SchemaInfo>,
	pub offset: usize,
	pub count: usize,
	pub total_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Acl {
	pub viewers: Vec<String>,
	pub owners: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Legal {
	pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
	id: String,
	version: u32,
	kind: String,
	acl: Acl,
	legal: Legal,
	data: HashMap<String, serde_json::Value>,
	ancestry: HashMap<String, Vec<String>>,
	meta: Vec<HashMap<String, serde_json::Value>>,
	tags: HashMap<String, String>,
	create_user: String,
	create_time: String,
	modify_user: String,
	modify_time: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct FetchRecordsRequest {
	records: Vec<String>,
	attributes: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct FetchRecordsResponse {
	records: Vec<Record>,
	invalid_records: Vec<String>,
	retry_records: Vec<String>,
}
