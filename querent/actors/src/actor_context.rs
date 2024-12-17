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

use std::{
	convert::Infallible,
	fmt,
	future::Future,
	ops::Deref,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	time::Duration,
};

use common::{metrics::IntCounter, Progress, ProtectedZoneGuard, TerimateSignal};
use tokio::sync::{oneshot, watch};
use tracing::{debug, error};

#[cfg(any(test, feature = "testsuite"))]
use crate::Querent;
use crate::{
	actor_state::AtomicState,
	registry::ActorRegistry,
	spawn_builder::{SpawnBuilder, SpawnContext},
	Actor, ActorExitStatus, ActorState, AskError, Command, DeferableReplyHandler, MessageBus,
	SendError, TrySendError,
};

// TODO hide all of this public stuff
pub struct ActorContext<A: Actor> {
	inner: Arc<ActorContextInner<A>>,
}

impl<A: Actor> Clone for ActorContext<A> {
	fn clone(&self) -> Self {
		ActorContext { inner: self.inner.clone() }
	}
}

impl<A: Actor> Deref for ActorContext<A> {
	type Target = ActorContextInner<A>;

	fn deref(&self) -> &Self::Target {
		self.inner.as_ref()
	}
}

pub struct ActorContextInner<A: Actor> {
	spawn_ctx: SpawnContext,
	self_messagebus: MessageBus<A>,
	progress: Progress,
	actor_state: AtomicState,
	backpressure_micros_counter_opt: Option<IntCounter>,
	observable_state_tx: watch::Sender<A::ObservableState>,
	// Boolean marking the presence of an observe message in the actor's high priority queue.
	observe_enqueued: AtomicBool,
}

impl<A: Actor> ActorContext<A> {
	pub(crate) fn new(
		self_messagebus: MessageBus<A>,
		spawn_ctx: SpawnContext,
		observable_state_tx: watch::Sender<A::ObservableState>,
		backpressure_micros_counter_opt: Option<IntCounter>,
	) -> Self {
		ActorContext {
			inner: ActorContextInner {
				self_messagebus,
				spawn_ctx,
				progress: Progress::default(),
				actor_state: AtomicState::default(),
				observable_state_tx,
				backpressure_micros_counter_opt,
				observe_enqueued: AtomicBool::new(false),
			}
			.into(),
		}
	}

	pub fn spawn_ctx(&self) -> &SpawnContext {
		&self.spawn_ctx
	}

	/// Sleeps for a given amount of time.
	///
	/// That sleep is measured by the querent scheduler, which means that it can be
	/// shortened if `Querent::simulate_sleep(..)` is used.
	///
	/// While sleeping, an actor is NOT protected from its supervisor.
	/// It is up to the user to call `ActorContext::protect_future(..)`.
	pub async fn sleep(&self, duration: Duration) {
		let scheduler_client = &self.spawn_ctx().scheduler_client;
		scheduler_client.dec_no_advance_time();
		scheduler_client.sleep(duration).await;
		scheduler_client.inc_no_advance_time();
	}

	#[cfg(any(test, feature = "testsuite"))]
	pub fn for_test(
		querent: &Querent,
		actor_messagebus: MessageBus<A>,
		observable_state_tx: watch::Sender<A::ObservableState>,
	) -> Self {
		Self::new(actor_messagebus, querent.spawn_ctx.clone(), observable_state_tx, None)
	}

	pub fn messagebus(&self) -> &MessageBus<A> {
		&self.self_messagebus
	}

	pub(crate) fn registry(&self) -> &ActorRegistry {
		&self.spawn_ctx.registry
	}

	pub fn actor_instance_id(&self) -> &str {
		self.messagebus().actor_instance_id()
	}

	/// This function returns a guard that prevents any supervisor from identifying the
	/// actor as dead.
	/// The protection ends when the `ProtectZoneGuard` is dropped.
	///
	/// In an ideal world, you should never need to call this function.
	/// It is only useful in some corner cases, like calling a long blocking
	/// from an external library that you trust.
	pub fn protect_zone(&self) -> ProtectedZoneGuard {
		self.progress.protect_zone()
	}

