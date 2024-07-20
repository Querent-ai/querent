use std::{fmt, ops::Range, path::Path, pin::Pin};

use async_stream::stream;
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use common::CollectedBytes;
use futures::{Stream, StreamExt};
use opendal::{Metakey, Operator};
use proto::semantics::GcsCollectorConfig;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult, REQUEST_SEMAPHORE};

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectMetadata {
	// Full path
	pub key: String,
	// Seconds since unix epoch.
	pub last_modified: f64,
	pub total_size: usize,
}

#[derive(Clone)]
pub struct OpendalStorage {
	op: Operator,
	_bucket: Option<String>,
	source_id: String,
}

impl fmt::Debug for OpendalStorage {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter
			.debug_struct("OpendalStorage")
			.field("operator", &self.op.info())
			.finish()
	}
}

impl OpendalStorage {
	/// Create a new google cloud storage.
	pub fn new_google_cloud_storage(
		cfg: opendal::services::Gcs,
		bucket: Option<String>,
		source_id: String,
	) -> Result<Self, SourceError> {
		let op = Operator::new(cfg)?.finish();
		Ok(Self { op, _bucket: bucket, source_id })
	}
}

#[async_trait]
impl Source for OpendalStorage {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
		self.op.check().await?;
		Ok(())
	}

	async fn copy_to(&self, path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
		let path = path.as_os_str().to_string_lossy();
		let mut storage_reader = self.op.reader(&path).await?;
		tokio::io::copy(&mut storage_reader, output).await?;
		output.flush().await?;
		Ok(())
	}

	async fn get_slice(&self, path: &Path, range: Range<usize>) -> SourceResult<Vec<u8>> {
		let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
		let path = path.as_os_str().to_string_lossy();
		let range = range.start as u64..range.end as u64;
		let storage_content = self.op.read_with(&path).range(range).await?;

		Ok(storage_content)
	}

	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
		let path = path.as_os_str().to_string_lossy();
		let range = range.start as u64..range.end as u64;
		let storage_reader = self.op.reader_with(&path).range(range).await?;

		Ok(Box::new(storage_reader))
	}

	async fn get_all(&self, path: &Path) -> SourceResult<Vec<u8>> {
		let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
		// let path = path.as_os_str().to_string_lossy();
		let path_str = path.to_string_lossy();
		let storage_content = self.op.read(&path_str).await?;

		Ok(storage_content)
	}

	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64> {
		let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
		let path = path.as_os_str().to_string_lossy();
		let meta = self.op.stat(&path).await?;
		Ok(meta.content_length())
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		let bucket_name = self._bucket.clone().unwrap_or_default();
		let op = self.op.clone();
		let source_id = self.source_id.clone();
		let stream = stream! {
			let mut object_lister = op.lister_with("")
				.recursive(true)
				.metakey(Metakey::ContentLength)
				.await?;

			while let Some(object) = object_lister.next().await {
				let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
				match object {
					Ok(object) => {
						let key = object.path().to_string();

						let mut storage_reader = match op.reader(&key).await {
							Ok(reader) => reader,
							Err(e) => {
								yield Err(SourceError::new(
									SourceErrorKind::Io,
									anyhow::anyhow!("Error getting reader: {:?}", e).into()
								));
								continue;
							}
						};
						let mut buffer: Vec<u8> = vec![0; 1024 * 1024 * 10]; // 10MB buffer

						loop {
							let bytes_read = match storage_reader.read(&mut buffer).await {
								Ok(bytes) => bytes,
								Err(e) => {
									yield Err(SourceError::new(
										SourceErrorKind::Io,
										anyhow::anyhow!("Error reading from storage: {:?}", e).into()
									));
									break;
								}
							};

							if bytes_read == 0 {
								break;
							}

							let collected_bytes = CollectedBytes::new(
								Some(Path::new(&key).to_path_buf()),
								Some(buffer[..bytes_read].to_vec()),
								false,
								Some(bucket_name.clone()),
								Some(bytes_read),
								source_id.clone(),
							);

							yield Ok(collected_bytes);
						}

						let eof_collected_bytes = CollectedBytes::new(
							Some(Path::new(&key).to_path_buf()),
							None,
							true,
							Some(bucket_name.clone()),
							None,
							source_id.clone(),
						);

						yield Ok(eof_collected_bytes);
					}
					Err(e) => {
						yield Err(SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Error listing object: {:?}", e).into()
						));
					}
				}
			}
		};
		Ok(Box::pin(stream))
	}
}

