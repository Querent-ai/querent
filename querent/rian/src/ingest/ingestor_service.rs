// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1). 
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services, 
//    or any service or product offering that provides database, big data, or analytics 
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied, 
// including but not limited to the warranties of merchantability, fitness for a particular purpose, 
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.ai).

use std::sync::Arc;

use actors::{Actor, ActorContext, ActorExitStatus, Handler, QueueCapacity};
use async_trait::async_trait;
use common::{CollectionBatch, IngestorCounters, RuntimeType, TerimateSignal};
use futures::StreamExt;
use ingestors::resolve_ingestor_with_extension;
use proto::semantics::IngestedTokens;
use tokio::{runtime::Handle, sync::mpsc::Sender, task::JoinHandle};
use tracing::{error, info};

use crate::{MAX_DATA_SIZE_IN_MEMORY, NUMBER_FILES_IN_MEMORY};

pub struct IngestorService {
	pub collector_id: String,
	pub timestamp: u64,
	pub counters: Arc<IngestorCounters>,
	token_sender: Sender<IngestedTokens>,
	workflow_handles: Vec<JoinHandle<()>>,
	workflow_semaphore: Arc<tokio::sync::Semaphore>,
	terminate_signal: TerimateSignal,
}

impl IngestorService {
	pub fn new(
		collector_id: String,
		token_sender: Sender<IngestedTokens>,
		timestamp: u64,
		terminate_signal: TerimateSignal,
	) -> Self {
		Self {
			collector_id,
			timestamp,
			counters: Arc::new(IngestorCounters::new()),
			token_sender,
			workflow_handles: Vec::new(),
			workflow_semaphore: Arc::new(tokio::sync::Semaphore::new(NUMBER_FILES_IN_MEMORY)),
			terminate_signal,
		}
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
		QueueCapacity::Bounded(1)
	}

	fn runtime_handle(&self) -> Handle {
		RuntimeType::NonBlocking.get_runtime_handle()
	}

	#[inline]
	fn yield_after_each_message(&self) -> bool {
		true
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
				if !self.workflow_handles.is_empty() {
					for handle in self.workflow_handles.iter() {
						handle.abort();
					}
				}
				Ok(())
			},
		}
	}
}

#[async_trait]
impl Handler<CollectionBatch> for IngestorService {
	type Reply = Result<Option<CollectionBatch>, anyhow::Error>;

	async fn handle(
		&mut self,
		message: CollectionBatch,
		_ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		if MAX_DATA_SIZE_IN_MEMORY.get().is_some() &&
			self.counters.get_current_memory_usage() as usize >
				*MAX_DATA_SIZE_IN_MEMORY.get().unwrap() ||
			message._permit.is_none() ||
			self.workflow_semaphore.available_permits() == 0
		{
			error!("Permit is None or no available permits");
			return Ok(Ok(Some(message)));
		}
		let local_workflow_permit = match self.workflow_semaphore.clone().acquire_owned().await {
			Ok(permit) => permit,
			Err(e) => {
				error!("Failed to acquire semaphore permit: {}", e);
				return Ok(Ok(Some(message)));
			},
		};

		self.workflow_handles.retain(|handle| !handle.is_finished());
		let file_ingestor = resolve_ingestor_with_extension(&message.ext).await.map_err(|e| {
			ActorExitStatus::Failure(anyhow::anyhow!("Failed to resolve ingestor: {}", e).into())
		})?;

		let token_sender = self.get_token_sender();
		if token_sender.is_closed() {
			error!("Token sender is closed");
			return Err(ActorExitStatus::Failure(anyhow::anyhow!("Token sender is closed").into()));
		}
		let counters = self.get_counters();
		let collector_id = self.get_collector_id();

		let total_bytes: usize = message.bytes.iter().map(|bytes| bytes.size.unwrap_or(0)).sum();
		let total_mbs = (total_bytes + 1023) / 1024 / 1024;
		self.counters.increment_total_megabytes(total_mbs as u64);
		self.counters.increment_total_docs(1);
		self.counters.set_current_memory_usage(
			self.counters.get_current_memory_usage() + total_bytes as u64,
		);
		let term_sig = self.terminate_signal.clone();
		let handle = tokio::spawn(async move {
			let ingested_token_stream = file_ingestor.ingest(message.bytes).await;
			match ingested_token_stream {
				Ok(mut ingested_tokens_stream) => {
					if token_sender.is_closed() {
						return;
					}
					let _permit = message._permit.unwrap();
					let _permit_workflow = local_workflow_permit;

					while let Some(ingested_tokens_result) = ingested_tokens_stream.next().await {
						if term_sig.is_dead() {
							break;
						}
						match ingested_tokens_result {
							Ok(ingested_tokens) => {
								if let Err(e) = token_sender.send(ingested_tokens).await {
									error!("Failed to send IngestedTokens to token_sender with error: {}", e);
									return;
								}
								counters.increment_total_ingested_tokens(1);
							},
							Err(e) => {
								error!("Failed to ingest file for collector_id:{} and file extension: {} with error: {}", collector_id, message.ext, e);
							},
						}
					}
					// Drop the permits here to release them
					drop(_permit);
					drop(_permit_workflow);
					counters.set_current_memory_usage(
						counters.get_current_memory_usage() - total_bytes as u64,
					);
				},
				Err(e) => {
					error!("Failed to ingest file for collector_id:{} and file extension: {} with error: {}", collector_id, message.ext, e);
				},
			}
		});
		self.workflow_handles.push(handle);
		_ctx.record_progress();
		Ok(Ok(None))
	}
}
