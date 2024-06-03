use std::{fmt, io, num::NonZeroU32, ops::Range, path::Path, sync::Arc};

use async_trait::async_trait;
use azure_core::{error::ErrorKind, Pageable, StatusCode};
use azure_storage::{Error as AzureError, StorageCredentials};
use azure_storage_blobs::{blob::operations::GetBlobResponse, prelude::*};
use common::{CollectedBytes, Retryable};
use futures::{
	io::{Error as FutureError, ErrorKind as FutureErrorKind},
	stream::{StreamExt, TryStreamExt},
};
use proto::semantics::AzureCollectorConfig;
use thiserror::Error;
use tokio::{
	io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
	sync::mpsc,
};
use tokio_util::{compat::FuturesAsyncReadCompatExt, io::StreamReader};
use tracing::instrument;

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};

/// Azure object storage implementation
pub struct AzureBlobStorage {
	container_client: ContainerClient,
	prefix: String,
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
		let account_key =  azure_storage_config.credentials;

		let storage_credentials = StorageCredentials::access_key(account_name.clone(), account_key);
		// Create the BlobServiceClient
		let blob_service_client = ClientBuilder::new(account_name, storage_credentials);

		// Create the ContainerClient
		let container_client =
			blob_service_client.container_client(azure_storage_config.container.clone());

		Self { container_client, prefix: azure_storage_config.prefix }
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
	async fn list_files(&self, _path: &Path) -> SourceResult<Vec<String>> {
		let mut blob_names = Vec::new();
		let mut stream = self.container_client.list_blobs().into_stream();

		while let Some(result) = stream.next().await {
			let segment = result.map_err(|e| SourceError::from(AzureErrorWrapper::from(e)))?;
			for blob in segment.blobs.blobs() {
				blob_names.push(blob.name.clone());
			}
		}

		Ok(blob_names)
	}

	async fn copy_to(&self, path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
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
		if let Some(first_blob_result) = self
			.container_client
			.list_blobs()
			.max_results(NonZeroU32::new(1u32).expect("1 is always non-zero."))
			.into_stream()
			.next()
			.await
		{
			match first_blob_result {
				Ok(_) => {
					Ok(())
				},
				Err(e) => {
					println!("Error connecting to Azure Blob Storage: {}", e);
					Err(e.into())
				},
			}
		} else {
			println!("Failed to get first blob result");
			Err(anyhow::anyhow!("Failed to connect to Azure Blob Storage"))
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
		let data = self.get_to_vec(path, None).await?;
		tracing::Span::current().record("fetched_bytes_len", data.len());
		Ok(data)
	}

	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64> {
		let name = self.blob_name(path);
		let properties_result =
			self.container_client.blob_client(name).get_properties().into_future().await;
		match properties_result {
			Ok(response) => Ok(response.blob.properties.content_length),
			Err(err) => Err(SourceError::from(AzureErrorWrapper::from(err))),
		}
	}

	async fn poll_data(&self, output: mpsc::Sender<CollectedBytes>) -> SourceResult<()> {
		let mut blob_stream = self.container_client.list_blobs().into_stream();

		while let Some(blob_result) = blob_stream.next().await {
			let blob = match blob_result {
				Ok(blob) => blob,
				Err(err) => return Err(SourceError::from(AzureErrorWrapper::from(err))),
			};

			let blobs_list = blob.blobs;

			for blob_info in blobs_list.blobs() {
				let blob_name = blob_info.name.clone();
				let blob_path = Path::new(&blob_name);

				let mut output_stream =
					self.container_client.blob_client(&blob_name).get().into_stream();

				while let Some(chunk_result) = output_stream.next().await {
					let chunk_response = match chunk_result {
						Ok(response) => response,
						Err(err) => return Err(SourceError::from(AzureErrorWrapper::from(err))),
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
							Some(self.container_client.container_name().to_string()),
							Some(bytes_read as usize),
						);

						output.send(CollectedBytes::from(collected_bytes)).await.map_err(|e| {
							SourceError::new(
								SourceErrorKind::Io,
								anyhow::anyhow!("Error sending collected bytes: {:?}", e).into(),
							)
						})?;
					}

					// Create CollectedBytes instance
					let collected_bytes = CollectedBytes::new(
						Some(blob_path.to_path_buf()),
						None,
						true,
						Some(self.container_client.container_name().to_string()),
						None,
					);
					output.send(CollectedBytes::from(collected_bytes)).await.map_err(|e| {
						SourceError::new(
							SourceErrorKind::Io,
							anyhow::anyhow!("Error sending collected bytes: {:?}", e).into(),
						)
					})?;
				}
			}
		}

		Ok(())
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

#[cfg(test)]
mod tests {

	use super::*;
	use tokio::io::AsyncReadExt;

	#[tokio::test]
	async fn test_azure_collector() {
		// Configure the GCS collector config with a mock credential
		let azure_config = AzureCollectorConfig {
		    account_url: "".to_string(),
			connection_string: "DefaultEndpointsProtocol=https;AccountName=querent;AccountKey=AB6gsGVuwGDs3OoFzJc0eQ4OtRj35wYgHGt3PPLafCHye3Ze9xw6t4cZfUNIXM5pNoBMGeehGUDo+AStQSTTnQ==;EndpointSuffix=core.windows.net".to_string(),
			credentials: "AB6gsGVuwGDs3OoFzJc0eQ4OtRj35wYgHGt3PPLafCHye3Ze9xw6t4cZfUNIXM5pNoBMGeehGUDo+AStQSTTnQ==".to_string(),
			container: "testfiles".to_string(),
			chunk_size: 1024,
			prefix: "".to_string(),
        };

		let azure_storage = AzureBlobStorage::new(azure_config);


		assert!(
			azure_storage.check_connectivity().await.is_ok(),
			"Failed to connect to azure storage"
		);

		let files_list = azure_storage.list_files(Path::new("")).await;

		// Initialize the azure storage
		match  files_list{
			Ok(file_names) =>
				for file_name in file_names {
					let path = Path::new(&file_name);
					let mut stream = match azure_storage.get_slice_stream(path, 0..usize::MAX).await
					{
						Ok(stream) => stream,
						Err(e) => {
							eprintln!("Failed to get stream for {}: {:?}", file_name, e);
							continue;
						},
					};

					let mut contents = Vec::new();
					if let Err(e) = stream.read_to_end(&mut contents).await {
						eprintln!("Failed to read contents of {}: {:?}", file_name, e);
					}
				},
			Err(e) => {
				println!("Failed to initialize Azure storage: {:?}", e);
				assert!(false, "Storage initialization failed with error: {:?}", e);
			},
		}
	}
}
