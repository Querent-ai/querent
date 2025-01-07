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

use reqwest::{header::HeaderMap, Client as HttpClient, Response};
use serde::Deserialize;
use std::{collections::HashMap, fmt::Debug};
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
	) -> OSDUClient {
		let service_account_key =
			parse_service_account_key(svc_access_key).expect("Failed to parse service account key");
		let auth: Authenticator<
			HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
		> = ServiceAccountAuthenticator::builder(service_account_key)
			.build()
			.await
			.expect("Failed to create authenticator");
		OSDUClient {
			base_api_url: base_api_url.to_string(),
			service_path: service_path.to_string(),
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

	pub async fn get_request(&mut self, param: Param) -> Result<Response, reqwest::Error> {
		let request_url =
			format!("{}{}/{}", self.base_api_url, self.service_path, get_url_params(param));
		let client = HttpClient::new();
		client.get(&request_url).headers(self.construct_headers().await).send().await
	}

	// For deployment available public /info endpoint, which provides build and git related information.
	pub async fn get_info(&mut self) -> Result<Response, reqwest::Error> {
		self.get_request(Param::Path("info".to_string())).await
	}

	// Page through all schemas in the OSDU service
	pub async fn get_paginated_schemas(&mut self) -> Result<Vec<SchemaInfo>, reqwest::Error> {
		let mut schemas = Vec::new();
		let mut offset = 0;
		let page_size = 100;

		loop {
			let request_url = format!(
				"{}{}/schema?offset={}&limit={}",
				self.base_api_url, self.service_path, offset, page_size
			);
			let client = HttpClient::new();
			let response =
				client.get(&request_url).headers(self.construct_headers().await).send().await?;

			let schema_response: SchemaResponse = response.json().await?;
			schemas.extend(schema_response.schema_infos.clone());

			if schema_response.schema_infos.len() < page_size {
				break;
			}

			offset += page_size;
		}

		Ok(schemas)
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
