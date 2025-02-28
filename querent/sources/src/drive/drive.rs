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

use async_stream::stream;
use async_trait::async_trait;
use common::{retry, CollectedBytes, RetryParams};
use futures::{Stream, StreamExt, TryStreamExt};
use google_drive3::{
	api::Scope,
	hyper::{self, client::HttpConnector},
	hyper_rustls::{HttpsConnector, HttpsConnectorBuilder},
	oauth2::{authorized_user::AuthorizedUserSecret, AuthorizedUserAuthenticator},
};
use hyper::Body;
use proto::semantics::GoogleDriveCollectorConfig;
use std::{
	io::Cursor,
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
};
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio_util::io::StreamReader;
use tracing::instrument;

use crate::{
	DataSource, SendableAsync, SourceError, SourceErrorKind, SourceResult, REQUEST_SEMAPHORE,
};

type DriveHub = google_drive3::DriveHub<HttpsConnector<HttpConnector>>;

pub const FIELDS: &str =
	"mimeType,id,kind,teamDriveId,name,driveId,description,size,md5Checksum,parents,trashed";

#[derive(Clone)]
pub struct GoogleDriveSource {
	hub: DriveHub,
	folder_id: String,
	page_token: Option<String>,
	source_id: String,
	retry_params: RetryParams,
}

impl std::fmt::Debug for GoogleDriveSource {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("GoogleDriveSource").field("folder_id", &self.folder_id).finish()
	}
}

impl GoogleDriveSource {
	pub async fn new(config: GoogleDriveCollectorConfig) -> Self {
		let auth = AuthorizedUserSecret {
			client_id: config.drive_client_id,
			client_secret: config.drive_client_secret,
			refresh_token: config.drive_refresh_token,
			key_type: "".to_string(),
		};

		let connector = HttpsConnectorBuilder::new()
			.with_native_roots()
			.expect("Failed to configure HTTPS connector with native roots")
			.https_or_http()
			.enable_http1()
			.enable_http2()
			.build();
		let auth = AuthorizedUserAuthenticator::builder(auth)
			.build()
			.await
			.expect("Failed to build authenticator");
		let http_client = hyper::Client::builder().build(connector);
		let hub = DriveHub::new(http_client, auth);

		GoogleDriveSource {
			hub,
			folder_id: config.folder_to_crawl,
			page_token: None,
			source_id: config.id.clone(),
			retry_params: RetryParams::aggressive(),
		}
	}

	async fn download_file(&self, file_id: &str) -> Result<Body, google_drive3::Error> {
		let (resp_obj, file) = self
			.hub
			.files()
			.get(file_id)
			.supports_all_drives(true)
			.acknowledge_abuse(false)
			.param("fields", FIELDS)
			.param("alt", "media")
			.add_scope(Scope::Readonly)
			.doit()
			.await?;

		let mime_type = file.mime_type.unwrap_or_default();
		if mime_type == "application/vnd.google-apps.folder" {
			return Err(google_drive3::Error::FieldClash("Cannot download a folder"));
		}

		if mime_type.starts_with("application/vnd.google-apps.") {
			match mime_type.as_str() {
				"application/vnd.google-apps.document" => "application/pdf",
				"application/vnd.google-apps.spreadsheet" =>
					"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
				"application/vnd.google-apps.presentation" =>
					"application/vnd.openxmlformats-officedocument.presentationml.presentation",
				_ => {
					return Err(google_drive3::Error::FieldClash(
						"Unsupported Google Apps file type",
					));
				},
			};
			Ok(resp_obj.into_body())
		} else {
			Ok(resp_obj.into_body())
		}
	}

