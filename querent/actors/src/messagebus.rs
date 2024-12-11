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
	any::Any,
	convert::Infallible,
	fmt,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc, Weak,
	},
	time::Instant,
};

use async_trait::async_trait;
use common::metrics::IntCounter;
use tokio::sync::oneshot;

use crate::{
	channel_with_priority::{Receiver, Sender, TrySendError},
	envelope::{wrap_in_envelope, Envelope},
	scheduler::SchedulerClient,
	Actor, ActorContext, ActorExitStatus, AskError, DeferableReplyHandler, Handler, QueueCapacity,
	RecvError, SendError,
};

/// A messagebus is the object that makes it possible to send a message
/// to an actor.
///
/// It is lightweight to clone.
///
/// The actor holds its `Inbox` counterpart.
///
/// The messagebus can receive high priority and low priority messages.
/// Commands are typically sent as high priority messages, whereas regular
/// actor messages are sent to the low priority channel.
///
/// Whenever a high priority message is available, it is processed
/// before low priority messages.
///
/// If all messagebuses are dropped, the actor will process all of the pending messages
/// and gracefully exit with [`crate::actor::ActorExitStatus::Success`].
pub struct MessageBus<A: Actor> {
	inner: Arc<Inner<A>>,
	// We do not rely on the `Arc:strong_count` here to avoid an intricate
	// race condition. We want to make sure the processing of the `Nudge`
	// message happens AFTER we decrement the refcount.
	ref_count: Arc<AtomicUsize>,
}

impl<A: Actor> MessageBus<A> {
	pub fn downgrade(&self) -> WeakMessagebus<A> {
		WeakMessagebus {
			inner: Arc::downgrade(&self.inner),
			ref_count: Arc::downgrade(&self.ref_count),
		}
	}
}

impl<A: Actor> Drop for MessageBus<A> {
	fn drop(&mut self) {
		let old_val = self.ref_count.fetch_sub(1, Ordering::SeqCst);
		if old_val == 2 {
			// This was the last messagebus.
			// `ref_count == 1` means that only the messagebus in the ActorContext
			// is remaining.
			let _ = self.send_message_with_high_priority(LastMessagebus);
		}
	}
}

#[derive(Debug)]
struct LastMessagebus;

#[async_trait]
impl<A: Actor> Handler<LastMessagebus> for A {
	type Reply = ();

	async fn handle(
		&mut self,
		_: LastMessagebus,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		// Being the last messagebus does not necessarily mean that we
		// want to stop the processing.
		//
		// There could be pending message in the queue that will
		// spawn actors which will get a new copy of the messagebus
		// etc.
		//
		// For that reason, the logic that really detects
		// the last messagebus happens when all message have been drained.
		//
		// The `LastMessagebus` message is just here to make sure the actor
		// loop does not get stuck waiting for a message that does
		// will never come.
		Ok(())
	}
}

#[derive(Copy, Clone)]
pub(crate) enum Priority {
	High,
	Low,
}

impl<A: Actor> Clone for MessageBus<A> {
	fn clone(&self) -> Self {
		self.ref_count.fetch_add(1, Ordering::SeqCst);
		MessageBus { inner: self.inner.clone(), ref_count: self.ref_count.clone() }
	}
}

impl<A: Actor> MessageBus<A> {
	pub(crate) fn is_last_messagebus(&self) -> bool {
		self.ref_count.load(Ordering::SeqCst) == 1
	}

	pub fn id(&self) -> &str {
		&self.inner.instance_id
	}

	pub(crate) fn scheduler_client(&self) -> Option<&SchedulerClient> {
		self.inner.scheduler_client_opt.as_ref()
	}
}

struct Inner<A: Actor> {
	pub(crate) tx: Sender<Envelope<A>>,
	scheduler_client_opt: Option<SchedulerClient>,
	instance_id: String,
}

