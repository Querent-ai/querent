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

use crate::{
	osdu::osdu::OSDUClient, resolve_ingestor_with_extension, string_to_async_read, DataSource,
	SendableAsync, SourceResult,
};
use async_trait::async_trait;
use common::{CollectedBytes, OsduFileGeneric};
use futures::Stream;
use proto::semantics::OsduServiceConfig;
use reqwest::Client;
use std::{
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
	sync::Arc,
};
use tokio::{io::AsyncRead, sync::Mutex};

#[derive(Debug)]
pub struct OSDUStorageService {
	pub config: OsduServiceConfig,
	pub osdu_storage_client: Arc<Mutex<OSDUClient>>,
	pub osdu_schema_client: Arc<Mutex<OSDUClient>>,
	pub osdu_file_client: Arc<Mutex<OSDUClient>>,
	pub retry_params: common::RetryParams,
}

impl OSDUStorageService {
	pub async fn new(config: OsduServiceConfig) -> anyhow::Result<Self> {
		let service_path = format!("/api/{}/{}", "storage", config.storage_version);
		let osdu_storage_client = OSDUClient::new(
			&config.base_url,
			&service_path,
			&config.data_partition_id,
			&config.x_collaboration.clone().unwrap_or_default(),
			&config.correlation_id.clone().unwrap_or_default(),
			&config.service_account_key,
			config.scopes.clone(),
		)
		.await?;

		let schema_service_path = format!("/api/{}/{}", "schema-service", config.schema_version);
		let osdu_schema_client = OSDUClient::new(
			&config.base_url,
			&schema_service_path,
			&config.data_partition_id,
			&config.x_collaboration.clone().unwrap_or_default(),
			&config.correlation_id.clone().unwrap_or_default(),
			&config.service_account_key,
			config.scopes.clone(),
		)
		.await?;

		let file_service_path = format!("/api/{}/{}", "file", config.file_version);
		let osdu_file_client = OSDUClient::new(
			&config.base_url,
			&file_service_path,
			&config.data_partition_id,
			&config.x_collaboration.clone().unwrap_or_default(),
			&config.correlation_id.clone().unwrap_or_default(),
			&config.service_account_key,
			config.scopes.clone(),
		)
		.await?;

		Ok(OSDUStorageService {
			config,
			osdu_storage_client: Arc::new(Mutex::new(osdu_storage_client)),
			osdu_schema_client: Arc::new(Mutex::new(osdu_schema_client)),
			osdu_file_client: Arc::new(Mutex::new(osdu_file_client)),
			retry_params: common::RetryParams::aggressive(),
		})
	}

	pub fn extract_file_size_from_record(record: &OsduFileGeneric) -> Option<usize> {
		// Try to parse TotalSize or FileSize as a number
		record.data.total_size.parse::<usize>().ok().or_else(|| {
			record.data.dataset_properties.file_source_info.file_size.parse::<usize>().ok()
		})
	}

	pub fn extract_file_extension_from_record(record: &OsduFileGeneric) -> Option<String> {
		// If no extension found from name, try to get it from EncodingFormatTypeID
		// This would require mapping the EncodingFormatTypeID to common extensions
		map_encoding_format_to_extension(&record.data.encoding_format_type_id).or_else(|| {
			map_encoding_format_to_extension(
				&record.data.dataset_properties.file_source_info.encoding_format_type_id,
			)
		})
	}
}

#[async_trait]
impl DataSource for OSDUStorageService {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		self.osdu_storage_client.lock().await.get_info().await?;
		if self.config.record_kinds.len() == 0 {
			// Only need schema client if no record kinds are specified
			self.osdu_schema_client.lock().await.get_info().await?;
		}
		self.osdu_file_client.lock().await.get_info().await?;
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
		let mut schema_client = self.osdu_schema_client.lock().await;
		let mut storage_client = self.osdu_storage_client.lock().await;
		let mut file_client = self.osdu_file_client.lock().await;
		let record_kinds = self.config.record_kinds.clone();
		let retry_params = self.retry_params.clone();
		let stream = async_stream::stream! {
			for kind_local in record_kinds.iter() {
				let record_kinds = vec![kind_local.name.clone()];
				let mut schema_fetch_receiver =
					schema_client.fetch_all_kinds(record_kinds, None, retry_params).await?;
				while let Some(kind) = schema_fetch_receiver.recv().await {
					let mut records_id_fetch_receiver =
						storage_client.fetch_record_ids_from_kind(&kind.clone(), None, retry_params).await?;
					while let Some(record_id) = records_id_fetch_receiver.recv().await {
						let mut records = storage_client.fetch_records_by_ids(vec![record_id.clone()], vec![], retry_params).await?;
						while let Some(record) = records.recv().await {
							let mut file_extension = kind_local.clone().file_extension.clone();
							let record_json_str = serde_json::to_string(&record)?;
							let size = record_json_str.len();
							let collected_bytes = CollectedBytes {
								data: Some(Box::pin(string_to_async_read(record_json_str))),
								file: Some(PathBuf::from(format!("osdu://record/{}", record_id.clone()))),
								doc_source: Some(format!("osdu://kind/{}", kind.clone()).to_string()),
								eof: true,
								extension: Some("json".to_string()),
								size: Some(size),
								source_id: record_id.clone(),
								_owned_permit: None,
								image_id: None,
							};

							yield Ok(collected_bytes);
							// Get file metadata from the record
							let file_size = OSDUStorageService::extract_file_size_from_record(&record);
							// If file extention is not known and size is less than 10MB then consider reading it as text
							if file_extension.is_empty(){
								file_extension = "txt".to_string();
							}
							if let Ok(signed_url_res) = file_client.get_signed_url(record_id.clone().as_str(), None, retry_params).await {
								if !signed_url_res.is_empty() {
									// Create a client to download the file
									let client = Client::new();
									// Start downloading the file as a stream
									match client.get(&signed_url_res)
										.send()
										.await {
										Ok(response) => {
											if response.status().is_success() {
												// Convert the response body into an AsyncRead
												let stream = response.bytes_stream();
												let reader = common::BytesStreamReader::new(stream);
												let file_collected_bytes = CollectedBytes {
													data: Some(Box::pin(reader)),
													file: Some(PathBuf::from(format!("osdu://file/{}.{}", record_id.clone(), file_extension))),
													doc_source: Some(format!("osdu://kind/{}", kind.clone()).to_string()),
													eof: true,
													extension: Some(file_extension),
													size: file_size,
													source_id: format!("{}", record_id.clone()),
													_owned_permit: None,
													image_id: None,
												};
												yield Ok(file_collected_bytes);
											} else {
												log::warn!("Failed to download file from signed URL for record {}: HTTP {}",
														   record_id, response.status());
											}
										},
										Err(e) => {
											log::error!("Error downloading file from signed URL for record {}: {}", record_id, e);
										}
									}
								}
							}
						}
					}
				}
			}
		};
		Ok(Box::pin(stream))
	}
}

fn map_encoding_format_to_extension(encoding_format_id: &str) -> Option<String> {
	let support_format = resolve_ingestor_with_extension(encoding_format_id);
	if support_format.is_ok() {
		return Some(support_format.unwrap());
	}
	None
}
