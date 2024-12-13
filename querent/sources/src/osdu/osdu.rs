use async_trait::async_trait;
use aws_config::{meta::region::RegionProviderChain, Region};
use reqwest::{header::HeaderMap, Client as HttpClient, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub struct Client {
	pub storage: StorageService,
}

impl Client {
	pub async fn new(cloud_provider: &dyn CloudAuthProvider, base_api_url: &str) -> Client {
		let access_token = cloud_provider.get_access_token().await;
		Client { storage: StorageService::new(base_api_url, &access_token) }
	}
}

pub struct StorageService {
	pub service_path: String,
	pub http_client: BaseHttpClient,
}

impl StorageService {
	pub fn new(base_api_url: &str, access_token: &str) -> StorageService {
		StorageService {
			service_path: "/api/storage/v2/records".to_string(),
			http_client: BaseHttpClient::new(base_api_url, access_token),
		}
	}

	pub async fn get_record(&self, record_id: &str) -> Result<RecordBase, reqwest::Error> {
		let response = self
			.http_client
			.get_request(&self.service_path, Param::Path(record_id.to_string()))
			.await?;

		response.json::<RecordBase>().await
	}
}

pub struct BaseHttpClient {
	pub base_api_url: String,
	pub access_token: String,
	pub data_partition_id: String,
}

impl BaseHttpClient {
	pub fn new(base_api_url: &str, access_token: &str) -> BaseHttpClient {
		BaseHttpClient {
			base_api_url: base_api_url.to_string(),
			access_token: access_token.to_string(),
			data_partition_id: "osdu".to_string(),
		}
	}

	fn construct_headers(&self) -> HeaderMap {
		let mut headers = HeaderMap::new();
		headers.insert("AUTHORIZATION", format!("Bearer {}", self.access_token).parse().unwrap());
		headers.insert("CONTENT_TYPE", "application/json".parse().unwrap());
		headers.insert("ACCEPT", "application/json".parse().unwrap());
		headers.insert("data-partition-id", self.data_partition_id.parse().unwrap());
		headers
	}

	async fn get_request(
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

#[derive(Serialize, Deserialize, Debug)]
pub struct RecordBase {
	pub id: String,
	pub kind: String,
	pub data: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StoreRecordResponse {
	pub record_count: i16,
	pub record_ids: Vec<String>,
}

enum Param {
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

#[async_trait]
pub trait CloudAuthProvider {
	async fn get_access_token(&self) -> String;
}

// AWS Implementation
pub struct AwsAuthProvider {
	pub profile: String,
	pub region: &'static str,
	pub resource_prefix: String,
}

#[async_trait]
impl CloudAuthProvider for AwsAuthProvider {
	async fn get_access_token(&self) -> String {
		let region_provider =
			RegionProviderChain::default_provider().or_else(Region::new(self.region));
		let config = aws_config::from_env().region(region_provider).load().await;

		let client = aws_sdk_sts::Client::new(&config);

		// Option 1: Assume a Role
		let assumed_role = client
			.assume_role()
			.role_arn(format!("{}:role/{}", self.resource_prefix, "QuerentRole"))
			.role_session_name("rian-session")
			.send()
			.await;

		match assumed_role {
			Ok(response) => response.credentials.unwrap().session_token,
			Err(e) => {
				eprintln!("Error fetching AWS token: {}", e);
				"".to_string()
			},
		}

		// Option 2: Fetch temporary session token directly
		// let session_token = client.get_session_token().send().await.unwrap();
		// session_token.credentials.unwrap().session_token.unwrap_or_default()
	}
}

// Azure Implementation
pub struct AzureAuthProvider {
	pub client_id: String,
	pub client_secret: String,
	pub tenant_id: String,
}

#[async_trait]
impl CloudAuthProvider for AzureAuthProvider {
	async fn get_access_token(&self) -> String {
		let token_url =
			format!("https://login.microsoftonline.com/{}/oauth2/v2.0/token", self.tenant_id);

		let params = [
			("grant_type", "client_credentials"),
			("client_id", &self.client_id),
			("client_secret", &self.client_secret),
			("scope", "https://management.azure.com/.default"),
		];

		let client = HttpClient::new();
		let response = client.post(&token_url).form(&params).send().await.unwrap();

		let token_response: TokenResponse = response.json().await.unwrap();
		token_response.access_token
	}
}

// GCP Implementation
pub struct GcpAuthProvider {
	pub service_account_key: String,
}

#[async_trait]
impl CloudAuthProvider for GcpAuthProvider {
	async fn get_access_token(&self) -> String {
		// Implement GCP token retrieval logic here
		"gcp_access_token_placeholder".to_string() // Replace with real token
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenResponse {
	pub access_token: String,
	pub expires_in: i32,
}
