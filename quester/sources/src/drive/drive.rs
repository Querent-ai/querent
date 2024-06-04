use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{Stream, StreamExt};
use google_drive3::{
	api::Scope,
	hyper,
	hyper::client::HttpConnector,
	hyper_rustls::{HttpsConnector, HttpsConnectorBuilder},
	oauth2::{ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod},
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
use tracing::instrument;

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};

type DriveHub = google_drive3::DriveHub<HttpsConnector<HttpConnector>>;

pub const FIELDS: &str =
	"mimeType,id,kind,teamDriveId,name,driveId,description,size,md5Checksum,parents,trashed";

#[derive(Clone)]
pub struct GoogleDriveSource {
	hub: DriveHub,
	folder_id: String,
	page_token: Option<String>,
}

impl std::fmt::Debug for GoogleDriveSource {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("GoogleDriveSource").field("folder_id", &self.folder_id).finish()
	}
}

impl GoogleDriveSource {
	pub async fn new(config: GoogleDriveCollectorConfig) -> Self {
		let secret = ApplicationSecret {
			client_id: config.drive_client_id.clone(),
			client_secret: config.drive_client_secret.clone(),
			auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
			token_uri: "https://oauth2.googleapis.com/token".to_string(),
			redirect_uris: vec![],
			project_id: None,
			auth_provider_x509_cert_url: None,
			client_email: None,
			client_x509_cert_url: None,
		};

		let auth =
			InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
				.persist_tokens_to_disk("tokencache.json")
				.build()
				.await
				.expect("authenticator could not be built");
		let connector = HttpsConnectorBuilder::new()
			.with_native_roots()
			.https_or_http()
			.enable_http1()
			.enable_http2()
			.build();

		let http_client = hyper::Client::builder().build(connector);
		let hub = DriveHub::new(http_client, auth);

		GoogleDriveSource { hub, folder_id: config.folder_to_crawl, page_token: None }
	}

	async fn download_file(&self, file_id: &str) -> Result<Body, google_drive3::Error> {
		let (_, file) = self
			.hub
			.files()
			.get(file_id)
			.supports_all_drives(true)
			.acknowledge_abuse(false)
			.param("fields", FIELDS)
			.add_scope(Scope::Full)
			.doit()
			.await?;

		let mime_type = file.mime_type.unwrap_or_default();
		if mime_type == "application/vnd.google-apps.folder" {
			return Err(google_drive3::Error::FieldClash("Cannot download a folder"));
		}

		if mime_type.starts_with("application/vnd.google-apps.") {
			let export_mime_type = match mime_type.as_str() {
				"application/vnd.google-apps.document" => "application/pdf",
				"application/vnd.google-apps.spreadsheet" =>
					"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
				"application/vnd.google-apps.presentation" =>
					"application/vnd.openxmlformats-officedocument.presentationml.presentation",
				_ =>
					return Err(google_drive3::Error::FieldClash(
						"Unsupported Google Apps file type",
					)),
			};

			let resp_obj = self.hub.files().export(file_id, export_mime_type).doit().await?;
			Ok(resp_obj.into_body())
		} else {
			let resp_obj = self.hub.files().export(file_id, &mime_type).doit().await?;
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
impl Source for GoogleDriveSource {
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
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		let mut page_token = self.page_token.clone();
		let folder_id = self.folder_id.clone();
		let hub = self.hub.clone();
		let stream = stream! {
			loop {
				let (_, list) =hub
					.files()
					.list()
					.q(&format!("'{}' in parents", folder_id))
					.page_token(page_token.as_deref().unwrap_or_default())
					.doit()
					.await
					.map_err(|err| {
						SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Error listing files in Google Drive: {:?}", err).into(),
						)
					})?;

				if let Some(files) = list.files {
					for file in files {
						if let Some(file_id) = file.id {
							let mut content_body =
								download_file(&hub, &file_id).await.map_err(|err| {
									SourceError::new(
										SourceErrorKind::Io,
										anyhow::anyhow!(
											"Error downloading file from Google Drive: {:?}",
											err
										)
										.into(),
									)
								})?;

							while let Some(chunk) = content_body.next().await {
								let chunk = chunk.map_err(|err| {
									SourceError::new(
										SourceErrorKind::Io,
										anyhow::anyhow!(
											"Error reading chunk from Google Drive: {:?}",
											err
										)
										.into(),
									)
								})?;

								let eof = chunk.is_empty();
								if eof {
									break;
								}

								let collected_bytes = CollectedBytes::new(
									Some(file.name.clone().map(PathBuf::from).unwrap_or_default()),
									Some(chunk.to_vec()),
									eof,
									Some(folder_id.clone()),
									Some(chunk.len()),
								);

								yield Ok(collected_bytes);
							}

							// Send EOF for the file
							let eof_collected_bytes = CollectedBytes::new(
								Some(file.name.clone().map(PathBuf::from).unwrap_or_default()),
								None,
								true,
								Some(folder_id.clone()),
								None,
							);

							yield Ok(eof_collected_bytes);
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

async fn download_file(hub: &DriveHub, file_id: &str) -> Result<Body, google_drive3::Error> {
	let (_, file) = hub
		.files()
		.get(file_id)
		.supports_all_drives(true)
		.acknowledge_abuse(false)
		.param("fields", FIELDS)
		.add_scope(Scope::Full)
		.doit()
		.await?;

	let mime_type = file.mime_type.unwrap_or_default();
	if mime_type == "application/vnd.google-apps.folder" {
		return Err(google_drive3::Error::FieldClash("Cannot download a folder"));
	}

	if mime_type.starts_with("application/vnd.google-apps.") {
		let export_mime_type = match mime_type.as_str() {
			"application/vnd.google-apps.document" => "application/pdf",
			"application/vnd.google-apps.spreadsheet" =>
				"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
			"application/vnd.google-apps.presentation" =>
				"application/vnd.openxmlformats-officedocument.presentationml.presentation",
			_ => return Err(google_drive3::Error::FieldClash("Unsupported Google Apps file type")),
		};

		let resp_obj = hub.files().export(file_id, export_mime_type).doit().await?;
		Ok(resp_obj.into_body())
	} else {
		let resp_obj = hub.files().export(file_id, &mime_type).doit().await?;
		Ok(resp_obj.into_body())
	}
}
