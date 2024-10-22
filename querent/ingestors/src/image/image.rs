use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use once_cell::sync::Lazy;
use proto::semantics::IngestedTokens;
use std::{pin::Pin, sync::Arc};
use tokio::{io::AsyncReadExt, sync::Semaphore};

use crate::{
	process_ingested_tokens_stream, processors::text_processing::TextCleanupProcessor,
	AsyncProcessor, BaseIngestor, IngestorResult,
};

static REQUEST_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(10));
static SUPPORTED_EXTENSIONS: Lazy<Vec<&'static str>> = Lazy::new(|| {
	vec!["jpeg", "jpg", "png", "bmp", "gif", "pnm", "tiff", "webp", "dds", "ico", "ff"]
});
// Define the ImageIngestor
pub struct ImageIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl ImageIngestor {
	pub fn new() -> Self {
		Self { processors: vec![Arc::new(TextCleanupProcessor::new())] }
	}
}

#[async_trait]
impl BaseIngestor for ImageIngestor {
	fn set_processors(&mut self, processors: Vec<Arc<dyn AsyncProcessor>>) {
		self.processors = processors;
	}

	async fn ingest(
		&self,
		all_collected_bytes: Vec<CollectedBytes>,
	) -> IngestorResult<Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send + 'static>>>
	{
		let stream = stream! {
			let mut buffer = Vec::new();
			let mut file = String::new();
			let mut doc_source = String::new();
			let mut source_id = String::new();
			let mut image_id: Option<String> = None;
			let mut extension = String::new();
			for collected_bytes in all_collected_bytes {
				if collected_bytes.data.is_none() || collected_bytes.file.is_none() {
					continue;
				}
				if file.is_empty() {
					file = collected_bytes.file.as_ref().unwrap().to_string_lossy().to_string();
				}
				if doc_source.is_empty() {
					doc_source = collected_bytes.doc_source.clone().unwrap_or_default();
				}
				if image_id.is_none() {
					image_id = collected_bytes.image_id.clone();
				}
				if extension.is_empty() {
					extension = collected_bytes.extension.clone().unwrap_or_default();
				}
				if let Some(mut data) = collected_bytes.data {
					let mut buf = Vec::new();
					if let Err(e) = data.read_to_end(&mut buf).await {
						tracing::error!("Failed to read image data: {:?}", e);
						continue;
					}
					buffer.extend_from_slice(&buf);
				}
				source_id = collected_bytes.source_id.clone();
			}
			if extension.is_empty() || !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
				return yield Ok(IngestedTokens {
					data: vec![],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id,
				});
			}
			// Acquire semaphore for image processing
			let permit_res = REQUEST_SEMAPHORE.acquire().await;
			if permit_res.is_err() {
				tracing::error!("Failed to acquire semaphore: {:?}", permit_res.err());
				return yield Ok(IngestedTokens {
					data: vec![],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id,
				});
			}
			let _permit = permit_res.unwrap();

			let img = image::load_from_memory(&buffer);
			if img.is_err() {
				tracing::error!("Failed to load image from memory: {:?}", img.err());
				return yield Ok(IngestedTokens {
					data: vec![],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id,
				});
			}
			let img = img.unwrap();
			if img.width() <= 3 || img.height() <= 36 {
				// too small to OCR
				return yield Ok(IngestedTokens {
					data: vec![],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id,
				});
			}
			let img = rusty_tesseract::Image::from_dynamic_image(&img);
			if img.is_err() {
				tracing::error!("Failed to convert image to tesseract image: {:?}", img.err());
				return yield Ok(IngestedTokens {
					data: vec![],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id,
				});
			}

			let img = img.unwrap();
			let default_args = rusty_tesseract::Args::default();
			let output = rusty_tesseract::image_to_string(&img, &default_args);
			if output.is_err() {
				tracing::error!("Failed to get text from image: {:?}", output.err());
				return yield Ok(IngestedTokens {
					data: vec![],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id,
				});
			}
			let output = output.unwrap();
			let ingested_tokens = IngestedTokens {
				data: vec![output.to_string()],
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: false,
				source_id: source_id.clone(),
				image_id,
			};

			yield Ok(ingested_tokens);

			yield Ok(IngestedTokens {
				data: vec![],
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: false,
				source_id: source_id.clone(),
				image_id: None,
			})
		};

		let processed_stream =
			process_ingested_tokens_stream(Box::pin(stream), self.processors.clone()).await;
		Ok(Box::pin(processed_stream))
	}
}

#[cfg(test)]
mod tests {
	use futures::StreamExt;
	use tokio::fs;

	use super::*;
	use std::{io::Cursor, path::Path};

	#[tokio::test]
	async fn test_image_ingestor() {
		let test_images_dir = Path::new("../../test_data/images/");
		let test_images = vec![
			("output.jpeg", "jpeg"),
			("input.jpg", "jpg"),
			("output.png", "png"),
			("output.bmp", "bmp"),
			("output.gif", "gif"),
			("output.pnm", "pnm"),
			("output.tiff", "tiff"),
			("output.webp", "webp"),
			("output.dds", "dds"),
			("output.dds", "ff"),
			("output.ico", "ico"),
		];

		for (file_name, ext) in test_images {
			let image_path = test_images_dir.join(file_name);
			let included_bytes = fs::read(&image_path).await.expect("Failed to read image file");
			let bytes = included_bytes.to_vec();
			let collected_bytes = CollectedBytes {
				data: Some(Box::pin(Cursor::new(bytes))),
				file: Some(image_path),
				doc_source: Some("test_source".to_string()),
				eof: false,
				extension: Some(ext.to_string()),
				size: Some(10),
				source_id: "FileSystem1".to_string(),
				_owned_permit: None,
				image_id: None,
			};
			let ingestor = ImageIngestor::new();
			let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

			let mut stream = result_stream;
			let mut all_data = Vec::new();
			while let Some(tokens) = stream.next().await {
				match tokens {
					Ok(tokens) =>
						if !tokens.data.is_empty() {
							all_data.push(tokens.data);
						},
					Err(e) => {
						eprintln!("Failed to get tokens for {}: {:?}", file_name, e);
					},
				}
			}
			assert!(
				all_data.len() >= 1,
				"Unable to ingest image file: {} with extension: {}",
				file_name,
				ext
			);
		}
	}
}