	async fn get_file_id_by_path(&self, path: &Path) -> Result<String, SourceError> {
		let mut query: String = format!("'{}' in parents", self.folder_id);
		for component in path.iter() {
			query.push_str(&format!(" and name = '{}'", component.to_string_lossy()));
		}
		let (_, files) = self.hub.files().list().q(&query).doit().await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error querying files in Google Drive: {:?}", err).into(),
			)
		})?;
		if let Some(files) = files.files.into_iter().next() {
			if files.len() > 1 {
				log::warn!("Multiple files found in Google Drive for path {:?}", path);
			}
			Ok(files[0].id.clone().unwrap())
		} else {
			Err(SourceError::new(
				SourceErrorKind::NotFound,
				anyhow::anyhow!("File not found in Google Drive").into(),
			))
		}
	}
}

#[async_trait]
impl DataSource for GoogleDriveSource {
	async fn copy_to(&self, path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let file_id = self.get_file_id_by_path(path).await?;
		let mut content_body = self.download_file(&file_id).await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error downloading file from Google Drive: {:?}", err).into(),
			)
		})?;
		while let Some(chunk) = content_body.next().await {
			let chunk = chunk.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error reading chunk from Google Drive: {:?}", err).into(),
				)
			})?;
			output.write_all(&chunk).await.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error writing chunk to output: {:?}", err).into(),
				)
			})?;
		}
		Ok(())
	}

	async fn check_connectivity(&self) -> anyhow::Result<()> {
		self.hub.files().list().page_size(1).doit().await?;
		Ok(())
	}

	#[instrument(level = "debug", skip(self, range), fields(range.start = range.start, range.end = range.end))]
	async fn get_slice(&self, path: &Path, range: Range<usize>) -> SourceResult<Vec<u8>> {
		let file_id = self.get_file_id_by_path(path).await?;
		let mut content_body = self.download_file(&file_id).await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error downloading file from Google Drive: {:?}", err).into(),
			)
		})?;
		let mut buffer = vec![0; range.len()];
		while let Some(chunk) = content_body.next().await {
			let chunk = chunk.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error reading chunk from Google Drive: {:?}", err).into(),
				)
			})?;
			let chunk = chunk.as_ref();
			if range.start < chunk.len() {
				let start = range.start;
				let end = std::cmp::min(range.end, chunk.len());
				let chunk = &chunk[start..end];
				buffer[..chunk.len()].copy_from_slice(chunk);
			}
		}
		Ok(buffer)
	}

	#[instrument(level = "debug", skip(self, range), fields(range.start = range.start, range.end = range.end))]
	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let file_id = self.get_file_id_by_path(path).await?;
		let mut content_body = self.download_file(&file_id).await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error downloading file from Google Drive: {:?}", err).into(),
			)
		})?;

		let buffer = vec![0; range.len()];
		let mut cursor = Cursor::new(buffer);
		while let Some(chunk) = content_body.next().await {
			let chunk = chunk.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error reading chunk from Google Drive: {:?}", err).into(),
				)
			})?;
			let chunk = chunk.as_ref();
			if range.start < chunk.len() {
				let start = range.start;
				let end = std::cmp::min(range.end, chunk.len());
				let chunk = &chunk[start..end];
				cursor.write_all(chunk).await.map_err(|err| {
					SourceError::new(
						SourceErrorKind::Io,
						anyhow::anyhow!("Error writing chunk to cursor: {:?}", err).into(),
					)
				})?;
			}
		}

		cursor.set_position(0);
		Ok(Box::new(cursor))
	}

	async fn get_all(&self, path: &Path) -> SourceResult<Vec<u8>> {
		let file_id = self.get_file_id_by_path(path).await?;
		let mut content_body = self.download_file(&file_id).await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error downloading file from Google Drive: {:?}", err).into(),
			)
		})?;
		let mut buffer = Vec::new();
		while let Some(chunk) = content_body.next().await {
			let chunk = chunk.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error reading chunk from Google Drive: {:?}", err).into(),
				)
			})?;
			buffer.extend_from_slice(&chunk);
		}
		Ok(buffer)
	}

	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64> {
		let file_id = self.get_file_id_by_path(path).await?;
		let (_, metadata) = self
			.hub
			.files()
			.get(&file_id)
			.param("fields", "size")
			.doit()
			.await
			.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error getting file metadata from Google Drive: {:?}", err)
						.into(),
				)
			})?;
		Ok(metadata.size.unwrap_or(0) as u64)
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'life0>>> {
		let mut page_token = self.page_token.clone();
		let folder_id = self.folder_id.clone();
		let hub = self.hub.clone();
		let source_id = self.source_id.clone();
		let stream = stream! {
			loop {
				let (_, list) = retry(&self.retry_params, || async {
					hub
					.files()
					.list()
					.q(&format!("'{}' in parents", folder_id))
					.page_token(page_token.as_deref().unwrap_or_default())
					.add_scope("https://www.googleapis.com/auth/drive".to_string())
					.doit()
					.await
					.map_err(|err| {
						SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Error listing files in Google Drive: {:?}", err).into(),
						)
					})
				}).await?;

				if let Some(files) = list.files {
					for file in files {
						let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
						if let Some(file_id) = file.id {
							let content_body =
								download_file(&hub, &file_id,&self.retry_params).await.map_err(|err| {
									SourceError::new(
										SourceErrorKind::Io,
										anyhow::anyhow!(
											"Error downloading file from Google Drive: {:?}",
											err
										)
										.into(),
									)
								})?;
							let collected_bytes = CollectedBytes::new(
								Some(file.name.clone().map(PathBuf::from).unwrap_or_default()),
								Some(Box::pin(body_to_async_read(content_body))),
								true,
								Some(folder_id.clone()),
								Some(file.size.unwrap_or(0) as usize),
								source_id.clone(),
								None,
							);
							yield Ok(collected_bytes);
						}
					}
				}

				if list.next_page_token.is_none() {
					break;
				}
				page_token = list.next_page_token;
			}
		};

		Ok(Box::pin(stream))
	}
}