impl From<opendal::Error> for SourceError {
	fn from(err: opendal::Error) -> Self {
		match err.kind() {
			opendal::ErrorKind::NotFound => SourceError::new(
				SourceErrorKind::NotFound,
				anyhow::anyhow!("Opendal error: {:?}", err).into(),
			),
			opendal::ErrorKind::PermissionDenied => SourceError::new(
				SourceErrorKind::Unauthorized,
				anyhow::anyhow!("Opendal error: {:?}", err).into(),
			),
			opendal::ErrorKind::ConfigInvalid => SourceError::new(
				SourceErrorKind::NotSupported,
				anyhow::anyhow!("Opendal error: {:?}", err).into(),
			),
			_ => SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Opendal error: {:?}", err).into(),
			),
		}
	}
}

pub fn get_gcs_storage(gcs_config: GcsCollectorConfig) -> Result<OpendalStorage, SourceError> {
	let credentials = general_purpose::STANDARD.encode(&gcs_config.credentials);
	let mut cfg = opendal::services::Gcs::default();
	cfg.credential(&credentials);
	cfg.bucket(&gcs_config.bucket);
	OpendalStorage::new_google_cloud_storage(cfg, Some(gcs_config.bucket), gcs_config.id.clone())
}

// #[cfg(test)]
// mod tests {

// 	use std::fs::read;

// 	use ingestors::{pdf::pdfv1::PdfIngestor, BaseIngestor};

// 	use super::*;

// 	#[tokio::test]
// 	async fn test_gcs_collector() {
// 		// Configure the GCS collector config with a mock credential
// 		let gcs_config = GcsCollectorConfig {
// 		    bucket: "querent-testing-api".to_string(),
//             credentials: r#"{"type": "service_account",
// 			"project_id": "querent-1",
// 			"private_key_id": "461da37176987940258109b411d592e7daee9a98",
// 			"private_key": "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDQUXrTHbQE//ud\n0mUEjFnY2AOpImhJGCxl5mfiIihUNlK3ZxHGtB2BR7Qrb+6X9FBTpbxVZmna+NiB\nEJRuJCZRdiI64n86N3CHUiQFekFh23rmqR75KtKZECHBAhlRhEsIvuBg5s5ibolN\nUuFg2aZsJnCrRuJ+kYJ2YwlWi58a1KGwM6K+HUnlD7PhjsbP9KFwwF/Y3rn9LBTd\nDNMMJw8rTPFliKoTlCppFEg+cAHg0QgztCu6fF3YDpa6eQMzoDKB1VBSHRR6q4tw\ndDKIDqLO8exqKgRif8DpCafE4dNSEL9Q8oU9hLnZEClzTcdtkjVcrxpto37PVJ52\nR6EAnBehAgMBAAECggEAYSwo0ZLQ9uYbjm5mjb0Uahi1eG9i2vnKOAxGmA7b5hBi\n/Ec5XQmGm9gBPKPdVYdy8tnkJKf9p9WdVHMR8eCt+SDUbchalaLnvE++GsoA9q9F\nQJRSLONjUl/ahug+PC6sO5uiGcGAMx0hse5/0Egmn9s8gkCyBV1F0Ih5AiRl5sLB\niBuaYVJUfsZz4Xf8SmSVlLsrhuPfEAyOHPudeF+QowQj1Go4NZht2zvobuRmNjj6\nMPmhjGmSNilK1pjFgvZ7IPxINP+dsMLNIKb1CcCy3KKxbzbw09Hrm5s++78W8jVW\n67xl2cDQFeVN6Lxzs8wdP6RbrZgHnt8Sz070EOWCUwKBgQD8xmgzItvDKbHe5Zrt\ngpspEGkUzJk/1M/EGPtX3imXnLrEHXlOkZyoXlXMJva1RZzvLAhyh3Teb39N/kaW\n/4Hqvg7yhiAcnp+5IPuBSlJe+sHzta8AUl3iLBXEbyldYtY031KejpuC3YKQsazb\nWRXqz0puL9Te/cbkylqXrbe9IwKBgQDS+d8tj4jgAucQsBnnF1Io7K8zwC9v3dfC\nvslY4SYxvdHIljo8ZTNf7cNqaVcdh8LwgQRYn/RHMxqLkMJ5CO/3eFiVruGd6h5w\nJDOr6dmnIVIVL6BvZDkJnx8I5Zlrpx31fkvOzJEDCmWoe4jThm3WUEyZTLYVDWsX\nvRpohHluawKBgEU8l0gCcU2IuybBn2kVECj0TMQcspFQWkRtT1MnEB9uF54mMJb7\nvXxEsp2DwqmuUqkUV4//WFyhD66uSmmLvOsueeumH1+Xd0p/JUSptdw8NSnrBu9A\noGSWDLRMenkQ3HmI/hleGGyE/gFiGWXPhfhWJR3/TgByZKtAXgYT2DMfAoGAZN0Z\nGcsZgR9iINRQTe8UVIRzbqZfB3hkArL7yAY8IGPDu8Y2qVEosqAVYPZjs7aIODs2\nPLicLL393uOiVgMz1nguwcEOFFUtoCdunK38ZK7Fc2OFrDuaGUN9rt817gXDiO6M\nh529Zlq+J0KIM7h9IozZUiEenAoCPSMnUPikpWkCgYEAqqyg7OUc34+Q+8Amhuc+\nVlO1RYMc8bglvMn4ftgHmMxTG/po1c7tjTjT/WhqpxsZBYeD2lqOImOrRkE37754\nU8gfGDjCizUFRQwtGjBB/ROPSXAMA/OMlnmXJhS55lal/HqvDgD1deKw+nErVLHF\nKmru4vPeN8lpVgXUOX9Su94=\n-----END PRIVATE KEY-----\n",
// 			"client_email": "case-study-bucket@querent-1.iam.gserviceaccount.com",
// 			"client_id": "115491774455815151715",
// 			"auth_uri": "https://accounts.google.com/o/oauth2/auth",
// 			"token_uri": "https://oauth2.googleapis.com/token",
// 			"auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
// 			"client_x509_cert_url": "https://www.googleapis.com/robot/v1/metadata/x509/case-study-bucket%40querent-1.iam.gserviceaccount.com",
// 			"universe_domain": "googleapis.com"}"#.to_string(),
//         };

