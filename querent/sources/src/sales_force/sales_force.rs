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

use std::{fmt::Debug, io::Cursor, ops::Range, path::Path, pin::Pin};

use crate::{DataSource, SendableAsync, SourceError, SourceErrorKind, SourceResult};
use async_stream::stream;
use async_trait::async_trait;
use common::{retry, CollectedBytes};
use futures::Stream;
use proto::semantics::SalesForceConfig;
use rustforce::{response::QueryResponse, Client};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncRead;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Account {
	#[serde(rename = "attributes")]
	attributes: Attribute,
	id: String,
	name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Attribute {
	url: String,
	#[serde(rename = "type")]
	sobject_type: String,
}

pub struct SalesForceApiClient {
	source_id: String,
	client: Client,
	config: SalesForceConfig,
	pub retry_params: common::RetryParams,
}

impl Debug for SalesForceApiClient {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SalesForceApiClient")
			.field("source_id", &self.source_id)
			.finish()
	}
}

impl Clone for SalesForceApiClient {
	fn clone(&self) -> Self {
		let client = Client::new(
			Some(self.config.client_id.clone()),
			Some(self.config.client_secret.clone()),
		);
		Self {
			source_id: self.source_id.clone(),
			client,
			config: self.config.clone(),
			retry_params: self.retry_params.clone(),
		}
	}
}

impl SalesForceApiClient {
	pub async fn new(config: SalesForceConfig) -> anyhow::Result<Self> {
		let config_clone = config.clone();
		let mut client = Client::new(Some(config.client_id), Some(config.client_secret));
		client.login_with_credential(config.username, config.password).await?;
		Ok(Self {
			source_id: config.id.clone(),
			client,
			config: config_clone,
			retry_params: common::RetryParams::default(),
		})
	}
}

fn string_to_async_read(description: String) -> impl AsyncRead + Send + Unpin {
	Cursor::new(description.into_bytes())
}

fn remove_special_characters(input: &str) -> String {
	input.chars().filter(|c| c.is_alphanumeric() || c.is_whitespace()).collect()
}

#[async_trait]
impl DataSource for SalesForceApiClient {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		Ok(())
	}

	async fn get_slice(&self, _path: &Path, _range: Range<usize>) -> SourceResult<Vec<u8>> {
		Ok(vec![])
	}

	async fn get_slice_stream(
		&self,
		_path: &Path,
		_range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		Ok(Box::new(string_to_async_read("".to_string())))
	}

	async fn get_all(&self, _path: &Path) -> SourceResult<Vec<u8>> {
		Ok(vec![])
	}

	async fn file_num_bytes(&self, _path: &Path) -> SourceResult<u64> {
		Ok(0)
	}

	async fn copy_to(&self, _path: &Path, _output: &mut dyn SendableAsync) -> SourceResult<()> {
		Ok(())
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'life0>>> {
		let base_query = self
			.config
			.query
			.clone()
			.unwrap_or_else(|| "SELECT Id, Name FROM Account".to_string()); // Use default if not provided
		let stream = stream! {
			let response = retry(&self.retry_params, || async {
				get_records(&self.client, &base_query).await
			}).await;
			let doc_source = format!("salesforce://{}", self.source_id);
			let file_name_path = format!("{}_{}.json", doc_source.clone(), self.config.username);
			let source_id = self.source_id.clone();
			match response {
				Ok(response) => {
					for record in response.records {
						let record = serde_json::to_string(&record).unwrap();
						let record = remove_special_characters(&record.clone());
						let len = record.len();
						yield Ok(CollectedBytes::new(
							Some(Path::new(&file_name_path).to_path_buf()),
							Some(Box::pin(string_to_async_read(record))),
							true,
							Some(doc_source.clone()),
							Some(len),
							source_id.clone(),
							None
						));
					}
				}
				Err(e) => {
					eprintln!("Error while getting the sales force records: {:?}", e);
					yield Ok(CollectedBytes::new(
						Some(Path::new(&file_name_path).to_path_buf()),
						Some(Box::pin(string_to_async_read("".to_string()))),
						true,
						Some(doc_source),
						Some(0),
						source_id.clone(),
						None
					));
				}
			}
		};
		Ok(Box::pin(stream))
	}
}

pub async fn get_records(
	client: &Client,
	query: &str,
) -> Result<QueryResponse<Account>, SourceError> {
	client.query_all(query).await.map_err(|err| {
		SourceError::new(
			SourceErrorKind::Io,
			anyhow::anyhow!("Error while getting the sales force records: {:?}", err).into(),
		)
	})
}
