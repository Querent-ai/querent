use std::{cell::Cell, collections::HashMap, ops::Mul, time::Duration};

use async_trait::async_trait;
use serde::Serialize;

use crate::{
	observation::ObservationType, Actor, ActorContext, ActorExitStatus, ActorHandle, ActorState,
	Command, Handler, Health, MessageBus, Observation, Quester, Supervisable,
};

// An actor that receives ping messages.
#[derive(Default, Clone)]
pub struct PingReceiverActor {
	ping_count: usize,
}

impl Actor for PingReceiverActor {
	type ObservableState = usize;

	fn name(&self) -> String {
		"Ping".to_string()
	}

	fn observable_state(&self) -> Self::ObservableState {
		self.ping_count
	}
}

#[derive(Debug)]
pub struct Ping;

#[async_trait]
impl Handler<Ping> for PingReceiverActor {
	type Reply = ();

	async fn handle(
		&mut self,
		_message: Ping,
		ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		self.ping_count += 1;
		assert_eq!(ctx.state(), ActorState::Processing);
		Ok(())
	}
}

#[derive(Default)]
pub struct PingerSenderActor {
	count: usize,
	peers: HashMap<String, MessageBus<PingReceiverActor>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SenderState {
	pub count: usize,
	pub num_peers: usize,
}

#[derive(Clone, Debug)]
pub struct AddPeer(MessageBus<PingReceiverActor>);

impl Actor for PingerSenderActor {
	type ObservableState = SenderState;

	fn name(&self) -> String {
		"PingSender".to_string()
	}

	fn observable_state(&self) -> Self::ObservableState {
		SenderState { count: self.count, num_peers: self.peers.len() }
	}
}

#[async_trait]
impl Handler<Ping> for PingerSenderActor {
	type Reply = ();

	async fn handle(
		&mut self,
		_message: Ping,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		self.count += 1;
		for peer in self.peers.values() {
			let _ = peer.send_message(Ping).await;
		}
		Ok(())
	}
}

#[async_trait]
impl Handler<AddPeer> for PingerSenderActor {
	type Reply = ();

	async fn handle(
		&mut self,
		message: AddPeer,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		let AddPeer(peer) = message;
		let peer_id = peer.actor_instance_id().to_string();
		self.peers.insert(peer_id, peer);
		Ok(())
	}
}

#[tokio::test]
async fn test_actor_stops_when_last_messagebus_is_dropped() {
	let quester = Quester::with_accelerated_time();
	let (ping_recv_messagebus, ping_recv_handle) =
		quester.spawn_builder().spawn(PingReceiverActor::default());
	drop(ping_recv_messagebus);
	let (exit_status, _) = ping_recv_handle.join().await;
	assert!(exit_status.is_success());
}

#[tokio::test]
async fn test_ping_actor() {
	let quester = Quester::with_accelerated_time();
	let (ping_recv_messagebus, ping_recv_handle) =
		quester.spawn_builder().spawn(PingReceiverActor::default());
	let (ping_sender_messagebus, ping_sender_handle) =
		quester.spawn_builder().spawn(PingerSenderActor::default());
	assert_eq!(
		ping_recv_handle.observe().await,
		Observation { obs_type: ObservationType::Alive, state: 0 }
	);
	// No peers. This one will have no impact.
	let ping_recv_messagebus = ping_recv_messagebus.clone();
	assert!(ping_sender_messagebus.send_message(Ping).await.is_ok());
	assert!(ping_sender_messagebus
		.send_message(AddPeer(ping_recv_messagebus.clone()))
		.await
		.is_ok());
	assert_eq!(
		ping_sender_handle.process_pending_and_observe().await,
		Observation {
			obs_type: ObservationType::Alive,
			state: SenderState { num_peers: 1, count: 1 }
		}
	);
	assert!(ping_sender_messagebus.send_message(Ping).await.is_ok());
	assert!(ping_sender_messagebus.send_message(Ping).await.is_ok());
	assert_eq!(
		ping_sender_handle.process_pending_and_observe().await,
		Observation {
			obs_type: ObservationType::Alive,
			state: SenderState { num_peers: 1, count: 3 }
		}
	);
	assert_eq!(
		ping_recv_handle.process_pending_and_observe().await,
		Observation { obs_type: ObservationType::Alive, state: 2 }
	);
	quester.kill();
	assert_eq!(
		ping_recv_handle.process_pending_and_observe().await,
		Observation { obs_type: ObservationType::PostMortem, state: 2 }
	);
	assert_eq!(
		ping_sender_handle.process_pending_and_observe().await,
		Observation {
			obs_type: ObservationType::PostMortem,
			state: SenderState { num_peers: 1, count: 3 }
		}
	);
	ping_sender_handle.join().await;
	assert!(ping_sender_messagebus.send_message(Ping).await.is_err());
}

struct BuggyActor;

#[derive(Clone, Debug)]
struct DoNothing;

#[derive(Clone, Debug)]
struct Block;

impl Actor for BuggyActor {
	type ObservableState = ();

