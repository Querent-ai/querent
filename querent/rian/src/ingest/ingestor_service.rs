use std::sync::Arc;

use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::{CollectionBatch, IngestorCounters, RuntimeType};
use futures::StreamExt;
use ingestors::resolve_ingestor_with_extension;
use proto::semantics::IngestedTokens;
use tokio::{runtime::Handle, sync::mpsc::Sender};
use tracing::{debug, error, info};

pub struct IngestorService {
	pub collector_id: String,
	pub timestamp: u64,
	pub counters: Arc<IngestorCounters>,
	token_sender: Sender<IngestedTokens>,
}

impl IngestorService {
	pub fn new(collector_id: String, token_sender: Sender<IngestedTokens>, timestamp: u64) -> Self {
		Self { collector_id, timestamp, counters: Arc::new(IngestorCounters::new()), token_sender }
	}

	pub fn get_timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn set_timestamp(&mut self, timestamp: u64) {
		self.timestamp = timestamp;
	}

	pub fn get_collector_id(&self) -> String {
		self.collector_id.clone()
	}

	pub fn get_counters(&self) -> Arc<IngestorCounters> {
		self.counters.clone()
	}

	pub fn get_token_sender(&self) -> Sender<IngestedTokens> {
		self.token_sender.clone()
	}
}

#[async_trait]
impl Actor for IngestorService {
	type ObservableState = Arc<IngestorCounters>;

	fn observable_state(&self) -> Self::ObservableState {
		self.counters.clone()
	}

	fn name(&self) -> String {
		"IngestorService".to_string()
	}

	fn queue_capacity(&self) -> QueueCapacity {
		QueueCapacity::Bounded(10)
	}

	fn runtime_handle(&self) -> Handle {
		RuntimeType::Blocking.get_runtime_handle()
	}

	#[inline]
	fn yield_after_each_message(&self) -> bool {
		false
	}

	async fn finalize(
		&mut self,
		exit_status: &ActorExitStatus,
		_ctx: &ActorContext<Self>,
	) -> anyhow::Result<()> {
		match exit_status {
			ActorExitStatus::DownstreamClosed |
			ActorExitStatus::Killed |
			ActorExitStatus::Failure(_) |
			ActorExitStatus::Panicked => Ok(()),
			ActorExitStatus::Quit | ActorExitStatus::Success => {
				info!("IngestorService exiting with success");
				Ok(())
			},
		}
	}
}

#[async_trait]
impl Handler<CollectionBatch> for IngestorService {
	type Reply = ();

	async fn handle(
		&mut self,
		message: CollectionBatch,
		_ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		debug!("Received CollectionBatch: {:?}", message.file);
		let file_ingestor =
			resolve_ingestor_with_extension(&message.clone().ext).await.map_err(|e| {
				ActorExitStatus::Failure(
					anyhow::anyhow!("Failed to resolve ingestor: {}", e).into(),
				)
			})?;

		// Spawn a new task to ingest the file
		let token_sender = self.get_token_sender();
		let counters = self.get_counters();
		let collector_id = self.get_collector_id();
		// Calculate and update total megabytes ingested
		let total_bytes: usize =
			message.clone().bytes.iter().map(|bytes| bytes.size.unwrap_or(0)).sum();

		let total_mbs = (total_bytes + 1023) / 1024 / 1024; // Ceiling division for bytes to MB
		self.counters.increment_total_megabytes(total_mbs as u64);
		self.counters.increment_total_docs(1);

		tokio::spawn(async move {
			let ingested_token_stream = file_ingestor.ingest(message.clone().bytes).await;
			match ingested_token_stream {
				Ok(mut ingested_tokens_stream) => {
					// Send IngestedTokens to the token_sender and trace the errors
					while let Some(ingested_tokens_result) = ingested_tokens_stream.next().await {
						match ingested_tokens_result {
							Ok(ingested_tokens) => {
								if let Err(e) = token_sender.send(ingested_tokens).await {
									error!("Failed to send IngestedTokens to token_sender with error: {}", e);
									break;
								}
								counters.increment_total_ingested_tokens(1);
							},
							Err(e) => {
								error!("Failed to ingest file for collector_id:{} and file extension: {} with error: {}", collector_id, message.ext, e);
							},
						}
					}
				},
				Err(e) => {
					error!("Failed to ingest file for collector_id:{} and file extension: {} with error: {}", collector_id, message.ext, e);
				},
			}
		});

		Ok(())
	}
}