impl<A: Actor> fmt::Debug for MessageBus<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_tuple("MessageBus").field(&self.actor_instance_id()).finish()
	}
}

impl<A: Actor> MessageBus<A> {
	pub fn actor_instance_id(&self) -> &str {
		&self.inner.instance_id
	}

	pub fn is_disconnected(&self) -> bool {
		self.inner.tx.is_disconnected()
	}

	/// Sends a message to the actor owning the associated inbox.
	///
	/// From an actor context, use the `ActorContext::send_message` method instead.
	///
	/// SendError is returned if the actor has already exited.
	pub async fn send_message<M>(
		&self,
		message: M,
	) -> Result<oneshot::Receiver<A::Reply>, SendError>
	where
		A: DeferableReplyHandler<M>,
		M: fmt::Debug + Send + 'static,
	{
		self.send_message_with_backpressure_counter(message, None).await
	}

	/// Attempts to queue a message in the low priority channel of the messagebus.
	///
	/// If sending the message would block, the method simply returns `TrySendError::Full(message)`.
	pub fn try_send_message<M>(
		&self,
		message: M,
	) -> Result<oneshot::Receiver<A::Reply>, TrySendError<M>>
	where
		A: DeferableReplyHandler<M>,
		M: fmt::Debug + Send + 'static,
	{
		let (envelope, response_rx) = self.wrap_in_envelope(message);
		self.inner.tx.try_send_low_priority(envelope).map_err(|err| {
			match err {
				TrySendError::Disconnected => TrySendError::Disconnected,
				TrySendError::Full(mut envelope) => {
					// We need to un pack the envelope.
					let message: M = envelope.message_typed().unwrap();
					TrySendError::Full(message)
				},
			}
		})?;
		Ok(response_rx)
	}

	fn wrap_in_envelope<M>(&self, message: M) -> (Envelope<A>, oneshot::Receiver<A::Reply>)
	where
		A: DeferableReplyHandler<M>,
		M: fmt::Debug + Send + 'static,
	{
		let guard = self
			.inner
			.scheduler_client_opt
			.as_ref()
			.map(|scheduler_client| scheduler_client.no_advance_time_guard());
		wrap_in_envelope(message, guard)
	}

	/// Sends a message to the actor owning the associated inbox.
	///
	/// If the actor experiences some backpressure, then
	/// `backpressure_micros` will be increased by the amount of
	/// microseconds of backpressure experienced.
	pub async fn send_message_with_backpressure_counter<M>(
		&self,
		message: M,
		backpressure_micros_counter_opt: Option<&IntCounter>,
	) -> Result<oneshot::Receiver<A::Reply>, SendError>
	where
		A: DeferableReplyHandler<M>,
		M: fmt::Debug + Send + 'static,
	{
		let (envelope, response_rx) = self.wrap_in_envelope(message);
		match self.inner.tx.try_send_low_priority(envelope) {
			Ok(()) => Ok(response_rx),
			Err(TrySendError::Full(envelope)) => {
				if let Some(backpressure_micros_counter) = backpressure_micros_counter_opt {
					let now = Instant::now();
					self.inner.tx.send_low_priority(envelope).await?;
					let elapsed = now.elapsed();
					backpressure_micros_counter.inc_by(elapsed.as_micros() as u64);
				} else {
					self.inner.tx.send_low_priority(envelope).await?;
				}
				Ok(response_rx)
			},
			Err(TrySendError::Disconnected) => Err(SendError::Disconnected),
		}
	}

	pub fn send_message_with_high_priority<M>(
		&self,
		message: M,
	) -> Result<oneshot::Receiver<A::Reply>, SendError>
	where
		A: DeferableReplyHandler<M>,
		M: fmt::Debug + Send + 'static,
	{
		let (envelope, response_rx) = self.wrap_in_envelope(message);
		self.inner.tx.send_high_priority(envelope)?;
		Ok(response_rx)
	}

