use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use proto::semantics::IngestedTokens;
use std::{path::PathBuf, pin::Pin, sync::Arc};

#[allow(unused_imports)]
use super::init;
use super::pdf_document::PdfDocumentParser;
use crate::{
	process_ingested_tokens_stream, processors::text_processing::TextCleanupProcessor,
	AsyncProcessor, BaseIngestor, IngestorResult,
};

// Define the PdfIngestor
pub struct PdfIngestor {
	pub processors: Vec<Arc<dyn AsyncProcessor>>,
	pub libpdfium_folder_path: PathBuf,
}

impl PdfIngestor {
	pub fn new(libpdfium_folder_path: PathBuf) -> Self {
		Self { processors: vec![Arc::new(TextCleanupProcessor::new())], libpdfium_folder_path }
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
		let binary_folder = self.libpdfium_folder_path.clone();
		let (pdfium, _) = init(&binary_folder.to_string_lossy().to_string());
		let parser = PdfDocumentParser::new(pdfium);
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

		let stream = stream! {
			let document = parser.parse(buffer.clone());
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
// 	use std::path::PathBuf;

// 	use tempfile::TempDir;

// 	use crate::pdf::{init, pdf_document::PdfDocumentParser};

// 	#[test]
// 	fn test_pdf() {
// 		let binary_folder = TempDir::new().unwrap();
// 		let (pdfium, _) = init(&binary_folder.path().to_string_lossy().to_string());
// 		let parser = PdfDocumentParser::new(pdfium);
// 		let file_path = PathBuf::from("/home/querent/querent/files/2112.08340v3.pdf");
// 		let data = std::fs::read(file_path).unwrap();
// 		let document = parser.parse(data.to_vec());
// 		assert!(document.is_ok());
// 		assert!(document.unwrap().text().len() > 0);
// 	}
// }
