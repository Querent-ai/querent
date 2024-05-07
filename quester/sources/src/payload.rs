use std::{io, ops::Range};

use async_trait::async_trait;
use aws_smithy_http::byte_stream::ByteStream;

#[async_trait]
/// BlobPayload is used to upload data and support multipart.
pub trait BlobPayload: BlobPayloadClone + Send + Sync {
	/// Return the total length of the payload.
	fn len(&self) -> u64;

	/// Retrieve bytestream for specified range.
	async fn range_byte_stream(&self, range: Range<u64>) -> io::Result<ByteStream>;

	/// Retrieve complete bytestream.
	async fn byte_stream(&self) -> io::Result<ByteStream> {
		let total_len = self.len();
		let range = 0..total_len;
		self.range_byte_stream(range).await
	}

	/// Load the whole Payload into memory.
	async fn read_all(&self) -> io::Result<Vec<u8>> {
		let total_len = self.len();
		let range = 0..total_len;
		let mut reader = self.range_byte_stream(range).await?.into_async_read();

		let mut data: Vec<u8> = Vec::with_capacity(total_len as usize);
		tokio::io::copy(&mut reader, &mut data).await?;

		Ok(data)
	}
}

pub trait BlobPayloadClone {
	fn box_clone(&self) -> Box<dyn BlobPayload>;
}

impl<T> BlobPayloadClone for T
where
	T: 'static + BlobPayload + Clone,
{
	fn box_clone(&self) -> Box<dyn BlobPayload> {
		Box::new(self.clone())
	}
}

impl Clone for Box<dyn BlobPayload> {
	fn clone(&self) -> Box<dyn BlobPayload> {
		self.box_clone()
	}
}

#[async_trait]
impl BlobPayload for Vec<u8> {
	fn len(&self) -> u64 {
		self.len() as u64
	}

	async fn range_byte_stream(&self, range: Range<u64>) -> io::Result<ByteStream> {
		Ok(ByteStream::from(self[range.start as usize..range.end as usize].to_vec()))
	}
}

#[derive(Debug)]
pub struct MultiPartPolicy {
	/// Ideal part size.
	/// Since S3 has a constraint on the number of parts, it cannot always be
	/// respected.
	pub target_part_num_bytes: usize,
	/// Maximum number of parts allowed.
	pub max_num_parts: usize,
	/// Threshold above which multipart is triggered.
	pub multipart_threshold_num_bytes: u64,
	/// Maximum size allowed for an object.
	pub max_object_num_bytes: u64,
	/// Maximum number of parts to be upload concurrently.
	pub max_concurrent_uploads: usize,
}

impl MultiPartPolicy {
	/// This function returns the size of the part that should
	/// be used. We should have `part_num_bytes(len)` <= `len`.
	///
	/// If this function returns `len`, then multipart upload
	/// will not be used.
	pub fn part_num_bytes(&self, len: u64) -> u64 {
		assert!(
			len < self.max_object_num_bytes,
			"This object storage does not support object of that size {}",
			self.max_object_num_bytes
		);
		assert!(self.max_num_parts > 0, "Misconfiguration: max_num_parts == 0 makes no sense.");
		if len < self.multipart_threshold_num_bytes || self.max_num_parts == 1 {
			return len;
		}
		let max_num_parts = self.max_num_parts as u64;
		// complete part is the smallest integer such that
		// <max_num_parts> * <min_part_len> >= len.
		let min_part_len = 1u64 + (len - 1u64) / max_num_parts;
		(min_part_len).max(self.target_part_num_bytes as u64)
	}

	/// Limits the number of parts that can be concurrently uploaded.
	pub fn max_concurrent_uploads(&self) -> usize {
		self.max_concurrent_uploads
	}
}

// Default values from https://github.com/apache/hadoop/blob/trunk/hadoop-tools/hadoop-aws/src/main/java/org/apache/hadoop/fs/s3a/Constants.java
// The best default value may however differ depending on vendors.
impl Default for MultiPartPolicy {
	fn default() -> Self {
		MultiPartPolicy {
			// S3 limits part size from 5M to 5GB, we want to end up with as few parts as possible
			// since each part is charged as a put request.
			target_part_num_bytes: 5_000_000_000,               // 5GB
			multipart_threshold_num_bytes: 128 * 1_024 * 1_024, // 128 MiB
			max_num_parts: 10_000,
			max_object_num_bytes: 5_000_000_000_000u64, // S3 allows up to 5TB objects
			max_concurrent_uploads: 100,
		}
	}
}