	pub(crate) async fn send_message_with_priority<M>(
		&self,
		message: M,
		priority: Priority,
	) -> Result<oneshot::Receiver<A::Reply>, SendError>
	where
		A: DeferableReplyHandler<M>,
		M: fmt::Debug + Send + 'static,
	{
		let (envelope, response_rx) = self.wrap_in_envelope(message);
		match priority {
			Priority::High => self.inner.tx.send_high_priority(envelope)?,
			Priority::Low => {
				self.inner.tx.send_low_priority(envelope).await?;
			},
		}
		Ok(response_rx)
	}

	/// Similar to `send_message`, except this method
	/// waits asynchronously for the actor reply.
	///
	/// From an actor context, use the `ActorContext::ask` method instead.
	pub async fn ask<M, T>(&self, message: M) -> Result<T, AskError<Infallible>>
	where
		A: DeferableReplyHandler<M, Reply = T>,
		M: fmt::Debug + Send + 'static,
	{
		self.ask_with_backpressure_counter(message, None).await
	}

	/// Similar to `ask`, but if a backpressure counter is passed,
	/// it increments the amount of time spent in the backpressure.
	///
	/// The backpressure duration only includes the amount of time
	/// it took to `queue` the request into the actor pipeline.
	///
	/// It does not include
	/// - the amount spent waiting in the queue,
	/// - the amount spent processing the message.
	///
	/// From an actor context, use the `ActorContext::ask` method instead.
	pub async fn ask_with_backpressure_counter<M, T>(
		&self,
		message: M,
		backpressure_micros_counter_opt: Option<&IntCounter>,
	) -> Result<T, AskError<Infallible>>
	where
		A: DeferableReplyHandler<M, Reply = T>,
		M: fmt::Debug + Send + 'static,
	{
		let resp = self
			.send_message_with_backpressure_counter(message, backpressure_micros_counter_opt)
			.await;
		resp.map_err(|_send_error| AskError::MessageNotDelivered)?
			.await
			.map_err(|_| AskError::ProcessMessageError)
	}

	/// Similar to `send_message`, except this method
	/// waits asynchronously for the actor reply.
	///
	/// From an actor context, use the `ActorContext::ask` method instead.
	pub async fn ask_for_res<M, T, E>(&self, message: M) -> Result<T, AskError<E>>
	where
		A: DeferableReplyHandler<M, Reply = Result<T, E>>,
		M: fmt::Debug + Send + 'static,
		E: fmt::Debug,
	{
		self.send_message(message)
			.await
			.map_err(|_send_error| AskError::MessageNotDelivered)?
			.await
			.map_err(|_| AskError::ProcessMessageError)?
			.map_err(AskError::from)
	}
}

pub struct Inbox<A: Actor> {
	rx: Arc<Receiver<Envelope<A>>>,
}

impl<A: Actor> Clone for Inbox<A> {
	fn clone(&self) -> Self {
		Inbox { rx: self.rx.clone() }
	}
}

impl<A: Actor> Inbox<A> {
	pub(crate) fn is_empty(&self) -> bool {
		self.rx.is_empty()
	}

	pub(crate) async fn recv(&self) -> Result<Envelope<A>, RecvError> {
		self.rx.recv().await
	}

	pub(crate) async fn recv_cmd_and_scheduled_msg_only(&self) -> Envelope<A> {
		self.rx.recv_high_priority().await
	}

	pub(crate) fn try_recv(&self) -> Result<Envelope<A>, RecvError> {
		self.rx.try_recv()
	}

