use crate::{
	image::image::ImageIngestor, process_ingested_tokens_stream,
	processors::text_processing::TextCleanupProcessor, AsyncProcessor, BaseIngestor, IngestorError,
	IngestorErrorKind, IngestorResult,
};
use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::{Stream, StreamExt};
use pdf_extract::{output_doc, ConvertToFmt, OutputDev, OutputError, PlainTextOutput};
use proto::semantics::IngestedTokens;
use rusty_tesseract::image::guess_format;
use std::{
	collections::{BTreeMap, HashMap},
	fmt,
	path::PathBuf,
	pin::Pin,
	sync::{Arc, Mutex},
};
use tokio::io::AsyncReadExt;

// Define the PdfIngestor
pub struct PdfIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl PdfIngestor {
	pub fn new() -> Self {
		Self { processors: vec![Arc::new(TextCleanupProcessor::new())] }
	}
}

#[async_trait]
impl BaseIngestor for PdfIngestor {
	fn set_processors(&mut self, processors: Vec<Arc<dyn AsyncProcessor>>) {
		self.processors = processors;
	}

	async fn ingest(
		&self,
		all_collected_bytes: Vec<CollectedBytes>,
	) -> IngestorResult<Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send + 'static>>>
	{
		// collect all the bytes into a single buffer
		let stream = stream! {
			let mut buffer = Vec::new();
			let mut file = String::new();
			let mut doc_source = String::new();
			let mut source_id = String::new();
			for collected_bytes in all_collected_bytes {
				if file.is_empty() {
					file =
						collected_bytes.file.clone().unwrap_or_default().to_string_lossy().to_string();
				}
				if doc_source.is_empty() {
					doc_source = collected_bytes.doc_source.clone().unwrap_or_default();
				}
				if let Some(mut data) = collected_bytes.data {
					let mut buf = Vec::new();
					data.read_to_end(&mut buf).await.unwrap();
					buffer.extend_from_slice(&buf);
				}
				source_id = collected_bytes.source_id.clone();
			}
			let doc = lopdf::Document::load_mem(&buffer);
			if let Err(e) = doc {
				yield Err(IngestorError::new(IngestorErrorKind::Internal, Arc::new(e.into())));
				return;
			}
			let doc = doc.unwrap();
			if doc.is_encrypted() {
				yield Ok(IngestedTokens {
					data: vec![],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id: None,
				});
				return;
			}
			let mut output = PagePlainTextOutput::new(Arc::new(doc.clone()));
			let _ = output_doc(&doc, &mut output);
			let page_images = output.images;
			for (_, (text, has_image)) in output.pages {
				let ingested_tokens = IngestedTokens {
					data: vec![text],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
					image_id: None,
				};
				yield Ok(ingested_tokens);
				if has_image {
					for image in &page_images {
						let img_data = image.1;
						let image_id = image.0.clone();
						let file_path = PathBuf::from(file.clone());
						// Guess the format of the image data
						let format = guess_format(&img_data);
						let mut ext;
						if let Ok(f) = format {
							ext = f.to_mime_type();
							ext = ext.split("/").last().unwrap_or("jpeg");
						} else {
							log::debug!("Failed to load image from memory with image id as  {:?}", image_id);
							yield Ok(IngestedTokens {
								data: vec![],
								file: file.clone(),
								doc_source: doc_source.clone(),
								is_token_stream: false,
								source_id: source_id.clone(),
								image_id: None,
							});
							continue;
						}
						let collected_bytes = CollectedBytes {
							data: Some(Box::pin(std::io::Cursor::new(img_data.clone()))),
							file: Some(file_path.clone()),
							doc_source: Some(doc_source.clone()),
							eof: false,
							extension: Some(ext.to_string()),
							size: Some(img_data.len()),
							source_id: source_id.clone(),
							_owned_permit: None,
							image_id: Some(image_id.to_string()),
						};
						let image_ingestor = ImageIngestor::new();
						let (tx, mut rx) = tokio::sync::mpsc::channel(100);
						tokio::spawn(async move {
							let image_stream = image_ingestor.ingest(vec![collected_bytes]).await.unwrap();
							let mut image_stream = Box::pin(image_stream);
							while let Some(tokens) = image_stream.next().await {
								match tokens {
									Ok(tokens) => if !tokens.data.is_empty() {
										// only yield good tokens
										tx.send(Ok(tokens)).await.unwrap();
									},
									Err(e) => tracing::error!("Failed to get tokens from images: {:?}", e),
								}
							}
						});

						while let Some(tokens) = rx.recv().await {
							yield tokens;
						}
					}
				}
			}

			yield Ok(IngestedTokens {
				data: vec![],
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: false,
				source_id: source_id.clone(),
				image_id: None,
			})
		};

		let processed_stream: Pin<
			Box<dyn Stream<Item = Result<IngestedTokens, IngestorError>> + Send>,
		> = process_ingested_tokens_stream(Box::pin(stream), self.processors.clone()).await;
		Ok(Box::pin(processed_stream))
	}
}