	fn name(&self) -> String {
		"BuggyActor".to_string()
	}

	fn observable_state(&self) {}
}

#[async_trait]
impl Handler<DoNothing> for BuggyActor {
	type Reply = ();

	async fn handle(
		&mut self,
		_message: DoNothing,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		Ok(())
	}
}

#[async_trait]
impl Handler<Block> for BuggyActor {
	type Reply = ();

	async fn handle(
		&mut self,
		_message: Block,
		ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		while ctx.kill_switch().is_alive() {
			tokio::task::yield_now().await;
		}
		Ok(())
	}
}

#[tokio::test]
async fn test_timeouting_actor() {
	let quester = Quester::with_accelerated_time();
	let (buggy_messagebus, buggy_handle) = quester.spawn_builder().spawn(BuggyActor);
	let buggy_messagebus = buggy_messagebus;
	assert_eq!(buggy_handle.observe().await.obs_type, ObservationType::Alive);
	assert!(buggy_messagebus.send_message(DoNothing).await.is_ok());
	assert_eq!(buggy_handle.observe().await.obs_type, ObservationType::Alive);
	assert!(buggy_messagebus.send_message(Block).await.is_ok());

	assert_eq!(buggy_handle.check_health(true), Health::Healthy);
	assert_eq!(buggy_handle.process_pending_and_observe().await.obs_type, ObservationType::Timeout);
	assert_eq!(buggy_handle.check_health(true), Health::Healthy);
	quester.sleep(crate::HEARTBEAT.mul(2)).await;
	assert_eq!(buggy_handle.check_health(true), Health::FailureOrUnhealthy);
	buggy_handle.kill().await;
}

#[tokio::test]
async fn test_pause_actor() {
	let quester = Quester::with_accelerated_time();
	let (ping_messagebus, ping_handle) =
		quester.spawn_builder().spawn(PingReceiverActor::default());
	for _ in 0u32..1000u32 {
		assert!(ping_messagebus.send_message(Ping).await.is_ok());
	}
	assert!(ping_messagebus.send_message_with_high_priority(Command::Pause).is_ok());
	let first_state = ping_handle.observe().await.state;
	assert!(first_state < 1000);
	let second_state = ping_handle.observe().await.state;
	assert_eq!(first_state, second_state);
	assert!(ping_messagebus.send_message_with_high_priority(Command::Resume).is_ok());
	let end_state = ping_handle.process_pending_and_observe().await.state;
	assert_eq!(end_state, 1000);
	quester.assert_quit().await;
}

#[tokio::test]
async fn test_actor_running_states() {
	let quester = Quester::with_accelerated_time();
	let (ping_messagebus, ping_handle) =
		quester.spawn_builder().spawn(PingReceiverActor::default());
	assert!(ping_handle.state() == ActorState::Processing);
	for _ in 0u32..10u32 {
		assert!(ping_messagebus.send_message(Ping).await.is_ok());
	}
	let obs = ping_handle.process_pending_and_observe().await;
	assert_eq!(*obs, 10);
	quester.sleep(Duration::from_millis(1)).await;
	assert!(ping_handle.state() == ActorState::Idle);
	quester.assert_quit().await;
}

#[derive(Clone, Debug, Default, Serialize)]
struct LoopingActor {
	pub loop_count: usize,
	pub single_shot_count: usize,
}

#[derive(Debug)]
struct Loop;

#[derive(Debug)]
struct SingleShot;

#[async_trait]
impl Actor for LoopingActor {
	type ObservableState = Self;

	fn observable_state(&self) -> Self::ObservableState {
		self.clone()
	}

	fn yield_after_each_message(&self) -> bool {
		false
	}

	async fn initialize(&mut self, ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
		self.handle(Loop, ctx).await
	}
}

#[async_trait]
impl Handler<Loop> for LoopingActor {
	type Reply = ();
	async fn handle(
		&mut self,
		_msg: Loop,
		ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		self.loop_count += 1;
		ctx.send_self_message(Loop).await?;
		Ok(())
	}
}

#[async_trait]
impl Handler<SingleShot> for LoopingActor {
	type Reply = ();