	pub async fn recv_typed_message<M: 'static>(&self) -> Option<M> {
		while let Ok(mut envelope) = self.rx.recv().await {
			if let Some(msg) = envelope.message_typed() {
				return Some(msg);
			}
		}
		None
	}

	/// Destroys the inbox and returns the list of pending messages or commands
	/// in the low priority channel.
	///
	/// Warning this iterator might never be exhausted if there is a living
	/// messagebus associated to it.
	pub fn drain_for_test(&self) -> Vec<Box<dyn Any>> {
		self.rx
			.drain_low_priority()
			.into_iter()
			.map(|mut envelope| envelope.message())
			.collect()
	}

	/// Destroys the inbox and returns the list of pending messages or commands
	/// in the low priority channel.
	///
	/// Warning this iterator might never be exhausted if there is a living
	/// messagebus associated to it.
	pub fn drain_for_test_typed<M: 'static>(&self) -> Vec<M> {
		self.rx
			.drain_low_priority()
			.into_iter()
			.flat_map(|mut envelope| envelope.message_typed())
			.collect()
	}
}

pub(crate) fn create_messagebus<A: Actor>(
	actor_name: String,
	queue_capacity: QueueCapacity,
	scheduler_client_opt: Option<SchedulerClient>,
) -> (MessageBus<A>, Inbox<A>) {
	let (tx, rx) = crate::channel_with_priority::channel(queue_capacity);
	let ref_count = Arc::new(AtomicUsize::new(1));
	let messagebus = MessageBus {
		inner: Arc::new(Inner {
			tx,
			instance_id: common::new_quid(&actor_name),
			scheduler_client_opt,
		}),
		ref_count,
	};
	let inbox = Inbox { rx: Arc::new(rx) };
	(messagebus, inbox)
}

pub struct WeakMessagebus<A: Actor> {
	inner: Weak<Inner<A>>,
	ref_count: Weak<AtomicUsize>,
}

impl<A: Actor> Clone for WeakMessagebus<A> {
	fn clone(&self) -> Self {
		Self { inner: self.inner.clone(), ref_count: self.ref_count.clone() }
	}
}

impl<A: Actor> WeakMessagebus<A> {
	pub fn upgrade(&self) -> Option<MessageBus<A>> {
		let inner = self.inner.upgrade()?;
		let ref_count = self.ref_count.upgrade()?;
		Some(MessageBus { inner, ref_count })
	}
}

#[cfg(test)]
mod tests {
	use std::{mem, time::Duration};

	use super::*;
	use crate::{
		tests::{Ping, PingReceiverActor},
		Querent,
	};

	#[tokio::test]
	async fn test_weak_messagebus_downgrade_upgrade() {
		let querent = Querent::with_accelerated_time();
		let (messagebus, _inbox) = querent.create_test_messagebus::<PingReceiverActor>();
		let weak_messagebus = messagebus.downgrade();
		assert!(weak_messagebus.upgrade().is_some());
	}

	#[tokio::test]
	async fn test_weak_messagebus_failing_upgrade() {
		let querent = Querent::with_accelerated_time();
		let (messagebus, _inbox) = querent.create_test_messagebus::<PingReceiverActor>();
		let weak_messagebus = messagebus.downgrade();
		drop(messagebus);
		assert!(weak_messagebus.upgrade().is_none());
	}

	struct BackPressureActor;

	impl Actor for BackPressureActor {
		type ObservableState = ();

		fn observable_state(&self) -> Self::ObservableState {}

		fn queue_capacity(&self) -> QueueCapacity {
			QueueCapacity::Bounded(0)
		}

		fn yield_after_each_message(&self) -> bool {
			false
		}
	}

	use async_trait::async_trait;

	#[async_trait]
	impl Handler<Duration> for BackPressureActor {
		type Reply = ();

		async fn handle(
			&mut self,
			sleep_duration: Duration,
			_ctx: &ActorContext<Self>,
		) -> Result<(), ActorExitStatus> {
			if !sleep_duration.is_zero() {
				tokio::time::sleep(sleep_duration).await;
			}
			Ok(())
		}
	}

