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

use std::time::Duration;

use anyhow::Context;
use common::metrics::IntCounter;
use sync_wrapper::SyncWrapper;
use tokio::sync::watch;
use tracing::{debug, error, info};

use crate::{
	envelope::Envelope,
	messagebus::{create_messagebus, Inbox},
	registry::{ActorJoinHandle, ActorRegistry},
	scheduler::{NoAdvanceTimeGuard, SchedulerClient},
	supervisor::Supervisor,
	Actor, ActorContext, ActorExitStatus, ActorHandle, MessageBus, QueueCapacity, TerimateSignal,
};

#[derive(Clone)]
pub struct SpawnContext {
	pub(crate) scheduler_client: SchedulerClient,
	pub(crate) terminate_sig: TerimateSignal,
	pub(crate) registry: ActorRegistry,
}

impl SpawnContext {
	pub fn new(scheduler_client: SchedulerClient) -> Self {
		SpawnContext {
			scheduler_client,
			terminate_sig: Default::default(),
			registry: ActorRegistry::default(),
		}
	}

	pub fn spawn_builder<A: Actor>(&self) -> SpawnBuilder<A> {
		SpawnBuilder::new(self.child_context())
	}

	pub fn create_messagebus<A: Actor>(
		&self,
		actor_name: impl ToString,
		queue_capacity: QueueCapacity,
	) -> (MessageBus<A>, Inbox<A>) {
		create_messagebus(
			actor_name.to_string(),
			queue_capacity,
			Some(self.scheduler_client.clone()),
		)
	}

	pub fn child_context(&self) -> SpawnContext {
		SpawnContext {
			scheduler_client: self.scheduler_client.clone(),
			terminate_sig: self.terminate_sig.child(),
			registry: self.registry.clone(),
		}
	}

	/// Schedules a new event.
	/// Once `timeout` is elapsed, the future `fut` is
	/// executed.
	///
	/// `fut` will be executed in the scheduler task, so it is
	/// required to be short.
	pub fn schedule_event<F: FnOnce() + Send + Sync + 'static>(
		&self,
		callback: F,
		timeout: Duration,
	) {
		self.scheduler_client.schedule_event(callback, timeout)
	}
}

/// `SpawnBuilder` makes it possible to configure misc parameters before spawning an actor.
#[derive(Clone)]
pub struct SpawnBuilder<A: Actor> {
	spawn_ctx: SpawnContext,
	#[allow(clippy::type_complexity)]
	messagebuses: Option<(MessageBus<A>, Inbox<A>)>,
	backpressure_micros_counter_opt: Option<IntCounter>,
}

impl<A: Actor> SpawnBuilder<A> {
	pub(crate) fn new(spawn_ctx: SpawnContext) -> Self {
		SpawnBuilder { spawn_ctx, messagebuses: None, backpressure_micros_counter_opt: None }
	}

	/// Sets a specific kill switch for the actor.
	///
	/// By default, the kill switch is inherited from the context that was used to
	/// spawn the actor.
	pub fn set_terminate_sig(mut self, terminate_sig: TerimateSignal) -> Self {
		self.spawn_ctx.terminate_sig = terminate_sig;
		self
	}

	/// Sets a specific set of messagebus.
	///
	/// By default, a brand new set of messagebuses will be created
	/// when the actor is spawned.
	///
	/// This function makes it possible to create non-DAG networks
	/// of actors.
	pub fn set_messagebuses(mut self, messagebus: MessageBus<A>, inbox: Inbox<A>) -> Self {
		self.messagebuses = Some((messagebus, inbox));
		self
	}

	/// Adds a counter to track the amount of time the actor is
	/// spending in "backpressure".
	///
	/// When using `.ask` the amount of time counted may be misleading.
	/// (See `MessageBus::ask_with_backpressure_counter` for more details)
	pub fn set_backpressure_micros_counter(
		mut self,
		backpressure_micros_counter: IntCounter,
	) -> Self {
		self.backpressure_micros_counter_opt = Some(backpressure_micros_counter);
		self
	}

	fn take_or_create_messagebuses(&mut self, actor: &A) -> (MessageBus<A>, Inbox<A>) {
		if let Some((messagebus, inbox)) = self.messagebuses.take() {
			return (messagebus, inbox);
		}
		let actor_name = actor.name();
		let queue_capacity = actor.queue_capacity();
		self.spawn_ctx.create_messagebus(actor_name, queue_capacity)
	}

