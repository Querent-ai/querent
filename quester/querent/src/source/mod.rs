use std::time::Duration;

use actors::{Actor, ActorContext, ActorExitStatus, Handler};
use async_trait::async_trait;
use common::RuntimeType;
use serde_json::Value as JsonValue;
use tokio::runtime::Handle;

pub type SourceContext = ActorContext<SourceActor>;

#[async_trait]
pub trait Source: Send + 'static {
    /// This method will be called before any calls to `emit_events`.
    async fn initialize(&mut self, _ctx: &SourceContext) -> Result<(), ActorExitStatus> {
        Ok(())
    }

    /// Main part of the source implementation, `emit_events` can emit 0..n batches.
    ///
    /// The `batch_sink` is a mailbox that has a bounded capacity.
    /// In that case, `batch_sink` will block.
    ///
    /// It returns an optional duration specifying how long the batch requester
    /// should wait before pooling gain.
    async fn emit_events(&mut self, ctx: &SourceContext) -> Result<Duration, ActorExitStatus>;

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
        RuntimeType::NonBlocking.get_runtime_handle()
    }

    fn yield_after_each_message(&self) -> bool {
        false
    }

    async fn initialize(&mut self, ctx: &SourceContext) -> Result<(), ActorExitStatus> {
        self.source.initialize(ctx).await?;
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
        let wait_for = self.source.emit_events(ctx).await?;
        if wait_for.is_zero() {
            ctx.send_self_message(Loop).await?;
            return Ok(());
        }
        ctx.schedule_self_msg(wait_for, Loop).await;
        Ok(())
    }
}