// Convert hyper::Body to AsyncRead
fn body_to_async_read(body: Body) -> impl AsyncRead + Send + Unpin {
	// Create a StreamReader that wraps the Body stream
	StreamReader::new(body.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)))
}

async fn download_file(
	hub: &DriveHub,
	file_id: &str,
	retry_params: &RetryParams,
) -> Result<Body, google_drive3::Error> {
	let (resp_obj, file) = retry(retry_params, || async {
		hub.files()
			.get(file_id)
			.supports_all_drives(true)
			.acknowledge_abuse(false)
			.param("fields", FIELDS)
			.param("alt", "media")
			.add_scope(Scope::Full)
			.doit()
			.await
			.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Connection,
					anyhow::anyhow!("Error querying files in Google Drive: {:?}", err).into(),
				)
			})
	})
	.await?;

	let mime_type = file.mime_type.unwrap_or_default();
	if mime_type == "application/vnd.google-apps.folder" {
		return Err(google_drive3::Error::FieldClash("Cannot download a folder"));
	}

	if mime_type.starts_with("application/vnd.google-apps.") {
		let _export_mime_type = match mime_type.as_str() {
			"application/vnd.google-apps.document" => "application/pdf",
			"application/vnd.google-apps.spreadsheet" =>
				"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
			"application/vnd.google-apps.presentation" =>
				"application/vnd.openxmlformats-officedocument.presentationml.presentation",
			_ => return Err(google_drive3::Error::FieldClash("Unsupported Google Apps file type")),
		};

		Ok(resp_obj.into_body())
	} else {
		Ok(resp_obj.into_body())
	}
}

#[cfg(test)]
mod tests {

	use std::{collections::HashSet, env};

	use futures::StreamExt;
	use proto::semantics::GoogleDriveCollectorConfig;

	use crate::DataSource;

	use super::GoogleDriveSource;
	use dotenv::dotenv;

