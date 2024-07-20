use std::{fmt, io, num::NonZeroU32, ops::Range, path::Path, pin::Pin, sync::Arc};

use async_stream::stream;
use async_trait::async_trait;
use azure_core::{error::ErrorKind, Pageable, StatusCode};
use azure_storage::{Error as AzureError, StorageCredentials};
use azure_storage_blobs::{blob::operations::GetBlobResponse, prelude::*};
use common::{CollectedBytes, Retryable};
use futures::{
	io::{Error as FutureError, ErrorKind as FutureErrorKind},
	stream::{StreamExt, TryStreamExt},
	Stream,
};
use proto::semantics::AzureCollectorConfig;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio_util::{compat::FuturesAsyncReadCompatExt, io::StreamReader};
use tracing::instrument;

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult, REQUEST_SEMAPHORE};

/// Azure object storage implementation
pub struct AzureBlobStorage {
	container_client: ContainerClient,
	prefix: String,
	source_id: String,
}

impl fmt::Debug for AzureBlobStorage {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("AzureBlobStorage").finish()
	}
}

impl AzureBlobStorage {
	/// Creates a new [`AzureBlobStorage`] instance.
	pub fn new(azure_storage_config: AzureCollectorConfig) -> Self {
		let connection_string = &azure_storage_config.connection_string;
		let account_name =
			Self::extract_value_from_connection_string(connection_string, "AccountName");
		let account_key = azure_storage_config.credentials;

		let storage_credentials = StorageCredentials::access_key(account_name.clone(), account_key);
		// Create the BlobServiceClient
		let blob_service_client = ClientBuilder::new(account_name, storage_credentials);
		// Create the ContainerClient
		let container_client =
			blob_service_client.container_client(azure_storage_config.container.clone());

		Self {
			container_client,
			prefix: azure_storage_config.prefix,
			source_id: azure_storage_config.id.clone(),
		}
	}

	/// Returns the blob name (a.k.a blob key).
	fn blob_name(&self, relative_path: &Path) -> String {
		let mut name = self.prefix.clone();
		name.push_str(relative_path.to_string_lossy().as_ref());
		name
	}

	fn extract_value_from_connection_string(connection_string: &str, key: &str) -> String {
		connection_string
			.split(';')
			.find(|part| part.starts_with(key))
			.and_then(|part| part.split('=').nth(1))
			.unwrap_or_default()
			.to_string()
	}

	/// Downloads a blob as vector of bytes.
	async fn get_to_vec(
		&self,
		path: &Path,
		range_opt: Option<Range<usize>>,
	) -> SourceResult<Vec<u8>> {
		let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
		let name = self.blob_name(path);
		let capacity = range_opt.as_ref().map(Range::len).unwrap_or(0);

		let mut response_stream = if let Some(range) = range_opt.as_ref() {
			self.container_client
				.blob_client(&name)
				.get()
				.range(range.clone())
				.into_stream()
		} else {
			self.container_client.blob_client(&name).get().into_stream()
		};

		let mut buf: Vec<u8> = Vec::with_capacity(capacity);
		download_all(&mut response_stream, &mut buf).await?;

		Ok(buf)
	}
}

/// Collect a download stream into an output buffer.
async fn download_all(
	chunk_stream: &mut Pageable<GetBlobResponse, AzureError>,
	output: &mut Vec<u8>,
) -> Result<(), AzureErrorWrapper> {
	output.clear();
	while let Some(chunk_result) = chunk_stream.next().await {
		let chunk_response = chunk_result?;
		let chunk_response_body_stream = chunk_response
			.data
			.map_err(|err| FutureError::new(FutureErrorKind::Other, err))
			.into_async_read()
			.compat();
		let mut body_stream_reader = BufReader::new(chunk_response_body_stream);
		tokio::io::copy_buf(&mut body_stream_reader, output).await?;
	}
	output.shrink_to_fit();
	Ok(())
}