pub type HasImage = bool;
struct PagePlainTextOutput {
	inner: PlainTextOutput<OutputWrapper>,
	pages: HashMap<u32, (String, HasImage)>,
	images: HashMap<u32, Vec<u8>>,
	current_page: u32,
	current_page_has_image: bool,
	reader: Arc<Mutex<String>>,
	document: Arc<lopdf::Document>,
	inner_pages: BTreeMap<u32, (u32, u16)>,
}

struct OutputWrapper(Arc<Mutex<String>>);

impl std::fmt::Write for OutputWrapper {
	fn write_str(&mut self, s: &str) -> std::fmt::Result {
		let mut reader = self.0.lock().unwrap();
		reader.write_str(s).map_err(|_| fmt::Error)
	}
}

impl ConvertToFmt for OutputWrapper {
	type Writer = OutputWrapper;

	fn convert(self) -> Self::Writer {
		self
	}
}

impl PagePlainTextOutput {
	fn new(document: Arc<lopdf::Document>) -> Self {
		let s = Arc::new(Mutex::new(String::new()));
		let writer = Arc::clone(&s);
		Self {
			inner_pages: document.get_pages().clone(),
			document,
			pages: HashMap::new(),
			current_page: 0,
			current_page_has_image: false,
			reader: s,
			inner: PlainTextOutput::new(OutputWrapper(writer)),
			images: HashMap::new(),
		}
	}
}

impl OutputDev for PagePlainTextOutput {
	fn begin_page(
		&mut self,
		page_num: u32,
		media_box: &pdf_extract::MediaBox,
		art_box: Option<(f64, f64, f64, f64)>,
	) -> Result<(), OutputError> {
		self.current_page = page_num;
		self.current_page_has_image = false;
		self.reader.lock().unwrap().clear(); // Ensure the buffer is clear at the start of each page
		let page = self.inner_pages.get(&page_num).unwrap_or(&(0, 0));
		let page_images = self.document.get_page_images(page.clone());
		if let Ok(images) = page_images {
			self.current_page_has_image = images.len() > 0;
			for image in images {
				self.images.insert(image.id.0, image.content.to_vec());
			}
		}
		self.inner.begin_page(page_num, media_box, art_box)
	}

	fn end_page(&mut self) -> Result<(), OutputError> {
		self.inner.end_page()?;
		let buf = self.reader.lock().unwrap().clone();
		self.pages.insert(self.current_page, (buf, self.current_page_has_image));
		self.reader.lock().unwrap().clear();

		Ok(())
	}

	fn output_character(
		&mut self,
		trm: &pdf_extract::Transform,
		width: f64,
		spacing: f64,
		font_size: f64,
		char: &str,
	) -> Result<(), OutputError> {
		self.inner.output_character(trm, width, spacing, font_size, char)
	}

	fn begin_word(&mut self) -> Result<(), OutputError> {
		self.inner.begin_word()
	}

	fn end_word(&mut self) -> Result<(), OutputError> {
		self.inner.end_word()
	}

	fn end_line(&mut self) -> Result<(), OutputError> {
		self.inner.end_line()
	}
}

#[cfg(test)]
mod tests {
	use futures::StreamExt;

	use super::*;
	use std::{io::Cursor, path::Path};

	#[tokio::test]
	async fn test_pdf_ingestor() {
		let included_bytes = include_bytes!("../../../../test_data/Demo.pdf");
		let bytes = included_bytes.to_vec();

		// Create a CollectedBytes instance
		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some(Path::new("dummy.pdf").to_path_buf()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("pdf".to_string()),
			size: Some(10),
			source_id: "FileSystem1".to_string(),
			_owned_permit: None,
			image_id: None,
		};

		// Create a TxtIngestor instance
		let ingestor = PdfIngestor::new();

		// Ingest the file
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
					eprintln!("Failed to get tokens: {:?}", e);
				},
			}
		}
		assert!(all_data.len() >= 1, "Unable to ingest PDF file");
	}

	#[tokio::test]
	async fn test_pdf_ingestor_with_images() {
		let included_bytes = include_bytes!("../../../../test_data/scanned.pdf");
		let bytes = included_bytes.to_vec();

		// Create a CollectedBytes instance
		let collected_bytes = CollectedBytes {
			data: Some(Box::pin(Cursor::new(bytes))),
			file: Some(Path::new("GeoExProQuerent.pdf").to_path_buf()),
			doc_source: Some("test_source".to_string()),
			eof: false,
			extension: Some("pdf".to_string()),
			size: Some(10),
			source_id: "FileSystem1".to_string(),
			_owned_permit: None,
			image_id: None,
		};

		// Create a TxtIngestor instance
		let ingestor = PdfIngestor::new();

		// Ingest the file
		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

		let mut stream = result_stream;
		let mut all_data = Vec::new();
		let mut is_image_found = false;
		while let Some(tokens) = stream.next().await {
			eprintln!("Tokens: {:?}", tokens);
			match tokens {
				Ok(tokens) =>
					if !tokens.data.is_empty() {
						if tokens.image_id.is_some() {
							is_image_found = true;
						}
						all_data.push(tokens.data);
					},
				Err(e) => {
					tracing::error!("Failed to get tokens from images: {:?}", e);
				},
			}
		}
		assert!(all_data.len() >= 1, "Unable to ingest PDF file");
		assert!(is_image_found, "Unable to ingest PDF file with images");
	}
}
