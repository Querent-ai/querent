use std::{
	collections::HashMap,
	fs::File,
	io::{BufReader, Cursor, Read},
	path::Path,
	pin::Pin,
};

use async_trait::async_trait;
use futures::{stream, Stream};
use serde_json::Value;
use url::Url;

use crate::{
	document_loaders::{process_doc_stream, Loader, LoaderError},
	schemas::Document,
	text_splitter::TextSplitter,
};

#[derive(Debug, Clone)]
pub struct HtmlLoader<R> {
	html: R,
	url: Url,
}

impl HtmlLoader<Cursor<Vec<u8>>> {
	pub fn from_string<S: Into<String>>(input: S, url: Url) -> Self {
		let input = input.into();
		let reader = Cursor::new(input.into_bytes());
		Self::new(reader, url)
	}
}

impl<R: Read> HtmlLoader<R> {
	pub fn new(html: R, url: Url) -> Self {
		Self { html, url }
	}
}

impl HtmlLoader<BufReader<File>> {
	pub fn from_path<P: AsRef<Path>>(path: P, url: Url) -> Result<Self, LoaderError> {
		let file = File::open(path)?;
		let reader = BufReader::new(file);
		Ok(Self::new(reader, url))
	}
}

#[async_trait]
impl<R: Read + Send + Sync + 'static> Loader for HtmlLoader<R> {
	async fn load(
		mut self,
	) -> Result<
		Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
		LoaderError,
	> {
		let cleaned_html = readability::extractor::extract(&mut self.html, &self.url)?;
		let doc = Document::new(format!("{}\n{}", cleaned_html.title, cleaned_html.text))
			.with_metadata(HashMap::from([("source".to_string(), Value::from(self.url.as_str()))]));

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