#[async_trait]
impl Source for AzureBlobStorage {
	async fn copy_to(&self, path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
		let name = self.blob_name(path);
		let mut output_stream = self.container_client.blob_client(name).get().into_stream();

		while let Some(chunk_result) = output_stream.next().await {
			let chunk_response = chunk_result.map_err(AzureErrorWrapper::from)?;
			let chunk_response_body_stream = chunk_response
				.data
				.map_err(|err| FutureError::new(FutureErrorKind::Other, err))
				.into_async_read()
				.compat();
			let mut body_stream_reader = BufReader::new(chunk_response_body_stream);
			tokio::io::copy_buf(&mut body_stream_reader, output).await?;
		}
		output.flush().await?;
		Ok(())
	}
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
		if let Some(first_blob_result) = self
			.container_client
			.list_blobs()
			.max_results(NonZeroU32::new(1u32).expect("1 is always non-zero."))
			.into_stream()
			.next()
			.await
		{
			match first_blob_result {
				Ok(_) => Ok(()),
				Err(e) => {
					eprintln!("Error connecting to Azure Blob Storage: {}", e);
					Err(e.into())
				},
			}
		} else {
			Err(anyhow::anyhow!("Failed to get first blob result"))
		}
	}

	#[instrument(level = "debug", skip(self, range), fields(range.start = range.start, range.end = range.end))]
	async fn get_slice(&self, path: &Path, range: Range<usize>) -> SourceResult<Vec<u8>> {
		self.get_to_vec(path, Some(range.clone())).await
	}

	#[instrument(level = "debug", skip(self, range), fields(range.start = range.start, range.end = range.end))]
	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
		let range = range.clone();
		let name = self.blob_name(path);
		let page_stream = self.container_client.blob_client(name).get().range(range).into_stream();
		let mut bytes_stream = page_stream
			.map(|page_res| {
				page_res
					.map(|page| page.data)
					.map_err(|err| FutureError::new(FutureErrorKind::Other, err))
			})
			.try_flatten()
			.map(|e| e.map_err(|err| FutureError::new(FutureErrorKind::Other, err)));
		// Peek into the stream so that any early error can be retried
		let first_chunk = bytes_stream.next().await;
		let reader: Box<dyn AsyncRead + Send + Unpin> = if let Some(res) = first_chunk {
			let first_chunk = res.map_err(AzureErrorWrapper::from)?;
			let reconstructed_stream =
				Box::pin(futures::stream::once(async { Ok(first_chunk) }).chain(bytes_stream));
			Box::new(StreamReader::new(reconstructed_stream))
		} else {
			Box::new(tokio::io::empty())
		};
		Ok(reader)
	}

	#[instrument(level = "debug", skip(self), fields(fetched_bytes_len))]
	async fn get_all(&self, path: &Path) -> SourceResult<Vec<u8>> {
		let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
		let data = self.get_to_vec(path, None).await?;
		tracing::Span::current().record("fetched_bytes_len", data.len());
		Ok(data)
	}

	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64> {
		let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
		let name = self.blob_name(path);
		let properties_result =
			self.container_client.blob_client(name).get_properties().into_future().await;
		match properties_result {
			Ok(response) => Ok(response.blob.properties.content_length),
			Err(err) => Err(SourceError::from(AzureErrorWrapper::from(err))),
		}
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		let mut blob_stream = self.container_client.list_blobs().into_stream();
		let container_client = self.container_client.clone();
		let source_id = self.source_id.clone();

		let stream = stream! {
			while let Some(blob_result) = blob_stream.next().await {
				let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
				let blob = match blob_result {
					Ok(blob) => blob,
					Err(err) => {
						yield Err(SourceError::from(AzureErrorWrapper::from(err)));
						continue;
					},
				};

				let blobs_list = blob.blobs;

				for blob_info in blobs_list.blobs() {
					let blob_name = blob_info.name.clone();
					let blob_path = Path::new(&blob_name);

					let mut output_stream =
						container_client.blob_client(&blob_name).get().into_stream();

					while let Some(chunk_result) = output_stream.next().await {
						let chunk_response = match chunk_result {
							Ok(response) => response,
							Err(err) => {
								yield Err(SourceError::from(AzureErrorWrapper::from(err)));
								continue;
							},
						};

						let chunk_response_body_stream = chunk_response
							.data
							.map_err(|err| FutureError::new(FutureErrorKind::Other, err))
							.into_async_read()
							.compat();

						let mut body_stream_reader = BufReader::new(chunk_response_body_stream);

						let mut buffer: Vec<u8> = vec![0; 1024 * 1024 * 10]; // 10MB buffer
						loop {
							let bytes_read = body_stream_reader.read(&mut buffer).await?;

							// Break the loop if EOF is reached
							if bytes_read == 0 {
								break;
							}
							// Only process and serialize if bytes were read
							let collected_bytes = CollectedBytes::new(
								Some(blob_path.to_path_buf()),
								Some(buffer[..bytes_read].to_vec()),
								false,
								Some(container_client.container_name().to_string()),
								Some(bytes_read as usize),
								source_id.clone(),
							);

							yield Ok(collected_bytes);
						}

						// Create CollectedBytes instance
						let collected_bytes = CollectedBytes::new(
							Some(blob_path.to_path_buf()),
							None,
							true,
							Some(container_client.container_name().to_string()),
							None,
							source_id.clone(),
						);
						yield Ok(collected_bytes);
					}
				}
			}
		};

		Ok(Box::pin(stream))
	}
}