	/// Executes a future in a protected zone.
	pub async fn protect_future<Fut, T>(&self, future: Fut) -> T
	where
		Fut: Future<Output = T>,
	{
		let _guard = self.protect_zone();
		future.await
	}

	/// Cooperatively yields, while keeping the actor protected.
	pub async fn yield_now(&self) {
		self.protect_future(tokio::task::yield_now()).await;
	}

	/// Gets a copy of the actor kill switch.
	/// This should rarely be used.
	///
	/// For instance, when quitting from the process_message function, prefer simply
	/// returning `Error(ActorExitStatus::Failure(..))`
	pub fn terminate_sig(&self) -> &TerimateSignal {
		&self.spawn_ctx.terminate_sig
	}

	#[must_use]
	pub fn progress(&self) -> &Progress {
		&self.progress
	}

	pub fn spawn_actor<SpawnedActor: Actor>(&self) -> SpawnBuilder<SpawnedActor> {
		self.spawn_ctx.clone().spawn_builder()
	}

	/// Records some progress.
	/// This function is only useful when implementing actors that may take more than
	/// `HEARTBEAT` to process a single message.
	/// In that case, you can call this function in the middle of the process_message method
	/// to prevent the actor from being identified as blocked or dead.
	pub fn record_progress(&self) {
		self.progress.record_progress();
	}

	pub(crate) fn state(&self) -> ActorState {
		self.actor_state.get_state()
	}

	pub(crate) fn pause(&self) {
		self.actor_state.pause();
	}

	pub(crate) fn resume(&self) {
		self.actor_state.resume();
	}

	/// Sets the queue as observed and returns the previous value.
	/// This method is used to make sure we do not have Observe messages
	/// stacking up in the observe queue.
	pub(crate) fn set_observe_enqueued_and_return_previous(&self) -> bool {
		self.observe_enqueued.swap(true, Ordering::Relaxed)
	}

	/// Updates the observable state of the actor.
	pub fn observe(&self, actor: &mut A) -> A::ObservableState {
		let obs_state = actor.observable_state();
		self.inner.observe_enqueued.store(false, Ordering::Relaxed);
		let _ = self.observable_state_tx.send(obs_state.clone());
		obs_state
	}

	pub(crate) fn exit(&self, exit_status: &ActorExitStatus) {
		self.actor_state.exit(exit_status.is_success());
		if should_activate_terminate_sig(exit_status) {
			error!(actor=%self.actor_instance_id(), exit_status=?exit_status, "exit activating-kill-switch");
			self.terminate_sig().kill();
		}
	}

	/// Posts a message in an actor's messagebus.
	///
	/// This method does not wait for the message to be handled by the
	/// target actor. However, it returns a oneshot receiver that the caller
	/// that makes it possible to `.await` it.
	/// If the reply is important, chances are the `.ask(...)` method is
	/// more indicated.
	///
	/// Droppping the receiver channel will not cancel the
	/// processing of the message. It is a very common usage.
	/// In fact most actors are expected to send message in a
	/// fire-and-forget fashion.
	///
	/// Regular messages (as opposed to commands) are queued and guaranteed
	/// to be processed in FIFO order.
	///
	/// This method hides logic to prevent an actor from being identified
	/// as frozen if the destination actor channel is saturated, and we
	/// are simply experiencing back pressure.
	pub async fn send_message<DestActor: Actor, M>(
		&self,
		messagebus: &MessageBus<DestActor>,
		msg: M,
	) -> Result<oneshot::Receiver<DestActor::Reply>, SendError>
	where
		DestActor: DeferableReplyHandler<M>,
		M: fmt::Debug + Send + 'static,
	{
		let _guard = self.protect_zone();
		debug!(from=%self.self_messagebus.actor_instance_id(), send=%messagebus.actor_instance_id(), msg=?msg);
		messagebus
			.send_message_with_backpressure_counter(
				msg,
				self.backpressure_micros_counter_opt.as_ref(),
			)
			.await
	}

