use crate::{
	InsightError, InsightErrorKind, InsightInput, InsightOutput, InsightResult, InsightRunner,
};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

pub struct CrossDocumentSummarizationRunner {}

#[async_trait]
impl InsightRunner for CrossDocumentSummarizationRunner {
	async fn run(&self, _input: InsightInput) -> InsightResult<InsightOutput> {
		Err(InsightError::new(
			InsightErrorKind::NotFound,
			anyhow::anyhow!("Not Implemented!").into(),
		))
	}

	async fn run_stream<'life0>(
		&'life0 self,
		_input: Pin<Box<dyn Stream<Item = InsightInput> + Send + 'life0>>,
	) -> InsightResult<Pin<Box<dyn Stream<Item = InsightResult<InsightOutput>> + Send + 'life0>>> {
		Err(InsightError::new(
			InsightErrorKind::NotFound,
			anyhow::anyhow!("Not Implemented!").into(),
		))
	}
}
