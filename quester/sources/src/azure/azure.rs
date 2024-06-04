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
		// Parse the connection string to extract account name and key
		let account = azure_storage_config.account_url;
		// TODOI dont think we need credentials once we move to rust side
		let _credentials = azure_storage_config.credentials;
		let access_key = azure_storage_config.access_key;
		let storage_credentials = StorageCredentials::access_key(account.clone(), access_key);
		// Create the BlobServiceClient
		let blob_service_client = BlobServiceClient::new(account.clone(), storage_credentials);

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
			let _ = first_blob_result?;
		}
		Ok(())
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

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		let mut blob_stream = self.container_client.list_blobs().into_stream();
		let container_client = self.container_client.clone();

		let stream = stream! {
			while let Some(blob_result) = blob_stream.next().await {
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
