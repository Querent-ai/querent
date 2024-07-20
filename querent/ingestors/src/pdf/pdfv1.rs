use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use pdf_extract::{output_doc, ConvertToFmt, OutputDev, OutputError, PlainTextOutput};
use proto::semantics::IngestedTokens;
use std::{
	collections::HashMap,
	fmt,
	path::PathBuf,
	pin::Pin,
	sync::{Arc, Mutex},
};

use crate::{
	process_ingested_tokens_stream, processors::text_processing::TextCleanupProcessor,
	AsyncProcessor, BaseIngestor, IngestorError, IngestorErrorKind, IngestorResult,
};

const LIBPDFIUM_BYTES: &[u8] = include_bytes!("resources/lib/libpdfium.so");
// Define the PdfIngestor
pub struct PdfIngestor {
	pub processors: Vec<Arc<dyn AsyncProcessor>>,
	pub libpdfium_path: String,
}

impl PdfIngestor {
	pub fn new() -> Self {
		// write the pdfium library to a file
		let temp_dir = PathBuf::from("/tmp/pdf_resources");
		let pdfium_lib_path = temp_dir.join("libpdfium.so");
		if !temp_dir.exists() {
			std::fs::create_dir_all(&temp_dir).unwrap();
			if !pdfium_lib_path.exists() {
				std::fs::write(pdfium_lib_path.clone(), LIBPDFIUM_BYTES).unwrap();
			}
		}

		Self {
			processors: vec![Arc::new(TextCleanupProcessor::new())],
			libpdfium_path: pdfium_lib_path.to_str().unwrap().to_string(),
		}
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
		let mut buffer = Vec::new();
		let mut file = String::new();
		let mut doc_source = String::new();
		let mut source_id = String::new();
		for collected_bytes in all_collected_bytes.iter() {
			if file.is_empty() {
				file =
					collected_bytes.clone().file.unwrap_or_default().to_string_lossy().to_string();
			}
			if doc_source.is_empty() {
				doc_source = collected_bytes.doc_source.clone().unwrap_or_default();
			}
			buffer.extend_from_slice(&collected_bytes.clone().data.unwrap_or_default());
			source_id = collected_bytes.source_id.clone();
		}

		let reader = std::io::Cursor::new(buffer);
		let doc = lopdf::Document::load_from(reader)
			.map_err(|err| IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into())))?;

		let stream = stream! {
			let mut output = PagePlainTextOutput::new();
			output_doc(&doc, &mut output).unwrap();
			for (_, text) in output.pages {
				println!("Text {:?}", text.clone());
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
	use std::path::PathBuf;

	use crate::pdf::pdf_document::PdfDocumentParser;

	#[test]
	fn test_pdf() {
		let pdfium_lib_path =
			"/home/querent/querent/quester/querent/ingestors/src/pdf/resources/lib/libpdfium.so";
		let parser = PdfDocumentParser::new(pdfium_lib_path);
		assert!(parser.is_ok());
		let parser = parser.unwrap();
		let file_path = PathBuf::from("/home/querent/querent/files/2112.08340v3.pdf");
		let data = std::fs::read(file_path).unwrap();
		let document = parser.parse(data.to_vec());
		assert!(document.is_ok());
	}
}