#[derive(Error, Debug)]
#[error("Azure error wrapper(inner={inner})")]
struct AzureErrorWrapper {
	inner: AzureError,
}

impl Retryable for AzureErrorWrapper {
	fn is_retryable(&self) -> bool {
		match self.inner.kind() {
			ErrorKind::HttpResponse { status, .. } => !matches!(
				status,
				StatusCode::NotFound |
					StatusCode::Unauthorized |
					StatusCode::BadRequest |
					StatusCode::Forbidden
			),
			ErrorKind::Io => true,
			_ => false,
		}
	}
}

impl From<AzureError> for AzureErrorWrapper {
	fn from(err: AzureError) -> Self {
		AzureErrorWrapper { inner: err }
	}
}

impl From<io::Error> for AzureErrorWrapper {
	fn from(err: io::Error) -> Self {
		AzureErrorWrapper { inner: AzureError::new(ErrorKind::Io, err) }
	}
}

impl From<AzureErrorWrapper> for SourceError {
	fn from(err: AzureErrorWrapper) -> Self {
		match err.inner.kind() {
			ErrorKind::HttpResponse { status, .. } => match status {
				StatusCode::NotFound =>
					SourceError::new(SourceErrorKind::NotFound, Arc::new(err.into())),
				_ => SourceError::new(SourceErrorKind::Service, Arc::new(err.into())),
			},
			ErrorKind::Io => SourceError::new(SourceErrorKind::Io, Arc::new(err.into())),
			ErrorKind::Credential =>
				SourceError::new(SourceErrorKind::Unauthorized, Arc::new(err.into())),
			_ => SourceError::new(SourceErrorKind::Service, Arc::new(err.into())),
		}
	}
}

// #[cfg(test)]
// mod tests {

// 	use std::collections::HashSet;

// use super::*;

// 	#[tokio::test]
// 	async fn test_azure_collector() {
// 		// Configure the GCS collector config with a mock credential
// 		let azure_config = AzureCollectorConfig {
// 		    account_url: "".to_string(),
// 			connection_string: "DefaultEndpointsProtocol=https;AccountName=querent;AccountKey=AB6gsGVuwGDs3OoFzJc0eQ4OtRj35wYgHGt3PPLafCHye3Ze9xw6t4cZfUNIXM5pNoBMGeehGUDo+AStQSTTnQ==;EndpointSuffix=core.windows.net".to_string(),
// 			credentials: "AB6gsGVuwGDs3OoFzJc0eQ4OtRj35wYgHGt3PPLafCHye3Ze9xw6t4cZfUNIXM5pNoBMGeehGUDo+AStQSTTnQ==".to_string(),
// 			container: "testfiles".to_string(),
// 			chunk_size: 1024,
// 			prefix: "".to_string(),
//         };

// 		let azure_storage = AzureBlobStorage::new(azure_config);

// 		assert!(
// 			azure_storage.check_connectivity().await.is_ok(),
// 			"Failed to connect to azure storage"
// 		);

// 		let result = azure_storage.poll_data().await;

// 		let mut stream = result.unwrap();
// 		let mut count_files: HashSet<String> = HashSet::new();
// 		while let Some(item) = stream.next().await {
// 			match item {
// 				Ok(collected_bytes) => {

// 					if let Some(pathbuf) = collected_bytes.file {
// 						if let Some(str_path) = pathbuf.to_str() {
// 							count_files.insert(str_path.to_string());
// 						}
// 					}
// 				}
// 				Err(_) => panic!("Expected successful data collection"),
// 			}
// 		}
// 		println!("Files are --- {:?}", count_files);

// 	}
// }
