use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use pdfium_render::pdfium::Pdfium;
use proto::semantics::IngestedTokens;
use std::{pin::Pin, sync::Arc};

#[allow(unused_imports)]
use super::init;
use super::{pdf_document::PdfDocumentParser, PDFIUM};
use crate::{
	process_ingested_tokens_stream, processors::text_processing::TextCleanupProcessor,
	AsyncProcessor, BaseIngestor, IngestorError, IngestorErrorKind, IngestorResult,
};

// Define the PdfIngestor
pub struct PdfIngestor {
	pub processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl PdfIngestor {
	pub fn new() -> Self {
		Self { processors: vec![Arc::new(TextCleanupProcessor::new())] }
	}

	pub fn get_pdfium_instance() -> Result<&'static Pdfium, IngestorError> {
		PDFIUM.get().ok_or_else(|| {
			IngestorError::new(
				IngestorErrorKind::Internal,
				Arc::new(anyhow::anyhow!("PDFium not initialized")),
			)
		})
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
			if collected_bytes.data.is_none() || collected_bytes.file.is_none() {
				continue;
			}
			if file.is_empty() {
				file = collected_bytes.file.as_ref().unwrap().to_string_lossy().to_string();
			}
			if doc_source.is_empty() {
				doc_source = collected_bytes.doc_source.clone().unwrap_or_default();
			}
			buffer.extend_from_slice(collected_bytes.data.as_ref().unwrap().as_slice());
			source_id = collected_bytes.source_id.clone();
		}
		let pdfium = Self::get_pdfium_instance()?;
		let stream = stream! {
			let parser = PdfDocumentParser::new();
			let document = parser.parse(buffer,pdfium);
			let pages = document.unwrap().all_texts();
			for text in pages {
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

// #[cfg(test)]
// mod tests {
// 	use std::{path::PathBuf, sync::Arc};

// 	use tempfile::TempDir;

// 	use crate::pdf::{init, pdf_document::PdfDocumentParser};

// 	#[test]
// 	fn test_pdf() {
// 		let binary_folder = TempDir::new().unwrap();
// 		let binary_folder2 = TempDir::new().unwrap();

// 		let (pdfium, _) = init(&binary_folder.path().to_string_lossy().to_string());
// 		let (pdfium2, _) = init(&binary_folder2.path().to_string_lossy().to_string());

// 		let parser = PdfDocumentParser::new(Arc::new(pdfium));
// 		let parser2 = PdfDocumentParser::new(Arc::new(pdfium2));
// 		let file_path = PathBuf::from("/home/querent/querent/files/2112.08340v3.pdf");
// 		let data = std::fs::read(file_path).unwrap();
// 		let document = parser.parse(data.to_vec());
// 		let document2 = parser2.parse(data.to_vec());
// 		assert!(document.is_ok());
// 		assert!(document.unwrap().text().len() > 0);
// 		assert!(document2.is_ok());
// 		assert!(document2.unwrap().text().len() > 0);
// 	}
// }
