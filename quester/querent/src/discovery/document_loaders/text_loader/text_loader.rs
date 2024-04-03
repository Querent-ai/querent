use std::pin::Pin;

use async_trait::async_trait;
use futures::{stream, Stream};

use crate::{
	document_loaders::{process_doc_stream, Loader, LoaderError},
	schemas::Document,
	text_splitter::TextSplitter,
};

#[derive(Debug, Clone)]
pub struct TextLoader {
	content: String,
}

impl TextLoader {
	pub fn new<T: Into<String>>(input: T) -> Self {
		Self { content: input.into() }
	}
}

#[async_trait]
impl Loader for TextLoader {
	async fn load(
		mut self,
	) -> Result<
		Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
		LoaderError,
	> {
		let doc = Document::new(self.content);
		let stream = stream::iter(vec![Ok(doc)]);
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
