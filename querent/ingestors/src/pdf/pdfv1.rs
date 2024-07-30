use crate::{
	process_ingested_tokens_stream, processors::text_processing::TextCleanupProcessor,
	AsyncProcessor, BaseIngestor, IngestorError, IngestorErrorKind, IngestorResult,
};
use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use pdf_extract::{output_doc, ConvertToFmt, OutputDev, OutputError, PlainTextOutput};
use proto::semantics::IngestedTokens;
use std::{
	collections::HashMap,
	fmt,
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
			let mut output = PagePlainTextOutput::new();
			output_doc(&doc, &mut output).unwrap();
			for (_, text) in output.pages {
				let ingested_tokens = IngestedTokens {
					data: vec![text],
					file: file.clone(),
					doc_source: doc_source.clone(),
					is_token_stream: false,
					source_id: source_id.clone(),
				};
				yield Ok(ingested_tokens);
			}
			yield Ok(IngestedTokens {
				data: vec![],
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: false,
				source_id: source_id.clone(),
			})
		};

		let processed_stream =
			process_ingested_tokens_stream(Box::pin(stream), self.processors.clone()).await;
		Ok(Box::pin(processed_stream))
	}
}

struct PagePlainTextOutput {
	inner: PlainTextOutput<OutputWrapper>,
	pages: HashMap<u32, String>,
	current_page: u32,
	reader: Arc<Mutex<String>>,
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
	fn new() -> Self {
		let s = Arc::new(Mutex::new(String::new()));
		let writer = Arc::clone(&s);
		Self {
			pages: HashMap::new(),
			current_page: 0,
			reader: s,
			inner: PlainTextOutput::new(OutputWrapper(writer)),
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
		self.reader.lock().unwrap().clear(); // Ensure the buffer is clear at the start of each page
		self.inner.begin_page(page_num, media_box, art_box)
	}

	fn end_page(&mut self) -> Result<(), OutputError> {
		self.inner.end_page()?;

		let buf = self.reader.lock().unwrap().clone();
		self.pages.insert(self.current_page, buf);
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
		};

		// Create a TxtIngestor instance
		let ingestor = PdfIngestor::new();

		// Ingest the file
		let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

		let mut stream = result_stream;
		while let Some(tokens) = stream.next().await {
			let tokens = tokens.unwrap();
			println!("These are the tokens in file --------------{:?}", tokens);
		}
	}
}
