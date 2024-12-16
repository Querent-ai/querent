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

use async_trait::async_trait;
use serde::Serialize;
use tracing::{info, warn};

use crate::{
	messagebus::Inbox, Actor, ActorContext, ActorExitStatus, ActorHandle, ActorState, Handler,
	Health, Supervisable,
};

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize)]
pub struct SupervisorMetrics {
	pub num_panics: usize,
	pub num_errors: usize,
	pub num_kills: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct SupervisorState<S> {
	pub metrics: SupervisorMetrics,
	pub state_opt: Option<S>,
}

impl<S> Default for SupervisorState<S> {
	fn default() -> Self {
		SupervisorState { metrics: Default::default(), state_opt: None }
	}
}

pub struct Supervisor<A: Actor> {
	actor_name: String,
	actor_factory: Box<dyn Fn() -> A + Send>,
	inbox: Inbox<A>,
	handle_opt: Option<ActorHandle<A>>,
	metrics: SupervisorMetrics,
}

#[derive(Debug, Copy, Clone)]
struct SuperviseLoop;

#[async_trait]
impl<A: Actor> Actor for Supervisor<A> {
	type ObservableState = SupervisorState<A::ObservableState>;

	fn observable_state(&self) -> Self::ObservableState {
		let state_opt: Option<A::ObservableState> =
			self.handle_opt.as_ref().map(|handle| handle.last_observation());
		SupervisorState { metrics: self.metrics, state_opt }
	}

	fn name(&self) -> String {
		format!("Supervisor({})", self.actor_name)
	}

	fn queue_capacity(&self) -> crate::QueueCapacity {
		crate::QueueCapacity::Unbounded
	}

	async fn initialize(&mut self, ctx: &ActorContext<Self>) -> Result<(), ActorExitStatus> {
		ctx.schedule_self_msg(*crate::HEARTBEAT, SuperviseLoop);
		Ok(())
	}

	async fn finalize(
		&mut self,
		exit_status: &ActorExitStatus,
		_ctx: &ActorContext<Self>,
	) -> anyhow::Result<()> {
		match exit_status {
			ActorExitStatus::Quit =>
				if let Some(handle) = self.handle_opt.take() {
					handle.quit().await;
				},
			ActorExitStatus::Killed =>
				if let Some(handle) = self.handle_opt.take() {
					handle.kill().await;
				},
			ActorExitStatus::Failure(_) |
			ActorExitStatus::Success |
			ActorExitStatus::DownstreamClosed => {},
			ActorExitStatus::Panicked => {},
		}

		Ok(())
	}
}

impl<A: Actor> Supervisor<A> {
	pub(crate) fn new(
		actor_name: String,
		actor_factory: Box<dyn Fn() -> A + Send>,
		inbox: Inbox<A>,
		handle: ActorHandle<A>,
	) -> Self {
		Supervisor {
			actor_name,
			actor_factory,
			inbox,
			handle_opt: Some(handle),
			metrics: Default::default(),
		}
	}

	async fn supervise(
		&mut self,
		ctx: &ActorContext<Supervisor<A>>,
	) -> Result<(), ActorExitStatus> {
		let handle_ref = self.handle_opt.as_ref().expect("The actor handle should always be set.");
		match handle_ref.check_health(true) {
			Health::Healthy => {
				handle_ref.refresh_observe();
				return Ok(());
			},
			Health::FailureOrUnhealthy => {},
			Health::Success => {
				return Err(ActorExitStatus::Success);
			},
		}
		warn!("unhealthy-actor");
		// The actor is failing we need to restart it.
		let actor_handle = self.handle_opt.take().unwrap();
		let actor_messagebus = actor_handle.messagebus().clone();
		let (actor_exit_status, _last_state) = if actor_handle.state() == ActorState::Processing {
			// The actor is probably frozen.
			// Let's kill it.
			warn!("killing");
			actor_handle.kill().await
		} else {
			actor_handle.join().await
		};
		match actor_exit_status {
			ActorExitStatus::Success => {
				return Err(ActorExitStatus::Success);
			},
			ActorExitStatus::Quit => {
				return Err(ActorExitStatus::Quit);
			},
			ActorExitStatus::DownstreamClosed => {
				return Err(ActorExitStatus::DownstreamClosed);
			},
			ActorExitStatus::Killed => {
				self.metrics.num_kills += 1;
			},
			ActorExitStatus::Failure(_err) => {
				self.metrics.num_errors += 1;
			},
			ActorExitStatus::Panicked => {
				self.metrics.num_panics += 1;
			},
		}
		info!("respawning-actor");
		let (_, actor_handle) = ctx
			.spawn_actor()
			.set_messagebuses(actor_messagebus, self.inbox.clone())
			.set_terminate_sig(ctx.terminate_sig().child())
			.spawn((*self.actor_factory)());
		self.handle_opt = Some(actor_handle);
		Ok(())
	}
}

#[async_trait]
impl<A: Actor> Handler<SuperviseLoop> for Supervisor<A> {
	type Reply = ();

