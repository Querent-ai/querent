use std::{collections::HashMap, io::Cursor, ops::Range, path::Path, pin::Pin, sync::Arc};

use anyhow::{anyhow, Result};
use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use onedrive_api::{
	Auth, ClientCredential, DriveLocation, ItemLocation, OneDrive, Permission, Tenant,
	TokenResponse,
};
use proto::semantics::OneDriveConfig;
use reqwest::{get, Client};
use tokio::io::{AsyncRead, AsyncWriteExt};
use tracing::instrument;

use crate::{
	default_copy_to_file, SendableAsync, Source, SourceError, SourceErrorKind, SourceResult,
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
		let onedrive = match Self::get_logined_onedrive(&config).await {
            Ok(logged_in_drive) => logged_in_drive,
            Err(e) => return Err(anyhow::anyhow!("Failed to log in to OneDrive: {}", e)),
        };

		Ok(OneDriveSource { onedrive, folder_path: config.folder_path, source_id: config.id.clone() })
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
				.map_err(|e| anyhow!("Login failed: {}", e)).unwrap()
				.access_token
			})
			.await;
		Ok(OneDrive::new(token.clone(), DriveLocation::me()))
	}

	async fn download_file(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
		let response = get(url).await;
		let bytes = response.unwrap().bytes().await;
		return Ok(bytes?.to_vec());
	}

	fn get_file_extension(file_name: &str) -> Option<String> {
		Path::new(file_name)
			.extension()
			.and_then(|ext| ext.to_str().map(|s| s.to_string()))
	}

	// fn construct_new_folder_path(item: &DriveItem) -> Option<String> {
	//     if let Some(folder) = &item.folder {
	//         if let Some(name) = &item.name {
	//             if let Some(parent) = &item.parent_reference {
	//                 if let Some(parent_id) = parent.get("id") {
	//                     if let Some(parent_id_str) = parent_id.as_str() {
	//                         return Some(format!("{}/{}", parent_id_str.trim_end_matches('/'), name));
	//                     }
	//                 }
	//             }
	//         }
	//     }
	//     None
	// }
}

impl std::fmt::Debug for OneDriveSource {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("GoogleDriveSource")
			.field("folder_id", &self.folder_path)
			.finish()
	}
}

#[async_trait]
impl Source for OneDriveSource {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		self.onedrive.get_drive().await.expect("Cannot get drive");
		Ok(())
	}

	async fn copy_to_file(&self, path: &Path, output_path: &Path) -> SourceResult<u64> {
		default_copy_to_file(self, path, output_path).await
	}

	#[instrument(level = "debug", skip(self, range), fields(range.start = range.start, range.end = range.end))]
	async fn get_slice(&self, path: &Path, range: Range<usize>) -> SourceResult<Vec<u8>> {
		let drive_item_all = self
			.onedrive
			.list_children(ItemLocation::from_path(&self.folder_path).unwrap())
			.await
			.map_err(|e| SourceError { kind: SourceErrorKind::Io, source: Arc::new(e.into()) })?;
		for drive_item in drive_item_all {
			if let Some(_file) = &drive_item.file {
				if let Some(download_url) = &drive_item.download_url {
					let bytes =
						Self::download_file(download_url).await.map_err(|_e| SourceError {
							kind: SourceErrorKind::Io,
							source: Arc::new(anyhow::anyhow!("Got error while downloading file")),
						})?;
					let slice = bytes[range.clone()].to_vec();
					return Ok(slice);
				}
			}
		}
		Err(SourceError {
			kind: SourceErrorKind::Io,
			source: Arc::new(anyhow::anyhow!("No files found")),
		})
	}

	#[instrument(level = "debug", skip(self, range), fields(range.start = range.start, range.end = range.end))]
	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let drive_item_all = self
			.onedrive
			.list_children(ItemLocation::from_path(&self.folder_path).unwrap())
			.await
			.map_err(|e| SourceError { kind: SourceErrorKind::Io, source: Arc::new(e.into()) })?;
		for drive_item in drive_item_all {
			if let Some(_file) = &drive_item.file {
				if let Some(download_url) = &drive_item.download_url {
					let bytes =
						Self::download_file(download_url).await.map_err(|_e| SourceError {
							kind: SourceErrorKind::Io,
							source: Arc::new(anyhow::anyhow!("Got error while downloading file")),
						})?;
					let slice = bytes[range.clone()].to_vec();
					return Ok(Box::new(Cursor::new(slice)));
				}
			}
		}
		Err(SourceError {
			kind: SourceErrorKind::Io,
			source: Arc::new(anyhow::anyhow!("No files found")),
		})
	}

	async fn get_all(&self, _path: &Path) -> SourceResult<Vec<u8>> {
		let drive_item_all = self
			.onedrive
			.list_children(ItemLocation::from_path(&self.folder_path).unwrap())
			.await
			.expect("Cannot list children");
		for drive_item in drive_item_all {
			if let Some(_file) = &drive_item.file {
				if let Some(download_url) = &drive_item.download_url {
					let bytes = Self::download_file(download_url).await.unwrap();
					return Ok(bytes)
				}
			}
		}
		Err(SourceError {
			kind: SourceErrorKind::Io,
			source: Arc::new(anyhow::anyhow!("No files found")),
		})
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		// Get the folder item by path
		let drive_item_all = self
			.onedrive
			.list_children(ItemLocation::from_path(&self.folder_path).unwrap())
			.await
			.expect("Cannot list children");
		let source_id = self.source_id.clone();

		let stream = stream! {
			for drive_item in drive_item_all {
				if let Some(_file) = &drive_item.file {
					let name = drive_item.name.unwrap();
					let extension = Self::get_file_extension(&name);
					if let Some(download_url) = &drive_item.download_url {
						let bytes = Self::download_file(download_url).await.unwrap();
						let res = CollectedBytes {
							data: Some(bytes),
							file: Some(Path::new(&name).to_path_buf()),
							eof: false,
							doc_source: Some("onedrive://".to_string()),
							extension: extension,
							size: Some(123),
							source_id: source_id.clone(),
						};
						yield Ok(res);

					}
				}
			}
		};
		Ok(Box::pin(stream))
	}

	async fn file_num_bytes(&self, _path: &Path) -> SourceResult<u64> {
		let drive_item_all = self
			.onedrive
			.list_children(ItemLocation::from_path(&self.folder_path).unwrap())
			.await
			.map_err(|e| SourceError { kind: SourceErrorKind::Io, source: Arc::new(e.into()) })?;
		for drive_item in drive_item_all {
			if let Some(_file) = &drive_item.file {
				if let Some(download_url) = &drive_item.download_url {
					let bytes =
						Self::download_file(download_url).await.map_err(|_e| SourceError {
							kind: SourceErrorKind::Io,
							source: Arc::new(anyhow::anyhow!("Got error while downloading file")),
						})?;
					return Ok(bytes.len().try_into().unwrap());
				}
			}
		}
		Err(SourceError {
			kind: SourceErrorKind::Io,
			source: Arc::new(anyhow::anyhow!("No files found")),
		})
	}

	async fn copy_to(&self, _path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let drive_item_all = self
			.onedrive
			.list_children(ItemLocation::from_path(&self.folder_path).unwrap())
			.await
			.map_err(|e| SourceError { kind: SourceErrorKind::Io, source: Arc::new(e.into()) })?;
		for drive_item in drive_item_all {
			if let Some(_file) = &drive_item.file {
				if let Some(download_url) = &drive_item.download_url {
					let bytes =
						Self::download_file(download_url).await.map_err(|_e| SourceError {
							kind: SourceErrorKind::Io,
							source: Arc::new(anyhow::anyhow!("Got error while downloading file")),
						})?;
					output.write_all(&bytes).await.map_err(|_e| SourceError {
						kind: SourceErrorKind::Io,
						source: Arc::new(anyhow::anyhow!("Got error while downloading file")),
					})?;
				}
			}
		}
		Ok(())
	}
}