// 		// Initialize the GCS storage
// 		match get_gcs_storage(gcs_config) {
// 			Ok(storage) => {
// 				// Define the path of the file to retrieve

// 				assert!(storage.check_connectivity().await.is_ok(), "Failed to connect");

// 				let data_stream = storage.poll_data().await;
// 				assert!(data_stream.is_ok(), "Failure in polling");

// 				let pdf_ingestor = PdfIngestor::new();

// 				let mut stream_data = Vec::new();

// 				let mut stream = data_stream.unwrap();

// 				match stream.next().await {
// 					Some(Ok(collected_bytes)) => {
// 						stream_data.push(collected_bytes);
// 					},
// 					Some(Err(e)) => {
// 						eprintln!("Received error: {:?}", e);
// 					},
// 					None => {
// 						println!("No data received");
// 					},
// 				}

// 				let pdf_file_path = Path::new("/home/ansh/pyg-trail/Eagle Ford Shale, USA/Application of inorganic whole-rock geochemistry to shale resource plays_ an example from the Eagle Ford Shale Formation_ Texas..pdf");

// 				let doc = lopdf::Document::load(pdf_file_path).unwrap();
// 				let pages = doc.get_pages();
// 				for (i, _) in pages.iter().enumerate() {
// 					let page_number = (i + 1) as u32;
// 					let _text = doc.extract_text(&[page_number]);
// 				}

// 				let file_bytes = read(pdf_file_path).expect("Failed to read file");

// 				let mut stream_data1 = Vec::new();

// 				stream_data1.push(CollectedBytes {
// 					data: Some(file_bytes),
// 					file: Some(pdf_file_path.to_path_buf()),
// 					eof: false,
// 					doc_source: Some("file://".to_string()),
// 					extension: Some("pdf".to_string()),
// 					size: Some(15016),
// 				});

// 				let mut token_stream =
// 					pdf_ingestor.ingest(stream_data1).await.expect("No data received");

// 				match token_stream.next().await {
// 					Some(Ok(token)) => {
// 						println!("Tokens  ----  {:?}", token);
// 					},
// 					Some(Err(e)) => {
// 						eprintln!("Error: {:?}", e);
// 					},
// 					None => {
// 						println!("Received empty");
// 					},
// 				}
// 			},
// 			Err(e) => {
// 				println!("Failed to initialize GCS storage: {:?}", e);
// 				assert!(false, "Storage initialization failed with error: {:?}", e);
// 			},
// 		}
// 	}
// }
