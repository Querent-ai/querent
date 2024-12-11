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

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use std::{io::Cursor, ops::Range, path::Path, pin::Pin, sync::Arc};

use anyhow::{anyhow, Result};
use async_stream::stream;
use async_trait::async_trait;
use common::{retry, CollectedBytes, RetryParams};
use futures::{Stream, TryStreamExt as _};
use once_cell::sync::Lazy;
use onedrive_api::{
	Auth, ClientCredential, DriveLocation, ItemLocation, OneDrive, Permission, Tenant,
};
use proto::semantics::OneDriveConfig;
use reqwest::get;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio_util::io::StreamReader;
use tracing::instrument;

use crate::{
	default_copy_to_file, DataSource, SendableAsync, SourceError, SourceErrorKind, SourceResult,
	REQUEST_SEMAPHORE,
};

#[derive(Clone)]
pub struct OneDriveSource {
	onedrive: OneDrive,
	folder_path: String,
	source_id: String,
	retry_params: RetryParams,
}

pub static TOKEN: Lazy<tokio::sync::Mutex<Option<String>>> =
	Lazy::new(|| tokio::sync::Mutex::new(None));

impl OneDriveSource {
	pub async fn new(config: OneDriveConfig) -> anyhow::Result<Self> {
		let onedrive = Self::get_logined_onedrive(&config).await?;
		Ok(OneDriveSource {
			onedrive,
			folder_path: config.folder_path,
			source_id: config.id.clone(),
			retry_params: RetryParams::aggressive(),
		})
	}

	pub async fn get_logined_onedrive(config: &OneDriveConfig) -> Result<OneDrive, anyhow::Error> {
		let mut token_guard = TOKEN.lock().await;

		if token_guard.is_none() {
			let auth = Auth::new(
				config.client_id.clone(),
				Permission::new_read().write(true).offline_access(true),
				config.redirect_uri.clone(),
				Tenant::Consumers,
			);
			let token_response = auth
				.login_with_refresh_token(
					&config.refresh_token,
					&ClientCredential::Secret(config.client_secret.clone()),
				)
				.await;

			match token_response {
				Ok(response) => {
					*token_guard = Some(response.access_token.clone());
				},
				Err(e) => {
					eprintln!("Login failed: {}", e);
					return Err(anyhow!("Login failed: {}", e));
				},
			}
		}

		if let Some(token) = &*token_guard {
			Ok(OneDrive::new(token.clone(), DriveLocation::me()))
		} else {
			Err(anyhow!("Failed to obtain access token from OneDrive"))
		}
	}

	async fn download_file(
		retry_params: &RetryParams,
		url: &str,
	) -> Result<impl AsyncRead + Send + Unpin, SourceError> {
		let response = retry(retry_params, || async {
			get(url).await.map_err(|e| SourceError {
				kind: SourceErrorKind::Connection,
				source: Arc::new(anyhow!("Got error while downloading file: {}", e)),
			})
		})
		.await?;
		let body = response.bytes_stream();
		Ok(StreamReader::new(
			body.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)),
		))
	}

	fn get_file_extension(file_name: &str) -> Option<String> {
		Path::new(file_name)
			.extension()
			.and_then(|ext| ext.to_str().map(|s| s.to_string()))
	}
}

impl std::fmt::Debug for OneDriveSource {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("OneDriveSource")
			.field("folder_path", &self.folder_path)
			.finish()
	}
}

#[async_trait]
impl DataSource for OneDriveSource {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let folder_path = self.folder_path.clone();
		let folder_location = ItemLocation::from_path(&folder_path);

		if folder_location.is_none() {
			return Err(anyhow!("Invalid folder path: {}", self.folder_path));
		}

		let folder_metadata = self.onedrive.get_item(folder_location.unwrap()).await;

