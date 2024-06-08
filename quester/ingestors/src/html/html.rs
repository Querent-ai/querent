use async_stream::stream;
use async_trait::async_trait;
use common::CollectedBytes;
use futures::Stream;
use querent_synapse::comm::IngestedTokens;
use std::{io::Cursor, pin::Pin, sync::Arc};
use tokio::io::AsyncReadExt;


use crate::{
    process_ingested_tokens_stream, AsyncProcessor, BaseIngestor, IngestorError, IngestorErrorKind,
    IngestorResult,
};

// Define the TxtIngestor
pub struct HtmlIngestor {
    processors: Vec<Arc<dyn AsyncProcessor>>,
}

impl HtmlIngestor {
    pub fn new() -> Self {
        Self { processors: Vec::new() }
    }
}

#[async_trait]
impl BaseIngestor for HtmlIngestor {
    fn set_processors(&mut self, processors: Vec<Arc<dyn AsyncProcessor>>) {
        self.processors = processors;
    }

    async fn ingest(
        &self,
        all_collected_bytes: Vec<CollectedBytes>,
    ) -> IngestorResult<Pin<Box<dyn Stream<Item = IngestorResult<IngestedTokens>> + Send + 'static>>>
    {
        let mut buffer = Vec::new();
        let mut file = String::new();
        let mut doc_source = String::new();
        for collected_bytes in all_collected_bytes.iter() {
            if file.is_empty() {
                file = collected_bytes.clone().file.unwrap_or_default().to_string_lossy().to_string();
            }
            if doc_source.is_empty() {
                doc_source = collected_bytes.doc_source.clone().unwrap_or_default();
            }
            buffer.extend_from_slice(&collected_bytes.clone().data.unwrap_or_default());
        }

        let buffer = Arc::new(buffer);
        let stream = {
            let buffer = Arc::clone(&buffer);
            stream! {
                let mut content = String::new();
                let mut cursor = Cursor::new(buffer.as_ref());

                cursor.read_to_string(&mut content).await
                    .map_err(|err| IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into())))?;

                println!("{}", content);

                // let dom = tl::parse(&content, tl::ParserOptions::default()).unwrap();
                // let parser = dom.parser();
                // let element = dom.get_element_by_id("body")
                // .expect("Failed to find element")
                // .get(parser)
                // .unwrap();

                // println!("body     =================== {}", element.inner_text(parser));

                // let url = Url::parse("http://www.querent.xyz").map_err(|err| IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into())))?;
                // let cleaned_html = extract(&mut cursor, &url).map_err(|err| IngestorError::new(IngestorErrorKind::Io, Arc::new(err.into())))?;

                // println!("text     {:?}", cleaned_html.text);

                // let data = format!("{}\n{}", cleaned_html.title, cleaned_html.text);

                let ingested_tokens = IngestedTokens {
                    data: Some(vec![content]),
                    file: file.clone(),
                    doc_source: doc_source.clone(),
                    is_token_stream: Some(false),
                };

                yield Ok(ingested_tokens);
            }
        };

        let processed_stream =
            process_ingested_tokens_stream(Box::pin(stream), self.processors.clone()).await;
        Ok(Box::pin(processed_stream))
    }
}


// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::path::Path;
// 	use futures::StreamExt;

//     #[tokio::test]
//     async fn test_html_ingestor() {

//         let bytes = std::fs::read("/home/ansh/pyg-trail/english_terminology.html").unwrap();

//         // Create a CollectedBytes instance
//         let collected_bytes = CollectedBytes {
//             data: Some(bytes),
//             file: Some(Path::new("english_terminology.html").to_path_buf()),
//             doc_source: Some("test_source".to_string()),
// 			eof: false,
// 			extension: Some("html".to_string()),
// 			size: Some(10),
//         };

//         // Create a TxtIngestor instance
//         let ingestor = HtmlIngestor::new();

//         // Ingest the file
//         let result_stream = ingestor.ingest(vec![collected_bytes]).await.unwrap();

//         // Collect the stream into a Vec
// 		let mut count = 0;
// 		let mut stream = result_stream;
//         while let Some(tokens) = stream.next().await {
// 			let tokens = tokens.unwrap();
// 			println!("These are the tokens in file --------------{:?}", tokens);
// 			count += 1;
// 		}
// 		assert_eq!(count, 1);
// 	}
// }