	fn create_actor_context_and_inbox(
		mut self,
		actor: &A,
	) -> (ActorContext<A>, Inbox<A>, watch::Receiver<A::ObservableState>) {
		let (messagebus, inbox) = self.take_or_create_messagebuses(actor);
		let obs_state = actor.observable_state();
		let (state_tx, state_rx) = watch::channel(obs_state);
		let ctx = ActorContext::new(
			messagebus,
			self.spawn_ctx.clone(),
			state_tx,
			self.backpressure_micros_counter_opt,
		);
		(ctx, inbox, state_rx)
	}

	/// Spawns an async actor.
	pub fn spawn(self, actor: A) -> (MessageBus<A>, ActorHandle<A>) {
		// We prevent fast forward of the scheduler during  initialization.
		let no_advance_time_guard = self.spawn_ctx.scheduler_client.no_advance_time_guard();
		let runtime_handle = actor.runtime_handle();
		let (ctx, inbox, state_rx) = self.create_actor_context_and_inbox(&actor);
		debug!(actor_id = %ctx.actor_instance_id(), "spawn-actor");
		let messagebus = ctx.messagebus().clone();
		let ctx_clone = ctx.clone();
		let loop_async_actor_future =
			async move { actor_loop(actor, inbox, no_advance_time_guard, ctx).await };
		let join_handle = ActorJoinHandle::new(runtime_handle.spawn(loop_async_actor_future));
		ctx_clone.registry().register(&messagebus, join_handle.clone());
		let actor_handle = ActorHandle::new(state_rx, join_handle, ctx_clone);
		(messagebus, actor_handle)
	}

	pub fn supervise_fn<F: Fn() -> A + Send + 'static>(
		mut self,
		actor_factory: F,
	) -> (MessageBus<A>, ActorHandle<Supervisor<A>>) {
		let actor = actor_factory();
		let actor_name = actor.name();
		let (messagebus, inbox) = self.take_or_create_messagebuses(&actor);
		self.messagebuses = Some((messagebus, inbox.clone()));
		let child_ctx = self.spawn_ctx.child_context();
		let parent_spawn_ctx = std::mem::replace(&mut self.spawn_ctx, child_ctx);
		let (messagebus, actor_handle) = self.spawn(actor);
		let supervisor = Supervisor::new(actor_name, Box::new(actor_factory), inbox, actor_handle);
		let (_supervisor_messagebus, supervisor_handle) =
			parent_spawn_ctx.spawn_builder().spawn(supervisor);
		(messagebus, supervisor_handle)
	}
}

impl<A: Actor + Clone> SpawnBuilder<A> {
	pub fn supervise(self, actor: A) -> (MessageBus<A>, ActorHandle<Supervisor<A>>) {
		self.supervise_fn(move || actor.clone())
	}
}

impl<A: Actor + Default> SpawnBuilder<A> {
	pub fn supervise_default(self) -> (MessageBus<A>, ActorHandle<Supervisor<A>>) {
		self.supervise_fn(Default::default)
	}
}

/// Receives an envelope from either the high priority queue or the low priority queue.
///
/// In the paused state, the actor will only attempt to receive high priority messages.
///
/// If no message is available, this function will yield until a message arrives.
/// If a high priority message is arrives first it is guaranteed to be processed first.
/// This other way around is however not guaranteed.
async fn recv_envelope<A: Actor>(inbox: &mut Inbox<A>, ctx: &ActorContext<A>) -> Envelope<A> {
	if ctx.state().is_running() {
		ctx.protect_future(inbox.recv()).await.expect(
			"Disconnection should be impossible because the ActorContext holds a MessageBus too",
		)
	} else {
		// The actor is paused. We only process command and scheduled message.
		ctx.protect_future(inbox.recv_cmd_and_scheduled_msg_only()).await
	}
}

fn try_recv_envelope<A: Actor>(inbox: &mut Inbox<A>) -> Option<Envelope<A>> {
	inbox.try_recv().ok()
}

struct ActorExecutionEnv<A: Actor> {
	actor: SyncWrapper<A>,
	inbox: Inbox<A>,
	ctx: ActorContext<A>,
}

impl<A: Actor> ActorExecutionEnv<A> {
	async fn initialize(&mut self) -> Result<(), ActorExitStatus> {
		self.actor.get_mut().initialize(&self.ctx).await
	}

	async fn process_messages(&mut self) -> ActorExitStatus {
		loop {
			if let Err(exit_status) = self.process_all_available_messages().await {
				return exit_status;
			}
		}
	}

