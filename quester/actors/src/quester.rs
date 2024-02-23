use std::{collections::HashMap, thread, time::Duration};

use crate::{
	messagebus::create_messagebus,
	registry::ActorObservation,
	scheduler::start_scheduler,
	spawn_builder::{SpawnBuilder, SpawnContext},
	Actor, ActorExitStatus, Command, Inbox, MessageBus, QueueCapacity,
};

/// Quester serves as the top-level context in which Actor can be spawned.
/// It is *not* a singleton. A typical application will usually have only one quester hosting all
/// of the actors but it is not a requirement.
///
/// In particular, unit test all have their own quester and hence can be executed in parallel.
pub struct Quester {
	pub(crate) spawn_ctx: SpawnContext,
}

impl Default for Quester {
	fn default() -> Quester {
		Quester::new()
	}
}

impl Quester {
	/// Creates a new quester.
	pub fn new() -> Quester {
		let scheduler_client = start_scheduler();
		Quester { spawn_ctx: SpawnContext::new(scheduler_client) }
	}

	/// Creates a quester were time is accelerated.
	///
	/// Time is accelerated in a way to exhibit a behavior as close as possible
	/// to what would have happened with normal time but faster.
	///
	/// The time "jumps" only happen when no actor is processing any message,
	/// running initialization or finalize.
	#[cfg(any(test, feature = "testsuite"))]
	pub fn with_accelerated_time() -> Quester {
		let quester = Quester::new();
		quester.spawn_ctx().scheduler_client.accelerate_time();
		quester
	}

	pub fn spawn_ctx(&self) -> &SpawnContext {
		&self.spawn_ctx
	}

	pub fn create_test_messagebus<A: Actor>(&self) -> (MessageBus<A>, Inbox<A>) {
		create_messagebus("test-messagebus".to_string(), QueueCapacity::Unbounded, None)
	}

	pub fn create_messagebus<A: Actor>(
		&self,
		actor_name: impl ToString,
		queue_capacity: QueueCapacity,
	) -> (MessageBus<A>, Inbox<A>) {
		self.spawn_ctx.create_messagebus(actor_name, queue_capacity)
	}

	pub fn get<A: Actor>(&self) -> Vec<MessageBus<A>> {
		self.spawn_ctx.registry.get::<A>()
	}

	pub fn get_one<A: Actor>(&self) -> Option<MessageBus<A>> {
		self.spawn_ctx.registry.get_one::<A>()
	}

	pub async fn observe(&self, timeout: Duration) -> Vec<ActorObservation> {
		self.spawn_ctx.registry.observe(timeout).await
	}

	pub fn kill(&self) {
		self.spawn_ctx.terminate_sig.kill();
	}

	/// This function acts as a drop-in replacement of
	/// `tokio::time::sleep`.
	///
	/// It can however be accelerated when using a time-accelerated
	/// quester.
	pub async fn sleep(&self, duration: Duration) {
		self.spawn_ctx.scheduler_client.sleep(duration).await;
	}

	pub fn spawn_builder<A: Actor>(&self) -> SpawnBuilder<A> {
		self.spawn_ctx.spawn_builder()
	}

	/// Inform an actor to process pending message and then stop processing new messages
	/// and exit successfully.
	pub async fn send_exit_with_success<A: Actor>(
		&self,
		messagebus: &MessageBus<A>,
	) -> Result<(), crate::SendError> {
		messagebus.send_message(Command::ExitWithSuccess).await?;
		Ok(())
	}

	/// Gracefully quits all registered actors.
	pub async fn quit(&self) -> HashMap<String, ActorExitStatus> {
		self.spawn_ctx.registry.quit().await
	}

	/// Gracefully quits all registered actors and asserts that none of them panicked.
	///
	/// This is useful for testing purposes to detect failed asserts in actors.
	#[cfg(any(test, feature = "testsuite"))]
	pub async fn assert_quit(self) {
		assert!(!self
			.quit()
			.await
			.values()
			.any(|status| matches!(status, ActorExitStatus::Panicked)));
	}
}

