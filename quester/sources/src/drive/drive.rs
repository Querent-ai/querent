use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{Stream, StreamExt};
use google_drive3::{
	api::Scope,
	hyper::{self, client::HttpConnector},
	hyper_rustls::{HttpsConnector, HttpsConnectorBuilder},
	oauth2::{ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod, ServiceAccountAuthenticator, ServiceAccountKey},
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
			redirect_uris: vec!["http://localhost".to_string()],
			project_id: None,
			auth_provider_x509_cert_url: None,
			client_email: None,
			client_x509_cert_url: None,
		};

		// let auth =
		// 	InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
		// 		.build()
		// 		.await
		// 		.expect("authenticator could not be built");

		let service_account_key = ServiceAccountKey {
			project_id: Some("querent-403619".to_string()),
			private_key_id: Some("ab6f05f2c4f462354ce1a81a67e382842829a3a1".to_string()),
			private_key: "-----BEGIN PRIVATE KEY-----\nMIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCLeR4TMbYZwUlU\nhKRJi9WDCd8zNXZJ8cPMKYxfxh2LuobL4lvagG+9HwUpso2qJJwGmWJDtCSncrWO\nPTMZ14zv72EtLNpCNkYtSlsGQwMu/uUA+p5UZiGoM2Ic827Y9EijQLxXVhKcQq5/\nFfVDLCxZtCFwSCL2xLXqLmS8PbCQW0iYlEHHrg+f+VWseEe+9Ib+4TzARQAIoY7E\n4lAMeGH/TNY/c4IUEb5i1NMzoaXCmIMujWPDpyXOWIweKQKfVnNe9CugeSBKX2+d\nAphPh2Y6MiUpuo6enznX0Rv/vU/IRKlqUbZQA1jYA70Y+d5BqFMxjnGfGsHlR4x/\nIVPOl7UdAgMBAAECggEAAlASGFni7ebnXyQq8EBGHFvpzFjW2w+MAmLu/biUjVhc\nu11HziYClFWDGaepEzjzsGVTPJGsaO1tRsxLgGJzZxgmWaAsh1wMilw5Ca/LSfh2\nli9RuE8QQFCH0DiWLjlQygo9BUq7WMV/TMKxtDkzjBJBWrILiGGHLbiyuW9hcedX\nVfGQL1oCMZT1diH40SpPWbjRiODIiE/pp08hE9m3+ov0b8Y8lzNchLDQjoc7CqzA\nlweak7jQ1wUG4svAWukPtkeKSECaxXjgqYD9TWxhmINITAqvkR48GKvDR2jTj7wS\nQxMh0wha0GU+SU5Z0xjtqwbC8lUOG7kRX3UhLKH7sQKBgQDDMjHoH8073edAlK24\nh7b+GXMBqth5ChsF6qvbdsd73neGDB8NrX5D+L/sJuUadxW0M5wjJyMJGlYEukG+\n6RSczFMYDXmfMIMnx1duhttVD5dvzx090k0ANTwawe9NKKnY6jCzgU1A812G4kdn\nV2WJUzBRC1chLAHUp2XqyCg0MQKBgQC261NNfEDPJK1tWWlmm4CMmU2pk35ziaOx\nnJrVK8s0ZBpHXym7QvVNwzkuHAoWW/15ssFKMVv+HlcARtZx50Y55puGskvDRtF4\nzRf7GovocE0idj0P1BUp0Mkxuih5VW3wrfiiqIHzrM6Re76BpT1d7oU8w5Vadb3V\nZmzCa0dwrQKBgH8GtlWiBHR2Nxze5KKWpy57L02hedhjDCzwh8B9btocb1nrn3XO\nNsJTKcqrkSKE5rnrcCusN2+gFORktY5grkpP6a9YbZJ8Bo4neq1x02BqkhlwBk6K\nAhQlkKS1Gl7zHH0OAn1+ouCmv3Gc5ezJgkk4utOy9pOeyN4zxe5hLVCxAoGAe4iK\nBbZ4fmyiw0qzKBy0wD94d6GosJav+m9tEbI11fgU10aphFJAIHhL0ZwWI+uUT/At\nIdIb8o7C6ujsQpiSkN/xARLAn+zf4tl/7JGNEzlknnWD34C3mjnq5q52Txsm2Hhl\nhlSPDuYRy6bqjdvuidVgHh1obGNABTLbGKIi6TECgYAOffuMlgROcDxJpb163LLk\nuXYGd8JQ1nnqtyt+72NE/5ZOg6j/rwyVOOsUOrExS6Pm6vactz5XqgX+902/blvN\nMrXQNJ40KBZZNZV1AiPFXvEG09wEx39hxCV+AkWRTprvDg8EXoC/YtYLMaG9ioVq\nlAZN4OfrWi8fq13v8foUxw==\n-----END PRIVATE KEY-----\n".to_string(),
			client_email: "querent-403619@querent-403619.iam.gserviceaccount.com".to_string(),
			client_id: Some("111363052744608238256".to_string()),
			token_uri: "https://oauth2.googleapis.com/token".to_string(),
			key_type: Some("service_account".to_string()),
			auth_uri: Some("https://accounts.google.com/o/oauth2/auth".to_string()),
			auth_provider_x509_cert_url: Some("https://www.googleapis.com/oauth2/v1/certs".to_string()),
			client_x509_cert_url: Some("https://www.googleapis.com/robot/v1/metadata/x509/querent-403619%40querent-403619.iam.gserviceaccount.com".to_string()),
		};
		let auth2 = ServiceAccountAuthenticator::builder(service_account_key).build().await.expect("authenticator could not be built");
		let _ = auth2.force_refreshed_token(&[config.drive_scopes.clone()]).await.map_err(|e| format!("Failed to refresh token: {}", e));

		let token = auth2.force_refreshed_token(&[config.drive_scopes]).await
            .map_err(|e| format!("Failed to get token: {}", e));
		println!("Access Token: {:?}", token);

		let connector = HttpsConnectorBuilder::new()
			.with_native_roots()
			.https_or_http()
			.enable_http1()
			.enable_http2()
			.build();

		let http_client = hyper::Client::builder().build(connector);
		let hub = DriveHub::new(http_client, auth2);

		GoogleDriveSource { hub, folder_id: config.folder_to_crawl, page_token: None }
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
			.add_scope(Scope::Full)
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
					.add_scope("https://www.googleapis.com/auth/drive".to_string())
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
	let (resp_obj, file) = hub
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

    use std::collections::HashSet;

    use futures::StreamExt;
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

		let result = drive_storage.hub.files().list()
            .add_scope("https://www.googleapis.com/auth/drive.readonly")
            .q(&format!("'{}' in parents", drive_storage.folder_id))
            .doit()
            .await;
        let _ = match result {
            Ok(res) => {
				println!("Response from files list {:?}", res);
			},
            Err(e) => eprintln!("Expected successful data collection {:?}", e),
        };


		let result = drive_storage.poll_data().await;

		let mut stream = result.unwrap();
		let mut count_files: HashSet<String> = HashSet::new();
		while let Some(item) = stream.next().await {
			match item {
				Ok(collected_bytes) => {
					println!("Collected bytes: {:?}", collected_bytes);
					if let Some(pathbuf) = collected_bytes.file {
						if let Some(str_path) = pathbuf.to_str() {
							count_files.insert(str_path.to_string());
						}
					}
				}
				Err(err) => eprintln!("Expected successful data collection {:?}", err),
			}
		}
		println!("Files are --- {:?}", count_files);

	}

}
