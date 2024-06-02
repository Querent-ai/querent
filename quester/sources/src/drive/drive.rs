// use async_trait::async_trait;
// use google_drive3::{
// 	api::Scope,
// 	hyper, hyper_rustls,
// 	oauth2::{ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod},
// };
// use hyper::body;
// use proto::semantics::GoogleDriveCollectorConfig;
// use std::{ops::Range, path::Path};
// use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
// use tracing::instrument;

// use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};

// type HyperConnector = hyper_rustls::HttpsConnector<hyper::client::HttpConnector>;

// type DriveHub = google_drive3::DriveHub<HyperConnector>;

// pub const FIELDS: &str =
// 	"mimeType,id,kind,teamDriveId,name,driveId,description,size,md5Checksum,parents,trashed";

// fn hyper_client() -> hyper::Client<HyperConnector> {
// 	hyper::Client::builder().build(
// 		hyper_rustls::HttpsConnectorBuilder::new()
// 			.with_native_roots()
// 			.https_or_http()
// 			.enable_http1()
// 			.enable_http2()
// 			.build(),
// 	)
// }

// #[derive(Clone)]
// pub struct GoogleDriveSource {
// 	hub: DriveHub,
// 	folder_id: String,
// }

// impl GoogleDriveSource {
// 	pub async fn new(config: GoogleDriveCollectorConfig) -> Self {
// 		let secret = ApplicationSecret {
// 			client_id: config.drive_client_id.clone(),
// 			client_secret: config.drive_client_secret.clone(),
// 			auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
// 			token_uri: "https://oauth2.googleapis.com/token".to_string(),
// 			redirect_uris: vec![],
// 			project_id: None,
// 			auth_provider_x509_cert_url: None,
// 			client_email: None,
// 			client_x509_cert_url: None,
// 		};

// 		let auth =
// 			InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
// 				.persist_tokens_to_disk("tokencache.json")
// 				.build()
// 				.await
// 				.expect("authenticator could not be built");

// 		let hub = DriveHub::new(hyper_client(), auth);

// 		GoogleDriveSource { hub, folder_id: config.folder_to_crawl }
// 	}

// 	async fn download_file(
// 		&self,
// 		file_id: &str,
// 	) -> Result<impl AsyncRead + Unpin, google_drive3::Error> {
// 		let (_, file) = self
// 			.hub
// 			.files()
// 			.get(file_id)
// 			.supports_all_drives(true)
// 			.acknowledge_abuse(false)
// 			.param("fields", FIELDS)
// 			.add_scope(Scope::Full)
// 			.doit()
// 			.await?;

// 		let mime_type = file.mime_type.unwrap_or_default();
// 		if mime_type == "application/vnd.google-apps.folder" {
// 			return Err::<_, google_drive3::Error>(google_drive3::Error::FieldClash(
// 				"Cannot download a folder",
// 			));
// 		}
// 		let mut download_file = false;
// 		if mime_type.starts_with("application/vnd.google-apps.") {
// 			if mime_type == "application/vnd.google-apps.document" {
// 				download_file = true;
// 			}
// 		} else {
// 			download_file = true;
// 		}

// 		if download_file {
// 			let resp_obj = self.hub.files().export(&file_id, &mime_type).doit().await?;
// 			let data: body::Bytes = body::to_bytes(resp_obj.into_body()).await.map_err(|err| {
// 				google_drive3::Error::FieldClash(
// 					"Error converting body to bytes: ",
// 				)
// 			})?;
// 			let data_bytes_vec = data.to_vec();

// 			// Based on return
// 			Ok(data_bytes_vec.as_slice())
// 		} else {
// 			Err(google_drive3::Error::FieldClash("Cannot download this file type"))
// 		}
// 	}

// 	async fn get_file_id_by_path(&self, path: &Path) -> Result<String, SourceError> {
// 		let mut query: String = format!("'{}' in parents", self.folder_id);
// 		for component in path.iter() {
// 			query.push_str(&format!(" and name = '{}'", component.to_string_lossy()));
// 		}
// 		let (_, files) = self.hub.files().list().q(&query).doit().await.map_err(|err| {
// 			SourceError::new(
// 				SourceErrorKind::Io,
// 				anyhow::anyhow!("Error querying files in Google Drive: {:?}", err).into(),
// 			)
// 		})?;
// 		if let Some(files) = files.files.into_iter().next() {
// 			if files.len() > 1 {
// 				log::warn!("Multiple files found in Google Drive for path {:?}", path);
// 			}
// 			Ok(files[0].id.clone().unwrap())
// 		} else {
// 			Err(SourceError::new(
// 				SourceErrorKind::NotFound,
// 				anyhow::anyhow!("File not found in Google Drive").into(),
// 			))
// 		}
// 	}
// }

// #[async_trait]
// impl Source for GoogleDriveSource {
// 	async fn copy_to(&self, path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
// 		let file_id = self.get_file_id_by_path(path).await?;
// 		let mut reader = BufReader::new(self.download_file(&file_id).await.map_err(|err| {
// 			SourceError::new(
// 				SourceErrorKind::Io,
// 				anyhow::anyhow!("Error downloading file from Google Drive: {:?}", err).into(),
// 			)
// 		})?);

// 		tokio::io::copy_buf(&mut reader, output).await.map_err(|err| {
// 			SourceError::new(
// 				SourceErrorKind::Io,
// 				anyhow::anyhow!("Error copying data to output: {:?}", err).into(),
// 			)
// 		})?;
// 		output.flush().await.map_err(|err| {
// 			SourceError::new(
// 				SourceErrorKind::Io,
// 				anyhow::anyhow!("Error flushing output: {:?}", err).into(),
// 			)
// 		})?;
// 		Ok(())
// 	}

