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

use proto::semantics::OsduServiceConfig;
use serde::{Deserialize, Serialize};

use crate::osdu::osdu::OSDUClient;

#[derive(Debug)]
pub struct OSDUStorageService {
	pub config: OsduServiceConfig,
	pub osdu_client: OSDUClient,
	pub retry_params: common::RetryParams,
}

impl OSDUStorageService {
	pub async fn new(config: OsduServiceConfig) -> anyhow::Result<Self> {
		let service_path = format!("/api/{}/{}/", "storage", config.version);
		let osdu_client = OSDUClient::new(
			&config.base_url,
			&service_path,
			&config.data_partition_id,
			&config.x_collaboration.clone().unwrap_or_default(),
			&config.correlation_id.clone().unwrap_or_default(),
			&config.service_account_key,
			config.scopes.clone(),
		)
		.await;

		Ok(OSDUStorageService {
			config,
			osdu_client,
			retry_params: common::RetryParams::aggressive(),
		})
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
