use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use pdf_extract::{output_doc, PlainTextOutput};
use querent_synapse::comm::IngestedTokens;
use std::{pin::Pin, sync::Arc};

use crate::{
	process_ingested_tokens_stream, AsyncProcessor, BaseIngestor, IngestorError, IngestorErrorKind,
	IngestorResult,
};

// Define the PdfIngestor
pub struct PdfIngestor {
	processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl PdfIngestor {
	pub fn new() -> Self {
		Self { processors: Vec::new() }
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
		for collected_bytes in all_collected_bytes.iter() {
			if file.is_empty() {
				file =
					collected_bytes.clone().file.unwrap_or_default().to_string_lossy().to_string();
			}
			if doc_source.is_empty() {
				doc_source = collected_bytes.doc_source.clone().unwrap_or_default();
			}
			buffer.extend_from_slice(&collected_bytes.clone().data.unwrap_or_default());
		}

		let reader = std::io::Cursor::new(buffer);
		let doc = lopdf::Document::load_from(reader)
			.map_err(|err| IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into())))?;

		let stream = stream! {
			// let pages = doc.get_pages();
			// for (i, _) in pages.iter().enumerate() {
			// 	let page_number = (i + 1) as u32;
			// 	let text = doc.extract_text(&[page_number])
			// 	.map_err(
			// 		|err| IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into())),
			// 	)?;
			// 	println!("TExt {:?}", text);
			// 	let ingested_tokens = IngestedTokens {
			// 		data: Some(vec![text]),
			// 		file: file.clone(),
			// 		doc_source: doc_source.clone(),
			// 		is_token_stream: Some(false),
			// 	};
			// 	yield Ok(ingested_tokens);
			// }

			let mut out = String::new();
			let mut output = PlainTextOutput::new(&mut out);
			let _ = output_doc(&doc, &mut output);
			yield Ok(IngestedTokens {
				data: Some(vec![out]),
				file: file.clone(),
				doc_source: doc_source.clone(),
				is_token_stream: Some(true)
			})

		};
		let processed_stream =
			process_ingested_tokens_stream(Box::pin(stream), self.processors.clone()).await;
		Ok(Box::pin(processed_stream))
	}
}

#[cfg(test)]
mod tests {
	extern crate pdf_extract;

	use pdf_extract::{output_doc, PlainTextOutput};

	#[tokio::test]
	async fn test_pdf_ingestor() {
		let bytes = std::fs::read("/home/ansh/pyg-trail/Eagle Ford Shale, USA/Asphaltene Precipitation and Deposition during Nitrogen Gas Cyclic Miscible and Immiscible Injection in Eagle Ford Shale and Its Impact on Oil Recovery.pdf").unwrap();
		let mut out = String::new();
		let mut output = PlainTextOutput::new(&mut out);

		//1
		let doc = lopdf::Document::load_mem(&bytes).unwrap();

		//2
		// let reader = std::io::Cursor::new(bytes);
		// let doc = lopdf::Document::load_from(reader).unwrap();

		//3
		// let doc = lopdf::Document::load(Path::new("/home/ansh/pyg-trail/Eagle Ford Shale, USA/Asphaltene Precipitation and Deposition during Nitrogen Gas Cyclic Miscible and Immiscible Injection in Eagle Ford Shale and Its Impact on Oil Recovery.pdf")).unwrap();

		let _ = output_doc(&doc, &mut output);

		println!("Output is {:?}", out);
	}
}