		match folder_metadata {
			Ok(_) => Ok(()),
			Err(e) => Err(anyhow!("Failed to verify folder existence: {}", e)),
		}
	}

	async fn copy_to_file(&self, path: &Path, output_path: &Path) -> SourceResult<u64> {
		default_copy_to_file(self, path, output_path).await
	}

	#[instrument(level = "debug", skip(self, range), fields(range.start = range.start, range.end = range.end))]
	async fn get_slice(&self, path: &Path, range: Range<usize>) -> SourceResult<Vec<u8>> {
		let drive_items = self
			.onedrive
			.list_children(ItemLocation::from_path(&self.folder_path).unwrap())
			.await
			.map_err(|e| SourceError { kind: SourceErrorKind::Io, source: Arc::new(e.into()) })?;
		for drive_item in drive_items {
			if let Some(_file) = &drive_item.file {
				if let Some(download_url) = &drive_item.download_url {
					let mut bytes_stream =
						Self::download_file(&self.retry_params, download_url).await?;
					let mut bytes = Vec::new();
					bytes_stream.read_to_end(&mut bytes).await.map_err(|e| SourceError {
						kind: SourceErrorKind::Io,
						source: Arc::new(anyhow!("Failed to read file: {}", e)),
					})?;
					let slice = bytes
						.get(range)
						.ok_or_else(|| SourceError {
							kind: SourceErrorKind::Io,
							source: Arc::new(anyhow!("Failed to get slice from bytes")),
						})?
						.to_vec();
					return Ok(slice);
				}
			}
		}
		Err(SourceError { kind: SourceErrorKind::Io, source: Arc::new(anyhow!("No files found")) })
	}

	#[instrument(level = "debug", skip(self, range), fields(range.start = range.start, range.end = range.end))]
	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let drive_items = self
			.onedrive
			.list_children(ItemLocation::from_path(&self.folder_path).unwrap())
			.await
			.map_err(|e| SourceError { kind: SourceErrorKind::Io, source: Arc::new(e.into()) })?;
		for drive_item in drive_items {
			if let Some(_file) = &drive_item.file {
				if let Some(download_url) = &drive_item.download_url {
					let mut bytes_stream =
						Self::download_file(&self.retry_params, download_url).await?;
					let mut bytes = Vec::new();
					bytes_stream.read_to_end(&mut bytes).await.map_err(|e| SourceError {
						kind: SourceErrorKind::Io,
						source: Arc::new(anyhow!("Failed to read file: {}", e)),
					})?;
					let slice = bytes
						.get(range)
						.ok_or_else(|| SourceError {
							kind: SourceErrorKind::Io,
							source: Arc::new(anyhow!("Failed to get slice from bytes")),
						})?
						.to_vec();
					return Ok(Box::new(Cursor::new(slice)));
				}
			}
		}
		Err(SourceError { kind: SourceErrorKind::Io, source: Arc::new(anyhow!("No files found")) })
	}

	async fn get_all(&self, _path: &Path) -> SourceResult<Vec<u8>> {
		let drive_items = self
			.onedrive
			.list_children(ItemLocation::from_path(&self.folder_path).unwrap())
			.await
			.map_err(|e| SourceError { kind: SourceErrorKind::Io, source: Arc::new(e.into()) })?;
		for drive_item in drive_items {
			if let Some(_file) = &drive_item.file {
				if let Some(download_url) = &drive_item.download_url {
					let mut bytes_stream =
						Self::download_file(&self.retry_params, download_url).await?;
					let mut bytes = Vec::new();
					bytes_stream.read_to_end(&mut bytes).await.map_err(|e| SourceError {
						kind: SourceErrorKind::Io,
						source: Arc::new(anyhow!("Failed to read file: {}", e)),
					})?;
					return Ok(bytes);
				}
			}
		}
		Err(SourceError { kind: SourceErrorKind::Io, source: Arc::new(anyhow!("No files found")) })
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'life0>>> {
		let drive_items = self
			.onedrive
			.list_children(ItemLocation::from_path(&self.folder_path).unwrap())
			.await
			.map_err(|e| SourceError { kind: SourceErrorKind::Io, source: Arc::new(e.into()) })?;
		let source_id = self.source_id.clone();

		let stream = stream! {
			for drive_item in drive_items {
				let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
				if let Some(_file) = &drive_item.file {
					let name = drive_item.name.clone().unwrap_or_default();
					let extension = Self::get_file_extension(&name);
					let size = drive_item.size.unwrap_or(0);
					if let Some(download_url) = &drive_item.download_url {
						let bytes_stream = Self::download_file(&self.retry_params, download_url).await?;
						yield Ok(CollectedBytes {
							data: Some(Box::pin(bytes_stream)),
							file: Some(Path::new(&name).to_path_buf()),
							eof: true,
							doc_source: Some("onedrive://".to_string()),
							extension: extension.clone(),
							size: Some(size as usize),
							source_id: source_id.clone(),
							_owned_permit: None,
							image_id: None,
						});
					}
				}
			}
		};
		Ok(Box::pin(stream))
	}

	async fn file_num_bytes(&self, _path: &Path) -> SourceResult<u64> {
		let drive_items = self
			.onedrive
			.list_children(ItemLocation::from_path(&self.folder_path).unwrap())
			.await
			.map_err(|e| SourceError { kind: SourceErrorKind::Io, source: Arc::new(e.into()) })?;
		for drive_item in drive_items {
			if let Some(_file) = &drive_item.file {
				if let Some(download_url) = &drive_item.download_url {
					let mut bytes_stream =
						Self::download_file(&self.retry_params, download_url).await?;
					let mut bytes = Vec::new();
					bytes_stream.read_to_end(&mut bytes).await.map_err(|e| SourceError {
						kind: SourceErrorKind::Io,
						source: Arc::new(anyhow!("Failed to read file: {}", e)),
					})?;
					return Ok(bytes.len() as u64);
				}
			}
		}
		Err(SourceError { kind: SourceErrorKind::Io, source: Arc::new(anyhow!("No files found")) })
	}

	async fn copy_to(&self, _path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let drive_items = self
			.onedrive
			.list_children(ItemLocation::from_path(&self.folder_path).unwrap())
			.await
			.map_err(|e| SourceError { kind: SourceErrorKind::Io, source: Arc::new(e.into()) })?;
		for drive_item in drive_items {
			if let Some(_file) = &drive_item.file {
				if let Some(download_url) = &drive_item.download_url {
					let mut bytes_stream =
						Self::download_file(&self.retry_params, download_url).await?;
					let mut bytes = Vec::new();
					bytes_stream.read_to_end(&mut bytes).await.map_err(|e| SourceError {
						kind: SourceErrorKind::Io,
						source: Arc::new(anyhow!("Failed to read file: {}", e)),
					})?;
					output.write_all(&bytes).await.map_err(|e| SourceError {
						kind: SourceErrorKind::Io,
						source: Arc::new(anyhow!("Got error while writing to output: {}", e)),
					})?;
				}
			}
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::{onedrive::onedrive::OneDriveSource, DataSource};
	use dotenv::dotenv;
	use futures::StreamExt;
	use proto::semantics::OneDriveConfig;
	use serial_test::serial;
	use std::{collections::HashSet, env};

	use super::TOKEN;

	async fn reset_token() {
		let mut token_guard = TOKEN.lock().await;
		*token_guard = None;
	}

	#[tokio::test]
	#[serial]
	async fn test_onedrive_collector() {
		dotenv().ok();
		reset_token().await;
		let onedrive_config = OneDriveConfig {
			client_id: env::var("ONEDRIVE_CLIENT_ID").unwrap_or_else(|_| "".to_string()),
			client_secret: env::var("ONEDRIVE_CLIENT_SECRET").unwrap_or_else(|_| "".to_string()),
			redirect_uri: "http://localhost:8000/callback".to_string(),
			refresh_token: env::var("ONEDRIVE_REFRESH_TOKEN").unwrap_or_else(|_| "".to_string()),
			folder_path: "/testing".to_string(),
			id: "test".to_string(),
		};

		let drive_storage = OneDriveSource::new(onedrive_config).await.unwrap();
		let connectivity = drive_storage.check_connectivity().await;

		assert!(connectivity.is_ok(), "Expected connectivity to work");

		let result = drive_storage.poll_data().await;

		let mut stream = result.unwrap();
		let mut count_files: HashSet<String> = HashSet::new();
		while let Some(item) = stream.next().await {
			match item {
				Ok(collected_bytes) =>
					if let Some(pathbuf) = collected_bytes.file {
						if let Some(str_path) = pathbuf.to_str() {
							count_files.insert(str_path.to_string());
						}
					},
				Err(err) => eprintln!("Expected successful data collection {:?}", err),
			}
		}
		assert!(count_files.len() > 0, "Expected successful data polling");
	}

	//The given test runs when we run it individually but it fails when we run it using cargo test. The reason is TOKEN value does not gets reset properly
	#[tokio::test]
	#[serial]
	async fn test_onedrive_collector_invalid_credentials() {
		dotenv().ok();
		reset_token().await;
		let onedrive_config = OneDriveConfig {
			client_id: "invalid_client_id".to_string(),
			client_secret: "invalid_client_secret".to_string(),
			redirect_uri: "http://localhost:8000/callback".to_string(),
			refresh_token: "invalid_refresh_token".to_string(),
			folder_path: "/testing".to_string(),
			id: "test".to_string(),
		};

		let drive_storage = OneDriveSource::new(onedrive_config).await;

		// println!("Drive storage is {:?}", drive_storage.err());

		assert!(drive_storage.is_err(), "Expected drive storage to fail");
	}

	#[tokio::test]
	#[serial]
	async fn test_onedrive_collector_invalid_folder() {
		dotenv().ok();
		let onedrive_config = OneDriveConfig {
			client_id: env::var("ONEDRIVE_CLIENT_ID").unwrap_or_else(|_| "".to_string()),
			client_secret: env::var("ONEDRIVE_CLIENT_SECRET").unwrap_or_else(|_| "".to_string()),
			redirect_uri: "http://localhost:8000/callback".to_string(),
			refresh_token: env::var("ONEDRIVE_REFRESH_TOKEN").unwrap_or_else(|_| "".to_string()),
			folder_path: "/invalid_folder_path-1".to_string(),
			id: "test".to_string(),
		};

		let drive_storage = OneDriveSource::new(onedrive_config).await.unwrap();

		let connectivity = drive_storage.check_connectivity().await;
		// println!("Error is {:?}", connectivity.err());

		assert!(connectivity.is_err(), "Expected drive storage to fail");
	}

	#[tokio::test]
	#[serial]
	async fn test_onedrive_collector_empty_folder() {
		dotenv().ok();
		let onedrive_config = OneDriveConfig {
			client_id: env::var("ONEDRIVE_CLIENT_ID").unwrap_or_else(|_| "".to_string()),
			client_secret: env::var("ONEDRIVE_CLIENT_SECRET").unwrap_or_else(|_| "".to_string()),
			redirect_uri: "http://localhost:8000/callback".to_string(),
			refresh_token: env::var("ONEDRIVE_REFRESH_TOKEN").unwrap_or_else(|_| "".to_string()),
			folder_path: "/empty-folder".to_string(),
			id: "test".to_string(),
		};

		let drive_storage = OneDriveSource::new(onedrive_config).await.unwrap();
		let result = drive_storage.poll_data().await;
		assert!(result.is_ok(), "Expected successful connectivity");

		let mut stream = result.unwrap();
		let mut count_files: HashSet<String> = HashSet::new();
		while let Some(item) = stream.next().await {
			match item {
				Ok(collected_bytes) =>
					if let Some(pathbuf) = collected_bytes.file {
						if let Some(str_path) = pathbuf.to_str() {
							count_files.insert(str_path.to_string());
						}
					},
				Err(err) => eprintln!("Expected successful data collection {:?}", err),
			}
		}
		assert!(count_files.len() == 0, "Expected zero data");
	}
}