	#[tokio::test]
	async fn test_messagebus_send_with_backpressure_counter_low_backpressure() {
		let querent = Querent::with_accelerated_time();
		let back_pressure_actor = BackPressureActor;
		let (messagebus, _handle) = querent.spawn_builder().spawn(back_pressure_actor);
		// We send a first message to make sure the actor has been properly spawned and is listening
		// for new messages.
		messagebus
			.ask_with_backpressure_counter(Duration::default(), None)
			.await
			.unwrap();
		// At this point the actor was started and even processed a message entirely.
		let backpressure_micros_counter =
			IntCounter::new("test_counter", "help for test_counter").unwrap();
		let wait_duration = Duration::from_millis(1);
		let processed = messagebus
			.send_message_with_backpressure_counter(
				wait_duration,
				Some(&backpressure_micros_counter),
			)
			.await
			.unwrap();
		assert!(backpressure_micros_counter.get() < 500);
		processed.await.unwrap();
		assert!(backpressure_micros_counter.get() < 500);
		querent.assert_quit().await;
	}

	#[tokio::test]
	async fn test_messagebus_send_with_backpressure_counter_backpressure() {
		let querent = Querent::with_accelerated_time();
		let back_pressure_actor = BackPressureActor;
		let (messagebus, _handle) = querent.spawn_builder().spawn(back_pressure_actor);
		// We send a first message to make sure the actor has been properly spawned and is listening
		// for new messages.
		messagebus
			.ask_with_backpressure_counter(Duration::default(), None)
			.await
			.unwrap();
		let backpressure_micros_counter =
			IntCounter::new("test_counter", "help for test_counter").unwrap();
		let wait_duration = Duration::from_millis(1);
		messagebus
			.send_message_with_backpressure_counter(
				wait_duration,
				Some(&backpressure_micros_counter),
			)
			.await
			.unwrap();
		// That second message will present some backpressure, since the capacity is 0 and
		// the first message will take 1000 micros to be processed.
		messagebus
			.send_message_with_backpressure_counter(
				Duration::default(),
				Some(&backpressure_micros_counter),
			)
			.await
			.unwrap();
		assert!(backpressure_micros_counter.get() > 1_000u64);
		querent.assert_quit().await;
	}

	#[tokio::test]
	async fn test_messagebus_waiting_for_processing_does_not_counter_as_backpressure() {
		let querent = Querent::with_accelerated_time();
		let back_pressure_actor = BackPressureActor;
		let (messagebus, _handle) = querent.spawn_builder().spawn(back_pressure_actor);
		messagebus
			.ask_with_backpressure_counter(Duration::default(), None)
			.await
			.unwrap();
		let backpressure_micros_counter =
			IntCounter::new("test_counter", "help for test_counter").unwrap();
		let start = Instant::now();
		messagebus
			.ask_with_backpressure_counter(Duration::from_millis(1), None)
			.await
			.unwrap();
		let elapsed = start.elapsed();
		assert!(elapsed.as_micros() > 1000);
		assert_eq!(backpressure_micros_counter.get(), 0);
		querent.assert_quit().await;
	}

	#[tokio::test]
	async fn test_try_send() {
		let querent = Querent::with_accelerated_time();
		let (messagebus, _inbox) = querent
			.create_messagebus::<PingReceiverActor>("hello".to_string(), QueueCapacity::Bounded(1));
		assert!(messagebus.try_send_message(Ping).is_ok());
		assert!(matches!(messagebus.try_send_message(Ping).unwrap_err(), TrySendError::Full(Ping)));
	}

	#[tokio::test]
	async fn test_try_send_disconnect() {
		let querent = Querent::with_accelerated_time();
		let (messagebus, inbox) = querent
			.create_messagebus::<PingReceiverActor>("hello".to_string(), QueueCapacity::Bounded(1));
		assert!(messagebus.try_send_message(Ping).is_ok());
		mem::drop(inbox);
		assert!(matches!(
			messagebus.try_send_message(Ping).unwrap_err(),
			TrySendError::Disconnected
		));
	}
}