	pub async fn ask<DestActor: Actor, M, T>(
		&self,
		messagebus: &MessageBus<DestActor>,
		msg: M,
	) -> Result<T, AskError<Infallible>>
	where
		DestActor: DeferableReplyHandler<M, Reply = T>,
		M: fmt::Debug + Send + 'static,
	{
		let _guard = self.protect_zone();
		debug!(from=%self.self_messagebus.actor_instance_id(), send=%messagebus.actor_instance_id(), msg=?msg, "ask");
		messagebus
			.ask_with_backpressure_counter(msg, self.backpressure_micros_counter_opt.as_ref())
			.await
	}

	/// Similar to `send_message`, except this method
	/// waits asynchronously for the actor reply.
	pub async fn ask_for_res<DestActor: Actor, M, T, E>(
		&self,
		messagebus: &MessageBus<DestActor>,
		msg: M,
	) -> Result<T, AskError<E>>
	where
		DestActor: DeferableReplyHandler<M, Reply = Result<T, E>>,
		M: fmt::Debug + Send + Sync + 'static,
		E: fmt::Debug,
	{
		let _guard = self.protect_zone();
		debug!(from=%self.self_messagebus.actor_instance_id(), send=%messagebus.actor_instance_id(), msg=?msg, "ask");
		messagebus.ask_for_res(msg).await
	}

	/// Send the Success message to terminate the destination actor with the Success exit status.
	///
	/// The message is queued like any regular message, so that pending messages will be processed
	/// first.
	pub async fn send_exit_with_success<Dest: Actor>(
		&self,
		messagebus: &MessageBus<Dest>,
	) -> Result<(), SendError> {
		let _guard = self.protect_zone();
		debug!(from=%self.self_messagebus.actor_instance_id(), to=%messagebus.actor_instance_id(), "success");
		messagebus.send_message(Command::ExitWithSuccess).await?;
		Ok(())
	}

	/// Sends a message to an actor's own messagebus.
	///
	/// Warning: This method is dangerous as it can very easily
	/// cause a deadlock.
	pub async fn send_self_message<M>(
		&self,
		msg: M,
	) -> Result<oneshot::Receiver<A::Reply>, SendError>
	where
		A: DeferableReplyHandler<M>,
		M: 'static + Sync + Send + fmt::Debug,
	{
		debug!(self=%self.self_messagebus.actor_instance_id(), msg=?msg, "self_send");
		self.self_messagebus.send_message(msg).await
	}

	/// Attempts to send a message to itself.
	/// The message will be queue to self's low_priority queue.
	///
	/// Warning: This method will always fail if
	/// an actor has a capacity of 0.
	pub fn try_send_self_message<M>(
		&self,
		msg: M,
	) -> Result<oneshot::Receiver<A::Reply>, TrySendError<M>>
	where
		A: DeferableReplyHandler<M>,
		M: 'static + Sync + Send + fmt::Debug,
	{
		self.self_messagebus.try_send_message(msg)
	}

	/// Schedules a message that will be sent to the high-priority
	/// queue of the actor MessageBus once `after_duration` has elapsed.
	pub fn schedule_self_msg<M>(&self, after_duration: Duration, message: M)
	where
		A: DeferableReplyHandler<M>,
		M: Sync + Send + std::fmt::Debug + 'static,
	{
		let self_messagebus = self.inner.self_messagebus.clone();
		let callback = move || {
			let _ = self_messagebus.send_message_with_high_priority(message);
		};
		self.inner.spawn_ctx.scheduler_client.schedule_event(callback, after_duration);
	}
}

/// If an actor exits in an unexpected manner, its kill
/// switch will be activated, and all other actors under the same
/// kill switch will be killed.
fn should_activate_terminate_sig(exit_status: &ActorExitStatus) -> bool {
	match exit_status {
		ActorExitStatus::DownstreamClosed => true,
		ActorExitStatus::Failure(_) => true,
		ActorExitStatus::Panicked => true,
		ActorExitStatus::Success => false,
		ActorExitStatus::Quit => false,
		ActorExitStatus::Killed => false,
	}
}
