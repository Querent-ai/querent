use std::time::Duration;

use actors::{Actor, ActorContext, ActorExitStatus, Handler, MessageBus};
use async_trait::async_trait;
use common::RuntimeType;
use serde_json::Value as JsonValue;
use tokio::runtime::Handle;

use crate::{ingest::ingestor_service::IngestorService, EventStreamer};

// custom sources
pub mod engine_source;
pub use engine_source::*;
pub mod collector_source;
pub use collector_source::*;

pub type SourceContext = ActorContext<SourceActor>;

pub const NUMBER_FILES_IN_MEMORY: usize = 100;

pub const BATCH_NUM_EVENTS_LIMIT: usize = 10;

pub const EMIT_BATCHES_TIMEOUT: Duration =
	Duration::from_millis(if cfg!(test) { 100 } else { 1_000 });

#[async_trait]
pub trait Source: Send + 'static {
	/// This method will be called before any calls to `emit_events`.
	async fn initialize(
		&mut self,
		_event_streamer_messagebus: &MessageBus<EventStreamer>,
		_ingestor_messagebus: &MessageBus<IngestorService>,
		_ctx: &SourceContext,
	) -> Result<(), ActorExitStatus> {
		Ok(())
	}

	/// Main part of the source implementation, `emit_events` can emit 0..n batches.
	///
	/// The `batch_sink` is a messagebus that has a bounded capacity.
	/// In that case, `batch_sink` will block.
	///
	/// It returns an optional duration specifying how long the batch requester
	/// should wait before pooling gain.
	async fn emit_events(
		&mut self,
		event_streamer_messagebus: &MessageBus<EventStreamer>,
		ingestor_messagebus: &MessageBus<IngestorService>,
		ctx: &SourceContext,
	) -> Result<Duration, ActorExitStatus>;

	/// Finalize is called once after the actor terminates.
	async fn finalize(
		&mut self,
		_exit_status: &ActorExitStatus,
		_ctx: &SourceContext,
	) -> anyhow::Result<()> {
		Ok(())
	}

	/// A name identifying the type of source.
	fn name(&self) -> String;

	/// Returns an observable_state for the actor.
	///
	/// This object is simply a json object, and its content may vary depending on the
	/// source.
	fn observable_state(&self) -> JsonValue;
}

/// The SourceActor acts as a thin wrapper over a source trait object to execute.
///
/// It mostly takes care of running a loop calling `emit_events(...)`.
pub struct SourceActor {
	pub source: Box<dyn Source>,
	pub event_streamer_messagebus: MessageBus<EventStreamer>,
	pub ingestor_messagebus: MessageBus<IngestorService>,
}

#[derive(Debug)]
struct Loop;

#[async_trait]
impl Actor for SourceActor {
	type ObservableState = JsonValue;

	fn name(&self) -> String {
		self.source.name()
	}

	fn observable_state(&self) -> Self::ObservableState {
		self.source.observable_state()
	}

	fn runtime_handle(&self) -> Handle {
		RuntimeType::Blocking.get_runtime_handle()
	}

	fn yield_after_each_message(&self) -> bool {
		false
	}

	async fn initialize(&mut self, ctx: &SourceContext) -> Result<(), ActorExitStatus> {
		self.source
			.initialize(&self.event_streamer_messagebus, &self.ingestor_messagebus, ctx)
			.await?;
		self.handle(Loop, ctx).await?;
		Ok(())
	}

	async fn finalize(
		&mut self,
		exit_status: &ActorExitStatus,
		ctx: &SourceContext,
	) -> anyhow::Result<()> {
		self.source.finalize(exit_status, ctx).await?;
		Ok(())
	}
}

#[async_trait]
impl Handler<Loop> for SourceActor {
	type Reply = ();

	async fn handle(&mut self, _message: Loop, ctx: &SourceContext) -> Result<(), ActorExitStatus> {
		let wait_for = self
			.source
			.emit_events(&self.event_streamer_messagebus, &self.ingestor_messagebus, ctx)
			.await?;
		if wait_for.is_zero() {
			ctx.send_self_message(Loop).await?;
			return Ok(());
		}
		ctx.schedule_self_msg(wait_for, Loop);
		Ok(())
	}
}