	async fn process_one_message(
		&mut self,
		mut envelope: Envelope<A>,
	) -> Result<(), ActorExitStatus> {
		self.yield_and_check_if_killed().await?;
		envelope.handle_message(self.actor.get_mut(), &self.ctx).await?;
		Ok(())
	}

	async fn yield_and_check_if_killed(&mut self) -> Result<(), ActorExitStatus> {
		if self.ctx.terminate_sig().is_dead() {
			return Err(ActorExitStatus::Killed);
		}
		if self.actor.get_mut().yield_after_each_message() {
			self.ctx.yield_now().await;
			if self.ctx.terminate_sig().is_dead() {
				return Err(ActorExitStatus::Killed);
			}
		} else {
			self.ctx.record_progress();
		}
		Ok(())
	}

	async fn process_all_available_messages(&mut self) -> Result<(), ActorExitStatus> {
		self.yield_and_check_if_killed().await?;
		let envelope = recv_envelope(&mut self.inbox, &self.ctx).await;
		self.process_one_message(envelope).await?;
		// If the actor is Running (not Paused), we consume all the messages in the mailbox
		// and call `on_drained_message`.
		if self.ctx.state().is_running() {
			loop {
				while let Some(envelope) = try_recv_envelope(&mut self.inbox) {
					self.process_one_message(envelope).await?;
				}
				// We have reached the last message.
				// Let's still yield and see if we have more messages:
				// an upstream actor might have experienced backpressure, and is now waiting for our
				// mailbox to have some room.
				self.ctx.yield_now().await;
				if self.inbox.is_empty() {
					break;
				}
			}
			self.actor.get_mut().on_drained_messages(&self.ctx).await?;
		}
		if self.ctx.messagebus().is_last_messagebus() {
			// No one will be able to send us more messages.
			// We can exit the actor.
			return Err(ActorExitStatus::Success);
		}

		Ok(())
	}

	async fn finalize(&mut self, exit_status: ActorExitStatus) -> ActorExitStatus {
		let _no_advance_time_guard = self
			.ctx
			.messagebus()
			.scheduler_client()
			.map(|scheduler_client| scheduler_client.no_advance_time_guard());
		if let Err(finalize_error) = self
			.actor
			.get_mut()
			.finalize(&exit_status, &self.ctx)
			.await
			.with_context(|| format!("finalization of actor {}", self.actor.get_mut().name()))
		{
			error!(error=?finalize_error, "finalizing failed, set exit status to panicked");
			return ActorExitStatus::Panicked;
		}
		exit_status
	}

	fn process_exit_status(&self, exit_status: &ActorExitStatus) {
		match &exit_status {
			ActorExitStatus::Success |
			ActorExitStatus::Quit |
			ActorExitStatus::DownstreamClosed |
			ActorExitStatus::Killed => {},
			ActorExitStatus::Failure(err) => {
				error!(cause=?err, exit_status=?exit_status, "actor-failure");
			},
			ActorExitStatus::Panicked => {
				error!(exit_status=?exit_status, "actor-failure");
			},
		}
		info!(actor_id = %self.ctx.actor_instance_id(), exit_status = %exit_status, "actor-exit");
		self.ctx.exit(exit_status);
	}
}

impl<A: Actor> Drop for ActorExecutionEnv<A> {
	// We rely on this object internally to fetch a post-mortem state,
	// even in case of a panic.
	fn drop(&mut self) {
		self.ctx.observe(self.actor.get_mut());
	}
}

async fn actor_loop<A: Actor>(
	actor: A,
	inbox: Inbox<A>,
	no_advance_time_guard: NoAdvanceTimeGuard,
	ctx: ActorContext<A>,
) -> ActorExitStatus {
	let mut actor_env = ActorExecutionEnv { actor: SyncWrapper::new(actor), inbox, ctx };

	let initialize_exit_status_res: Result<(), ActorExitStatus> = actor_env.initialize().await;
	drop(no_advance_time_guard);

	let after_process_exit_status = if let Err(initialize_exit_status) = initialize_exit_status_res
	{
		// We do not process messages if initialize yield an error.
		// We still call finalize however!
		initialize_exit_status
	} else {
		actor_env.process_messages().await
	};

	// TODO the no advance time guard for finalize has a race condition. Ideally we would
	// like to have the guard before we drop the last envelope.
	let final_exit_status = actor_env.finalize(after_process_exit_status).await;
	// The last observation is collected on `ActorExecutionEnv::Drop`.
	actor_env.process_exit_status(&final_exit_status);
	final_exit_status
}