	async fn handle(
		&mut self,
		_msg: SingleShot,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		self.single_shot_count += 1;
		Ok(())
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn test_looping() -> anyhow::Result<()> {
	let quester = Quester::with_accelerated_time();
	let looping_actor = LoopingActor::default();
	let (looping_actor_messagebus, looping_actor_handle) =
		quester.spawn_builder().spawn(looping_actor);
	assert!(looping_actor_messagebus.send_message(SingleShot).await.is_ok());
	looping_actor_handle.process_pending_and_observe().await;
	let (exit_status, state) = looping_actor_handle.quit().await;
	assert!(matches!(exit_status, ActorExitStatus::Quit));
	assert_eq!(state.single_shot_count, 1);
	assert!(state.loop_count > 0);
	Ok(())
}

#[derive(Default)]
struct SummingActor {
	sum: u64,
}

#[async_trait]
impl Handler<u64> for SummingActor {
	type Reply = ();

	async fn handle(&mut self, add: u64, _ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
		self.sum += add;
		Ok(())
	}
}

impl Actor for SummingActor {
	type ObservableState = u64;

	fn observable_state(&self) -> Self::ObservableState {
		self.sum
	}
}

#[derive(Default)]
struct SpawningActor {
	res: u64,
	handle_opt: Option<(MessageBus<SummingActor>, ActorHandle<SummingActor>)>,
}

#[async_trait]
impl Actor for SpawningActor {
	type ObservableState = u64;

	fn observable_state(&self) -> Self::ObservableState {
		self.res
	}

	async fn finalize(
		&mut self,
		_exit_status: &ActorExitStatus,
		_ctx: &ActorContext<Self>,
	) -> anyhow::Result<()> {
		if let Some((_, child_handler)) = self.handle_opt.take() {
			self.res = child_handler.process_pending_and_observe().await.state;
			child_handler.kill().await;
		}
		Ok(())
	}
}

#[async_trait]
impl Handler<u64> for SpawningActor {
	type Reply = ();

	async fn handle(
		&mut self,
		message: u64,
		ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		let (messagebus, _) = self
			.handle_opt
			.get_or_insert_with(|| ctx.spawn_actor().spawn(SummingActor::default()));
		ctx.send_message(messagebus, message).await?;
		Ok(())
	}
}

#[tokio::test]
async fn test_actor_spawning_actor() -> anyhow::Result<()> {
	let quester = Quester::with_accelerated_time();
	let (messagebus, handle) = quester.spawn_builder().spawn(SpawningActor::default());
	messagebus.send_message(1).await?;
	messagebus.send_message(2).await?;
	messagebus.send_message(3).await?;
	drop(messagebus);
	let (exit, result) = handle.join().await;
	assert!(matches!(exit, ActorExitStatus::Success));
	assert_eq!(result, 6);
	Ok(())
}

struct BuggyFinalizeActor;

#[async_trait]
impl Actor for BuggyFinalizeActor {
	type ObservableState = ();

	fn name(&self) -> String {
		"BuggyFinalizeActor".to_string()
	}

	fn observable_state(&self) {}

	async fn finalize(
		&mut self,
		_exit_status: &ActorExitStatus,
		_: &ActorContext<Self>,
	) -> anyhow::Result<()> {
		anyhow::bail!("finalize error")
	}
}

#[tokio::test]
async fn test_actor_finalize_error_set_exit_status_to_panicked() -> anyhow::Result<()> {
	let quester = Quester::with_accelerated_time();
	let (messagebus, handle) = quester.spawn_builder().spawn(BuggyFinalizeActor);
	assert!(matches!(handle.state(), ActorState::Processing));
	drop(messagebus);
	let (exit, _) = handle.join().await;
	assert!(matches!(exit, ActorExitStatus::Panicked));
	Ok(())
}

#[derive(Default)]
struct Adder(u64);

impl Actor for Adder {
	type ObservableState = u64;

	fn yield_after_each_message(&self) -> bool {
		false
	}

	fn observable_state(&self) -> Self::ObservableState {
		self.0
	}
}

#[derive(Debug)]
struct AddOperand(u64);

#[async_trait]
impl Handler<AddOperand> for Adder {
	type Reply = u64;

	async fn handle(
		&mut self,
		add_op: AddOperand,
		_ctx: &ActorContext<Self>,
	) -> Result<u64, ActorExitStatus> {
		self.0 += add_op.0;
		Ok(self.0)
	}
}

#[derive(Debug)]
struct Sleep(Duration);

#[tokio::test]
async fn test_actor_return_response() -> anyhow::Result<()> {
	let quester = Quester::with_accelerated_time();
	let adder = Adder::default();
	let (messagebus, _handle) = quester.spawn_builder().spawn(adder);
	let plus_two = messagebus.send_message(AddOperand(2)).await?;
	let plus_two_plus_four = messagebus.send_message(AddOperand(4)).await?;
	assert_eq!(plus_two.await.unwrap(), 2);
	assert_eq!(plus_two_plus_four.await.unwrap(), 6);
	quester.assert_quit().await;
	Ok(())
}

#[derive(Default)]
struct TestActorWithDrain {
	counts: ProcessAndDrainCounts,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
struct ProcessAndDrainCounts {
	process_calls_count: usize,
	drain_calls_count: usize,
}

#[async_trait]
impl Actor for TestActorWithDrain {
	type ObservableState = ProcessAndDrainCounts;

	fn observable_state(&self) -> ProcessAndDrainCounts {
		self.counts
	}

	async fn on_drained_messages(
		&mut self,
		_ctx: &ActorContext<Self>,
	) -> Result<(), ActorExitStatus> {
		self.counts.drain_calls_count += 1;
		Ok(())
	}
}

#[async_trait]
impl Handler<()> for TestActorWithDrain {
	type Reply = ();

	async fn handle(
		&mut self,
		_message: (),
		_ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		self.counts.process_calls_count += 1;
		Ok(())
	}
}

#[tokio::test]
async fn test_drain_is_called() {
	let quester = Quester::with_accelerated_time();
	let test_actor_with_drain = TestActorWithDrain::default();
	let (messagebus, handle) = quester.spawn_builder().spawn(test_actor_with_drain);
	assert_eq!(
		*handle.process_pending_and_observe().await,
		ProcessAndDrainCounts { process_calls_count: 0, drain_calls_count: 0 }
	);
	handle.pause();
	messagebus.send_message(()).await.unwrap();
	messagebus.send_message(()).await.unwrap();
	messagebus.send_message(()).await.unwrap();
	handle.resume();
	quester.sleep(Duration::from_millis(1)).await;
	assert_eq!(
		*handle.process_pending_and_observe().await,
		ProcessAndDrainCounts { process_calls_count: 3, drain_calls_count: 1 }
	);
	messagebus.send_message(()).await.unwrap();
	quester.sleep(Duration::from_millis(1)).await;
	assert_eq!(
		*handle.process_pending_and_observe().await,
		ProcessAndDrainCounts { process_calls_count: 4, drain_calls_count: 2 }
	);
	quester.assert_quit().await;
}

#[tokio::test]
async fn test_unsync_actor() {
	#[derive(Default)]
	struct UnsyncActor(Cell<u64>);

	impl Actor for UnsyncActor {
		type ObservableState = u64;

		fn observable_state(&self) -> Self::ObservableState {
			self.0.get()
		}
	}

	#[async_trait]
	impl Handler<u64> for UnsyncActor {
		type Reply = u64;

		async fn handle(
			&mut self,
			number: u64,
			_ctx: &ActorContext<Self>,
		) -> Result<u64, ActorExitStatus> {
			*self.0.get_mut() += number;
			Ok(self.0.get())
		}
	}
	let quester = Quester::with_accelerated_time();
	let unsync_message_actor = UnsyncActor::default();
	let (messagebus, _handle) = quester.spawn_builder().spawn(unsync_message_actor);

	let response = messagebus.ask(1).await.unwrap();
	assert_eq!(response, 1);

	quester.assert_quit().await;
}

#[tokio::test]
async fn test_unsync_actor_message() {
	#[derive(Default)]
	struct UnsyncMessageActor(u64);

	impl Actor for UnsyncMessageActor {
		type ObservableState = u64;

		fn observable_state(&self) -> Self::ObservableState {
			self.0
		}
	}

	#[async_trait]
	impl Handler<Cell<u64>> for UnsyncMessageActor {
		type Reply = anyhow::Result<u64>;

		async fn handle(
			&mut self,
			number: Cell<u64>,
			_ctx: &ActorContext<Self>,
		) -> Result<anyhow::Result<u64>, ActorExitStatus> {
			self.0 += number.get();
			Ok(Ok(self.0))
		}
	}
	let quester = Quester::with_accelerated_time();
	let unsync_message_actor = UnsyncMessageActor::default();
	let (messagebus, _handle) = quester.spawn_builder().spawn(unsync_message_actor);

	let response_rx = messagebus.send_message(Cell::new(1)).await.unwrap();
	assert_eq!(response_rx.await.unwrap().unwrap(), 1);

	let response = messagebus.ask(Cell::new(1)).await.unwrap().unwrap();
	assert_eq!(response, 2);

	let response = messagebus.ask_for_res(Cell::new(1)).await.unwrap();
	assert_eq!(response, 3);

	let response_rx = messagebus.send_message_with_high_priority(Cell::new(1)).unwrap();
	assert_eq!(response_rx.await.unwrap().unwrap(), 4);

	let response_rx = messagebus.try_send_message(Cell::new(1)).unwrap();
	assert_eq!(response_rx.await.unwrap().unwrap(), 5);

	quester.assert_quit().await;
}
