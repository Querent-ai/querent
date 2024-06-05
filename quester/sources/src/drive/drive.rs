use async_trait::async_trait;
use common::CollectedBytes;
use futures::StreamExt;
use google_drive3::{
	api::Scope,
	hyper,
	hyper::client::HttpConnector,
	hyper_rustls::{HttpsConnector, HttpsConnectorBuilder},
};
use hyper::Body;
use proto::semantics::GoogleDriveCollectorConfig;
use yup_oauth2::{authorized_user::AuthorizedUserSecret, AuthorizedUserAuthenticator};
use std::{
	io::Cursor,
	ops::Range,
	path::{Path, PathBuf},
};
use tokio::{
	io::{AsyncRead, AsyncWriteExt},
	sync::mpsc,
};
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

impl GoogleDriveSource {
	pub async fn new(config: GoogleDriveCollectorConfig) -> Self {
		tracing::info!("Entered atleast once");

		let auth = AuthorizedUserAuthenticator::builder(AuthorizedUserSecret {
			client_id: config.drive_client_id.clone(),
			client_secret: config.drive_client_secret.clone(),
			refresh_token: config.drive_refresh_token.clone(),
			key_type: "service_account".to_string(),
		}).build().await.unwrap();

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
			.param("alt", "media")
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

	// async fn download_from_url(&self, url: &str) -> Result<Vec<u8>, hyper::Error> {
	// 	let response = reqwest::get(url)
	// 		.await;
	// 	match response {
	// 		Ok(res) => {
	// 			let bytes_res = res
	// 				.bytes()
	// 				.await;
	// 			match bytes_res {
	// 				Ok(bytes) => {
	// 					Ok(bytes.to_vec())
	// 				},
	// 				Err(e) => {
	// 					eprintln!("Got error: {:?}", e);
	// 					let temp_res: Vec<u8> = [].to_vec();
	// 					Ok(temp_res)
	// 				},
	// 			}
	// 		},
	// 		Err(e) => {
	// 			eprintln!("Got an error: {:?}", e);
	// 			let temp_res: Vec<u8> = [].to_vec();
	// 			Ok(temp_res)
	// 		}
	// 	}
		
	// }

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
	async fn list_files(&self, _path: &Path) -> SourceResult<Vec<String>> {
		let mut files_list = Vec::new();
		let mut page_token: Option<String> = None;

		loop {
			let mut results = self.hub
			.files()
			.list()
			.q(&format!("'{}' in parents and trashed = false", self.folder_id))
			.param("fields", "nextPageToken, files(id, name)")
			.add_scope(Scope::Full);

			if let Some(ref token) = page_token {
				results = results.page_token(&token);
			}

			let results_final = results.doit().await;

			match results_final {
				Ok((_, file_list)) => {
					if let Some(files) = file_list.files {
						for file in files {
							if let Some(name) = file.name  {
								files_list.push(name)
							}
						}
					}
					page_token = file_list.next_page_token;
                    if page_token.is_none() {
                        break;
                    }
				},
				Err(e) => {
					eprintln!("Got error: {:?}", e);
				}
			}
		}

		Ok(files_list)

	}
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

	async fn poll_data(&self, output: mpsc::Sender<CollectedBytes>) -> SourceResult<()> {
		let mut page_token = self.page_token.clone();
		loop {
			let (_, list) = self
				.hub
				.files()
				.list()
				.q(&format!("'{}' in parents", self.folder_id))
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
							self.download_file(&file_id).await.map_err(|err| {
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
								Some(self.folder_id.clone()),
								Some(chunk.len()),
							);

							output.send(CollectedBytes::from(collected_bytes)).await.map_err(
								|e| {
									SourceError::new(
										SourceErrorKind::Io,
										anyhow::anyhow!("Error sending collected bytes: {:?}", e)
											.into(),
									)
								},
							)?;
						}

						// Send EOF for the file
						let eof_collected_bytes = CollectedBytes::new(
							Some(file.name.clone().map(PathBuf::from).unwrap_or_default()),
							None,
							true,
							Some(self.folder_id.clone()),
							None,
						);

						output.send(CollectedBytes::from(eof_collected_bytes)).await.map_err(
							|e| {
								SourceError::new(
									SourceErrorKind::Io,
									anyhow::anyhow!("Error sending collected bytes: {:?}", e)
										.into(),
								)
							},
						)?;
					}
				}
			}

			if list.next_page_token.is_none() {
				break;
			}
			page_token = list.next_page_token;
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use proto::semantics::GoogleDriveCollectorConfig;

    use crate::Source;

    use super::GoogleDriveSource;


	#[tokio::test]
	async fn test_drive_collector() {
		let google_config = GoogleDriveCollectorConfig {
			drive_client_secret: "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".to_string(),
			drive_client_id: "4402204563-lso0f98dve9k33durfvqdt6dppl7iqn5.apps.googleusercontent.com".to_string(),
			drive_refresh_token: "1//0g7Sd9WayGH-yCgYIARAAGBASNwF-L9Irh8XWYJ_zz43V0Ema-OqTCaHzdJKrNtgJDrrrRSs8z6iJU9dgR8tA1fucRKjwUVggwy8".to_string(),
			drive_scopes: "https://www.googleapis.com/auth/drive".to_string(),
			drive_token: "ya29.a0AfB_byAMnws17-UAYR2hU29zC83Rw4bxn2LsF5i_sWQ5xDMI00li205pXlA-JrwVmBh0kNBK7sKP33urPZ9-DM9DDKMv6EQsaqJsy57aHQYUwddT42SwuZAVINyTwp340Qiy_hSaVG5ezT9PIYRO5Qd1Yn9wm5rd7Aq-".to_string(),
			folder_to_crawl: "1BtLKXcYBrS16CX0R4V1X7Y4XyO9Ct7f8".to_string(),
			specific_file_type: "application/pdf".to_string()
		};

		let drive_storage = GoogleDriveSource::new(google_config).await;
		let connectivity = drive_storage.check_connectivity().await;

		println!("Connectivity: {:?}", connectivity);

		let files_list = drive_storage.list_files(Path::new("")).await;
		match files_list {
			Ok(files) => {
				for file in files {
					println!("Files in folder are - {:?}", file);
				}

				// let temp_path = Path::new("");
				// let file_content = drive_storage.get_all(Path::new(&temp_path)).await;
				// println!("Final file content: {:?}", file_content.unwrap());
			},
			//17eOjb3PXngJWA2DlEYCvLS7ez-5S-kEA
			Err(e) => {
				println!("Error : {:?}", e);
			}
		}
	}

}