	#[tokio::test]
	async fn test_drive_collector() {
		dotenv().ok();
		let google_config = GoogleDriveCollectorConfig {
			id: "Drive-source-id".to_string(),
			drive_client_secret: env::var("DRIVE_CLIENT_SECRET").unwrap_or_else(|_| "".to_string()),
			drive_client_id: env::var("DRIVE_CLIENT_ID").unwrap_or_else(|_| "".to_string()),
			drive_refresh_token: env::var("DRIVE_REFRESH_TOKEN").unwrap_or_else(|_| "".to_string()),
			folder_to_crawl: "1DL5KV4NbjBZwRK-9fgacdFP-rBJaJOjE".to_string(),
		};

		let drive_storage = GoogleDriveSource::new(google_config).await;
		let connectivity = drive_storage.check_connectivity().await;

		assert!(connectivity.is_ok(), "Expected connectivity to be ok");

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
		assert!(count_files.len() > 0, "Expected at least one file");
	}

	#[tokio::test]
	async fn test_drive_collector_with_zip_file() {
		dotenv().ok();
		let google_config = GoogleDriveCollectorConfig {
			id: "Drive-source-id".to_string(),
			drive_client_secret: env::var("DRIVE_CLIENT_SECRET").unwrap_or_else(|_| "".to_string()),
			drive_client_id: env::var("DRIVE_CLIENT_ID").unwrap_or_else(|_| "".to_string()),
			drive_refresh_token: env::var("DRIVE_REFRESH_TOKEN").unwrap_or_else(|_| "".to_string()),
			folder_to_crawl: "1aG1P1lOn4aUbApXGr4ddcs-GuVxgymHl".to_string(),
		};

		let drive_storage = GoogleDriveSource::new(google_config).await;
		let connectivity = drive_storage.check_connectivity().await;

		assert!(connectivity.is_ok(), "Expected connectivity to be ok");

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
		assert!(count_files.len() > 0, "Expected at least one file");
	}

	#[tokio::test]
	async fn test_drive_collector_invalid_credentials() {
		dotenv().ok();
		let google_config = GoogleDriveCollectorConfig {
			id: "Drive-source-id".to_string(),
			drive_client_secret: "InvalidSecret".to_string(),
			drive_client_id: "InvalidClientID".to_string(),
			drive_refresh_token: "InvalidRefreshToken".to_string(),
			folder_to_crawl: "1BtLKXcYBrS16CX0R4V1X7Y4XyO9Ct7f8".to_string(),
		};

		let drive_storage = GoogleDriveSource::new(google_config).await;
		let connectivity = drive_storage.check_connectivity().await;

		assert!(connectivity.is_err(), "Expected connectivity to fail with invalid credentials");
	}

	#[tokio::test]
	async fn test_drive_collector_invalid_folder() {
		dotenv().ok();
		let google_config = GoogleDriveCollectorConfig {
			id: "Drive-source-id".to_string(),
			drive_client_secret: env::var("DRIVE_CLIENT_SECRET").unwrap_or_else(|_| "".to_string()),
			drive_client_id: env::var("DRIVE_CLIENT_ID").unwrap_or_else(|_| "".to_string()),
			drive_refresh_token: env::var("DRIVE_REFRESH_TOKEN").unwrap_or_else(|_| "".to_string()),
			folder_to_crawl: "invalid-folder-id".to_string(),
		};

		let drive_storage = GoogleDriveSource::new(google_config).await;
		let connectivity = drive_storage.check_connectivity().await;
		assert!(connectivity.is_ok(), "Expected connectivity to pass");

		let result = drive_storage.poll_data().await;

		let mut stream = result.unwrap();
		let mut count_files: HashSet<String> = HashSet::new();
		let mut found_error = false;
		while let Some(item) = stream.next().await {
			match item {
				Ok(collected_bytes) =>
					if let Some(pathbuf) = collected_bytes.file {
						if let Some(str_path) = pathbuf.to_str() {
							count_files.insert(str_path.to_string());
						}
					},
				Err(err) => {
					found_error = true;
					eprintln!("Expected successful data collection {:?}", err)
				},
			}
		}

		assert!(found_error, "Expected data collection to fail with invalid folder ID");
	}
}
