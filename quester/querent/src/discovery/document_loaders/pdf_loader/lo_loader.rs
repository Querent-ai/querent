use std::{collections::HashMap, io::Read, path::Path, pin::Pin};

use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;

use crate::{
	document_loaders::{process_doc_stream, Loader, LoaderError},
	schemas::Document,
	text_splitter::TextSplitter,
};

#[derive(Debug, Clone)]
pub struct LoPdfLoader {
	document: lopdf::Document,
}

impl LoPdfLoader {
	/// Creates a new PdfLoader from anything that implements the Read trait.
	/// This is a generic constructor which can be used with any type of reader.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use std::io::Cursor;
	/// let data = Cursor::new(vec![...] /* some PDF data */);
	/// let loader = PdfLoader::new(data)?;
	/// ```
	///
	pub fn new<R: Read>(reader: R) -> Result<Self, LoaderError> {
		let document = lopdf::Document::load_from(reader)?;
		Ok(Self { document })
	}
	/// Creates a new PdfLoader from a path to a PDF file.
	/// This loads the PDF document and creates a PdfLoader from it.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// let loader = PdfLoader::from_path("/path/to/my.pdf")?;
	/// ```
	///
	pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoaderError> {
		let document = lopdf::Document::load(path)?;
		Ok(Self { document })
	}
}

#[async_trait]
impl Loader for LoPdfLoader {
	async fn load(
		mut self,
	) -> Result<
		Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
		LoaderError,
	> {
		let stream = stream! {
			let pages = self.document.get_pages();
			for (i, _) in pages.iter().enumerate() {
				let page_number = (i + 1) as u32;
				let text = self.document.extract_text(&[page_number])?;
				let mut metadata = HashMap::new();
				metadata.insert("page_number".to_string(), Value::from(page_number));
				let doc=Document::new(text).with_metadata(metadata);
				yield Ok(doc);

			}
		};

		Ok(Box::pin(stream))
	}

	async fn load_and_split<TS: TextSplitter + 'static>(
		mut self,
		splitter: TS,
	) -> Result<
		Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
		LoaderError,
	> {
		let doc_stream = self.load().await?;
		let stream = process_doc_stream(doc_stream, splitter).await;
		Ok(Box::pin(stream))
	}
}