impl Drop for Quester {
	fn drop(&mut self) {
		if cfg!(any(test, feature = "testsuite")) &&
			!self.spawn_ctx.registry.is_empty() &&
			!thread::panicking()
		{
			panic!(
				"There are still running actors at the end of the test. Did you call \
                 quester.assert_quit()?"
			);
		}
		self.spawn_ctx.terminate_sig.kill();
	}
}

#[cfg(test)]
mod tests {
	use core::panic;
	use std::time::Duration;

	use async_trait::async_trait;

	use crate::{Actor, ActorContext, ActorExitStatus, Handler, Quester};

	#[derive(Default)]
	pub struct CountingMinutesActor {
		count: usize,
	}

	#[async_trait]
	impl Actor for CountingMinutesActor {
		type ObservableState = usize;

		fn observable_state(&self) -> usize {
			self.count
		}

		async fn initialize(&mut self, ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
			self.handle(Loop, ctx).await
		}
	}

	#[derive(Debug)]
	struct Loop;

	#[async_trait]
	impl Handler<Loop> for CountingMinutesActor {
		type Reply = ();
		async fn handle(
			&mut self,
			_msg: Loop,
			ctx: &ActorContext<Self>,
		) -> Result<(), ActorExitStatus> {
			self.count += 1;
			ctx.schedule_self_msg(Duration::from_secs(60), Loop);
			Ok(())
		}
	}

	#[derive(Default)]
	pub struct ExitPanickingActor {}

	#[async_trait]
	impl Actor for ExitPanickingActor {
		type ObservableState = ();

		fn observable_state(&self) -> Self::ObservableState {}
	}

	impl Drop for ExitPanickingActor {
		fn drop(&mut self) {
			panic!("Panicking on drop")
		}
	}

	#[tokio::test]
	async fn test_schedule_for_actor() {
		let quester = Quester::with_accelerated_time();
		let actor_with_schedule = CountingMinutesActor::default();
		let (_messagebus, handler) = quester.spawn_builder().spawn(actor_with_schedule);
		let count_after_initialization = handler.process_pending_and_observe().await.state;
		assert_eq!(count_after_initialization, 1);
		quester.sleep(Duration::from_secs(200)).await;
		let count_after_advance_time = handler.process_pending_and_observe().await.state;
		assert_eq!(count_after_advance_time, 4);
		quester.assert_quit().await;
	}

	#[tokio::test]
	async fn test_actor_quit_after_quester_quit() {
		let quester = Quester::with_accelerated_time();
		let actor_with_schedule = CountingMinutesActor::default();
		let (_messagebus, handler) = quester.spawn_builder().spawn(actor_with_schedule);
		quester.sleep(Duration::from_secs(200)).await;
		let res = quester.quit().await;
		assert_eq!(res.len(), 1);
		assert!(matches!(res.values().next().unwrap(), ActorExitStatus::Quit));
		assert!(matches!(handler.quit().await, (ActorExitStatus::Quit, 4)));
	}

	#[tokio::test]
	async fn test_quester_join_after_actor_quit() {
		let quester = Quester::default();
		let actor_with_schedule = CountingMinutesActor::default();
		let (_messagebus, handler) = quester.spawn_builder().spawn(actor_with_schedule);
		assert!(matches!(handler.quit().await, (ActorExitStatus::Quit, 1)));
		assert!(!quester
			.quit()
			.await
			.values()
			.any(|status| matches!(status, ActorExitStatus::Panicked)));
	}

	#[tokio::test]
	async fn test_quester_quit_with_panicking_actor() {
		let quester = Quester::default();
		let panicking_actor = ExitPanickingActor::default();
		let actor_with_schedule = CountingMinutesActor::default();
		let (_messagebus, _handler) = quester.spawn_builder().spawn(panicking_actor);
		let (_messagebus, _handler) = quester.spawn_builder().spawn(actor_with_schedule);
		assert!(quester
			.quit()
			.await
			.values()
			.any(|status| matches!(status, ActorExitStatus::Panicked)));
	}

	#[tokio::test]
	#[should_panic(
		expected = "There are still running actors at the end of the test. Did you call \
                    quester.assert_quit()?"
	)]
	async fn test_enforce_quester_assert_quit_calls() {
		let quester = Quester::with_accelerated_time();
		let actor_with_schedule = CountingMinutesActor::default();
		let _ = quester.spawn_builder().spawn(actor_with_schedule);
	}
}