	async fn handle(
		&mut self,
		_msg: SuperviseLoop,
		ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		self.supervise(ctx).await?;
		ctx.schedule_self_msg(*crate::HEARTBEAT, SuperviseLoop);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use async_trait::async_trait;
	use tracing::info;

	use crate::{
		supervisor::SupervisorMetrics,
		tests::{Ping, PingReceiverActor},
		Actor, ActorContext, ActorExitStatus, AskError, Handler, Observe, Querent,
	};

	#[derive(Copy, Clone, Debug)]
	enum FailingActorMessage {
		Panic,
		ReturnError,
		Increment,
		Freeze(Duration),
	}

	#[derive(Default, Clone)]
	struct FailingActor {
		counter: usize,
	}

	#[async_trait]
	impl Actor for FailingActor {
		type ObservableState = usize;

		fn name(&self) -> String {
			"FailingActor".to_string()
		}

		fn observable_state(&self) -> Self::ObservableState {
			self.counter
		}

		async fn finalize(
			&mut self,
			_exit_status: &ActorExitStatus,
			_ctx: &ActorContext<Self>,
		) -> anyhow::Result<()> {
			info!("finalize-failing-actor");
			Ok(())
		}
	}

	#[async_trait]
	impl Handler<FailingActorMessage> for FailingActor {
		type Reply = usize;

		async fn handle(
			&mut self,
			msg: FailingActorMessage,
			ctx: &ActorContext<Self>,
		) -> Result<Self::Reply, ActorExitStatus> {
			match msg {
				FailingActorMessage::Panic => {
					panic!("Failing actor panicked");
				},
				FailingActorMessage::ReturnError => {
					return Err(ActorExitStatus::from(anyhow::anyhow!("failing actor error")));
				},
				FailingActorMessage::Increment => {
					self.counter += 1;
				},
				FailingActorMessage::Freeze(wait_duration) => {
					ctx.sleep(wait_duration).await;
				},
			}
			Ok(self.counter)
		}
	}

	#[tokio::test]
	async fn test_supervisor_restart_on_panic() {
		let querent = Querent::with_accelerated_time();
		let actor = FailingActor::default();
		let (messagebus, supervisor_handle) = querent.spawn_builder().supervise(actor);
		assert_eq!(messagebus.ask(FailingActorMessage::Increment).await.unwrap(), 1);
		assert_eq!(messagebus.ask(FailingActorMessage::Increment).await.unwrap(), 2);
		assert!(messagebus.ask(FailingActorMessage::Panic).await.is_err());
		assert_eq!(messagebus.ask(FailingActorMessage::Increment).await.unwrap(), 1);
		assert_eq!(
			supervisor_handle.observe().await.metrics,
			SupervisorMetrics { num_panics: 1, num_errors: 0, num_kills: 0 }
		);
		assert!(!matches!(supervisor_handle.quit().await.0, ActorExitStatus::Panicked));
	}

	#[tokio::test]
	async fn test_supervisor_restart_on_error() {
		let querent = Querent::with_accelerated_time();
		let actor = FailingActor::default();
		let (messagebus, supervisor_handle) = querent.spawn_builder().supervise(actor);
		assert_eq!(messagebus.ask(FailingActorMessage::Increment).await.unwrap(), 1);
		assert_eq!(messagebus.ask(FailingActorMessage::Increment).await.unwrap(), 2);
		assert!(messagebus.ask(FailingActorMessage::ReturnError).await.is_err());
		assert_eq!(messagebus.ask(FailingActorMessage::Increment).await.unwrap(), 1);
		assert_eq!(
			supervisor_handle.observe().await.metrics,
			SupervisorMetrics { num_panics: 0, num_errors: 1, num_kills: 0 }
		);
		assert!(!matches!(supervisor_handle.quit().await.0, ActorExitStatus::Panicked));
	}

	#[tokio::test]
	async fn test_supervisor_kills_and_restart_frozen_actor() {
		let querent = Querent::with_accelerated_time();
		let actor = FailingActor::default();
		let (messagebus, supervisor_handle) = querent.spawn_builder().supervise(actor);
		assert_eq!(messagebus.ask(FailingActorMessage::Increment).await.unwrap(), 1);
		assert_eq!(messagebus.ask(FailingActorMessage::Increment).await.unwrap(), 2);
		assert_eq!(
			supervisor_handle.observe().await.metrics,
			SupervisorMetrics { num_panics: 0, num_errors: 0, num_kills: 0 }
		);
		messagebus
			.send_message(FailingActorMessage::Freeze(crate::HEARTBEAT.mul_f32(3.0f32)))
			.await
			.unwrap();
		assert_eq!(messagebus.ask(FailingActorMessage::Increment).await.unwrap(), 1);
		assert_eq!(
			supervisor_handle.observe().await.metrics,
			SupervisorMetrics { num_panics: 0, num_errors: 0, num_kills: 1 }
		);
		assert!(!matches!(supervisor_handle.quit().await.0, ActorExitStatus::Panicked));
	}

	#[tokio::test]
	async fn test_supervisor_forwards_quit_commands() {
		let querent = Querent::with_accelerated_time();
		let actor = FailingActor::default();
		let (messagebus, supervisor_handle) = querent.spawn_builder().supervise(actor);
		assert_eq!(messagebus.ask(FailingActorMessage::Increment).await.unwrap(), 1);
		let (exit_status, _state) = supervisor_handle.quit().await;
		assert!(matches!(
			messagebus.ask(FailingActorMessage::Increment).await.unwrap_err(),
			AskError::MessageNotDelivered
		));
		assert!(matches!(exit_status, ActorExitStatus::Quit));
	}

	#[tokio::test]
	async fn test_supervisor_forwards_kill_command() {
		let querent = Querent::with_accelerated_time();
		let actor = FailingActor::default();
		let (messagebus, supervisor_handle) = querent.spawn_builder().supervise(actor);
		assert_eq!(messagebus.ask(FailingActorMessage::Increment).await.unwrap(), 1);
		let (exit_status, _state) = supervisor_handle.kill().await;
		assert!(messagebus.ask(FailingActorMessage::Increment).await.is_err());
		assert!(matches!(
			messagebus.ask(FailingActorMessage::Increment).await.unwrap_err(),
			AskError::MessageNotDelivered
		));
		assert!(matches!(exit_status, ActorExitStatus::Killed));
	}

	#[tokio::test]
	async fn test_supervisor_exits_successfully_when_supervised_actor_messagebus_is_dropped() {
		let querent = Querent::with_accelerated_time();
		let actor = FailingActor::default();
		let (_, supervisor_handle) = querent.spawn_builder().supervise(actor);
		let (exit_status, _state) = supervisor_handle.join().await;
		assert!(matches!(exit_status, ActorExitStatus::Success));
		querent.assert_quit().await;
	}

	#[tokio::test]
	async fn test_supervisor_state() {
		let querent = Querent::with_accelerated_time();
		let ping_actor = PingReceiverActor::default();
		let (messagebus, handler) = querent.spawn_builder().supervise(ping_actor);
		let obs = handler.observe().await;
		assert_eq!(obs.state.state_opt, Some(0));
		let _ = messagebus.ask(Ping).await;
		assert_eq!(messagebus.ask(Observe).await.unwrap(), 1);
		querent.sleep(Duration::from_secs(60)).await;
		let obs = handler.observe().await;
		assert_eq!(obs.state.state_opt, Some(1));
		handler.quit().await;
		querent.assert_quit().await;
	}
}