// #[cfg(test)]
// mod tests {

// 	use std::collections::HashSet;

// 	use futures::StreamExt;
// 	use proto::semantics::OneDriveConfig;

// 	use crate::{onedrive::onedrive::OneDriveSource, Source};

// 	#[tokio::test]
// 	async fn test_onedrive_collector() {
// 		let google_config = OneDriveConfig {
// 			client_id: "c7c05424-b4d5-4af9-8f97-9e2de234b1b4".to_string(),
//             client_secret: "I-08Q~fZ~Vsbm6Mc7rj4sqyzgjlYIA5WN5jG.cLn".to_string(),
//             redirect_uri: "http://localhost:8000/callback".to_string(),
//             refresh_token: M.C540_BAY.0.U.-Cg3wuI8L3FPX!LmwIHH1W8ChFNgervWiVAwuppNW9EC1W8iXHE797KeL!OU6*ywNfZD1*FVuVNroTPyH3HrzaP3ZiG!xepBUpmDKq1NjmXDFya6rlBABG*ahheNyOHv*WV9gYb*voX11ic00XJmxYyzEnHCxjbZ5SU75rWqzAgltIilcVoQm8VhLSeMYpRkUzDWS*Jeg6Ht8AuPJHpmetwdME7b33pOiKupGlFKn7OH1SoO7Xsc6JYcp96hneg8TS8mLg1!tVN9NkRcv1q1JjxxgLPPRXn*Xub7Y61rew91E9GdaXTAzJzFiRAL8ISH2*vq4gEzxmAG*wtfV9nMzT85JH2xxpdMvrvaXsrMrqJUm".to_string(),
//             folder_path: "/testing".to_string(),
// 		};

// 		let drive_storage = OneDriveSource::new(google_config).await.unwwrap();
// 		let connectivity = drive_storage.check_connectivity().await;

// 		println!("Connectivity: {:?}", connectivity);

// 		let result = drive_storage.poll_data().await;

// 		let mut stream = result.unwrap();
// 		let mut count_files: HashSet<String> = HashSet::new();
// 		while let Some(item) = stream.next().await {
// 			match item {
// 				Ok(collected_bytes) => {
//                     println!("Collected bytes: {:?}", collected_bytes);
// 					if let Some(pathbuf) = collected_bytes.file {
// 						if let Some(str_path) = pathbuf.to_str() {
// 							count_files.insert(str_path.to_string());
// 						}
// 					}
// 				},
// 				Err(err) => eprintln!("Expected successful data collection {:?}", err),
// 			}
// 		}
// 		println!("Files are --- {:?}", count_files);
// 	}
// }