// 	async fn check_connectivity(&self) -> anyhow::Result<()> {
// 		self.hub.files().list().page_size(1).doit().await?;
// 		Ok(())
// 	}

// 	#[instrument(level = "debug", skip(self, range), fields(range.start = range.start, range.end = range.end))]
// 	async fn get_slice(&self, path: &Path, range: Range<usize>) -> SourceResult<Vec<u8>> {
// 		let file_id = self.get_file_id_by_path(path).await?;
// 		let mut reader = BufReader::new(self.download_file(&file_id).await.map_err(|err| {
// 			SourceError::new(
// 				SourceErrorKind::Io,
// 				anyhow::anyhow!("Error downloading file from Google Drive: {:?}", err).into(),
// 			)
// 		})?);
// 		let mut buffer = vec![0; range.len()];
// 		reader.read_exact(&mut buffer).await.map_err(|err| {
// 			SourceError::new(
// 				SourceErrorKind::Io,
// 				anyhow::anyhow!("Error reading slice from Google Drive: {:?}", err).into(),
// 			)
// 		})?;
// 		Ok(buffer)
// 	}

// 	#[instrument(level = "debug", skip(self, range), fields(range.start = range.start, range.end = range.end))]
// 	async fn get_slice_stream(
// 		&self,
// 		path: &Path,
// 		range: Range<usize>,
// 	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
// 		let file_id = self.get_file_id_by_path(path).await?;
// 		let reader = self.download_file(&file_id).await.map_err(|err| {
// 			SourceError::new(
// 				SourceErrorKind::Io,
// 				anyhow::anyhow!("Error downloading file from Google Drive: {:?}", err).into(),
// 			)
// 		})?;

// 		let mut reader = BufReader::new(reader);
// 		let mut buffer = vec![0; range.len()];
// 		reader.read_exact(&mut buffer).await.map_err(|err| {
// 			SourceError::new(
// 				SourceErrorKind::Io,
// 				anyhow::anyhow!("Error reading slice from Google Drive: {:?}", err).into(),
// 			)
// 		})?;
// 		Ok(Box::new(reader))
// 	}

// 	async fn get_all(&self, path: &Path) -> SourceResult<Vec<u8>> {
// 		let file_id = self.get_file_id_by_path(path).await?;
// 		let mut reader = BufReader::new(self.download_file(&file_id).await.map_err(|err| {
// 			SourceError::new(
// 				SourceErrorKind::Io,
// 				anyhow::anyhow!("Error downloading file from Google Drive: {:?}", err).into(),
// 			)
// 		})?);
// 		let mut buffer = Vec::new();
// 		reader.read_to_end(&mut buffer).await.map_err(|err| {
// 			SourceError::new(
// 				SourceErrorKind::Io,
// 				anyhow::anyhow!("Error reading file from Google Drive: {:?}", err).into(),
// 			)
// 		})?;
// 		Ok(buffer)
// 	}

// 	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64> {
// 		let file_id = self.get_file_id_by_path(path).await?;
// 		let (_, metadata) = self
// 			.hub
// 			.files()
// 			.get(&file_id)
// 			.param("fields", "size")
// 			.doit()
// 			.await
// 			.map_err(|err| {
// 				SourceError::new(
// 					SourceErrorKind::Io,
// 					anyhow::anyhow!("Error getting file metadata from Google Drive: {:?}", err)
// 						.into(),
// 				)
// 			})?;
// 		Ok(metadata.size.unwrap_or(0) as u64)
// 	}

// 	async fn poll_data(&mut self, output: &mut dyn SendableAsync) -> SourceResult<()> {
// 		let mut page_token = None;
// 		loop {
// 			let (_, list) = self
// 				.hub
// 				.files()
// 				.list()
// 				.q(&format!("'{}' in parents", self.folder_id))
// 				.page_token(page_token.as_deref().unwrap_or_default())
// 				.doit()
// 				.await
// 				.map_err(|err| {
// 					SourceError::new(
// 						SourceErrorKind::Io,
// 						anyhow::anyhow!("Error listing files in Google Drive: {:?}", err).into(),
// 					)
// 				})?;
// 			if let Some(files) = list.files {
// 				for file in files {
// 					if let Some(file_id) = file.id {
// 						let mut reader =
// 							BufReader::new(self.download_file(&file_id).await.map_err(|err| {
// 								SourceError::new(
// 									SourceErrorKind::Io,
// 									anyhow::anyhow!(
// 										"Error downloading file from Google Drive: {:?}",
// 										err
// 									)
// 									.into(),
// 								)
// 							})?);
// 						tokio::io::copy_buf(&mut reader, output).await.map_err(|err| {
// 							SourceError::new(
// 								SourceErrorKind::Io,
// 								anyhow::anyhow!("Error copying data to output: {:?}", err).into(),
// 							)
// 						})?;
// 					}
// 				}
// 			}
// 			output.flush().await.map_err(|err| {
// 				SourceError::new(
// 					SourceErrorKind::Io,
// 					anyhow::anyhow!("Error flushing output: {:?}", err).into(),
// 				)
// 			})?;
// 			if list.next_page_token.is_none() {
// 				break;
// 			}
// 			page_token = list.next_page_token;
// 		}
// 		Ok(())
// 	}
// }
