use std::{io::Cursor, ops::Range, path::Path, pin::Pin, sync::Arc};

use anyhow::{anyhow, Result};
use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{Stream, TryStreamExt as _};
use onedrive_api::{
	Auth, ClientCredential, DriveLocation, ItemLocation, OneDrive, Permission, Tenant,
};
use proto::semantics::OneDriveConfig;
use reqwest::get;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio_util::io::StreamReader;
use tracing::instrument;

use crate::{
	default_copy_to_file, SendableAsync, Source, SourceError, SourceErrorKind, SourceResult,
	REQUEST_SEMAPHORE,
};

#[derive(Clone)]
pub struct OneDriveSource {
	onedrive: OneDrive,
	folder_path: String,
	source_id: String,
}

pub static TOKEN: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();

impl OneDriveSource {
	pub async fn new(config: OneDriveConfig) -> anyhow::Result<Self> {
		let onedrive = Self::get_logined_onedrive(&config).await?;
		Ok(OneDriveSource {
			onedrive,
			folder_path: config.folder_path,
			source_id: config.id.clone(),
		})
	}

	pub async fn get_logined_onedrive(config: &OneDriveConfig) -> Result<OneDrive, anyhow::Error> {
		let token = TOKEN
			.get_or_init(|| async {
				let auth = Auth::new(
					config.client_id.clone(),
					Permission::new_read().write(true).offline_access(true),
					config.redirect_uri.clone(),
					Tenant::Consumers,
				);
				auth.login_with_refresh_token(
					&config.refresh_token,
					&ClientCredential::Secret(config.client_secret.clone()),
				)
				.await
				.map_err(|e| anyhow!("Login failed: {}", e))
				.unwrap()
				.access_token
			})
			.await;
		Ok(OneDrive::new(token.clone(), DriveLocation::me()))
	}

	async fn download_file(
		url: &str,
	) -> Result<impl AsyncRead + Send + Unpin, Box<dyn std::error::Error>> {
		let response = get(url).await?;
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
impl Source for OneDriveSource {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		self.onedrive.get_drive().await?;
		Ok(())
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
						Self::download_file(download_url).await.map_err(|e| SourceError {
							kind: SourceErrorKind::Io,
							source: Arc::new(anyhow!("Got error while downloading file: {}", e)),
						})?;
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
						Self::download_file(download_url).await.map_err(|e| SourceError {
							kind: SourceErrorKind::Io,
							source: Arc::new(anyhow!("Got error while downloading file: {}", e)),
						})?;
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
						Self::download_file(download_url).await.map_err(|e| SourceError {
							kind: SourceErrorKind::Io,
							source: Arc::new(anyhow!("Got error while downloading file: {}", e)),
						})?;
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
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
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
						let bytes_stream = Self::download_file(download_url).await.map_err(|e| SourceError {
							kind: SourceErrorKind::Io,
							source: Arc::new(anyhow!("Got error while downloading file: {}", e)),
						})?;
						yield Ok(CollectedBytes {
							data: Some(Box::pin(bytes_stream)),
							file: Some(Path::new(&name).to_path_buf()),
							eof: true,
							doc_source: Some("onedrive://".to_string()),
							extension: extension.clone(),
							size: Some(size as usize),
							source_id: source_id.clone(),
							_owned_permit: None,
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
						Self::download_file(download_url).await.map_err(|e| SourceError {
							kind: SourceErrorKind::Io,
							source: Arc::new(anyhow!("Got error while downloading file: {}", e)),
						})?;
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
						Self::download_file(download_url).await.map_err(|e| SourceError {
							kind: SourceErrorKind::Io,
							source: Arc::new(anyhow!("Got error while downloading file: {}", e)),
						})?;
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
	use crate::{onedrive::onedrive::OneDriveSource, Source};
	use dotenv::dotenv;
	use futures::StreamExt;
	use proto::semantics::OneDriveConfig;
	use std::{collections::HashSet, env};

	#[tokio::test]
	async fn test_onedrive_collector() {
		dotenv().ok();
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

		println!("Connectivity: {:?}", connectivity);

		let result = drive_storage.poll_data().await;

		let mut stream = result.unwrap();
		let mut count_files: HashSet<String> = HashSet::new();
		while let Some(item) = stream.next().await {
			match item {
				Ok(collected_bytes) => {
					if let Some(pathbuf) = collected_bytes.file {
						if let Some(str_path) = pathbuf.to_str() {
							count_files.insert(str_path.to_string());
						}
					}
				},
				Err(err) => eprintln!("Expected successful data collection {:?}", err),
			}
		}
		println!("Files are --- {:?}", count_files);
	}
}
