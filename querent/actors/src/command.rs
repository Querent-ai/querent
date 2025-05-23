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

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

use async_trait::async_trait;

use crate::{Actor, ActorContext, ActorExitStatus, Handler};

/// Commands are messages that can be send to control the behavior of an actor.
///
/// They are similar to UNIX signals.
///
/// They are treated with a higher priority than regular actor messages.
#[derive(Debug)]
pub enum Command {
	/// Temporarily pauses the actor. A paused actor only checks
	/// on its high priority channel and still shows "progress". It appears as
	/// healthy to the supervisor.
	///
	/// Scheduled message are still processed.
	///
	/// Semantically, it is similar to SIGSTOP.
	Pause,

	/// Resume a paused actor. If the actor was not paused this command
	/// has no effects.
	///
	/// Semantically, it is similar to SIGCONT.
	Resume,

	/// Stops the actor with a success exit status code.
	///
	/// Upstream `actors` that terminates should send the `ExitWithSuccess`
	/// command to downstream actors to inform them that there are no more
	/// incoming messages.
	///
	/// It is similar to `Quit`, except for the resulting exit status.
	ExitWithSuccess,

	/// Asks the actor to gracefully shutdown.
	///
	/// The actor will stop processing messages and its finalize function will
	/// be called.
	///
	/// The exit status is then `ActorExitStatus::Quit`.
	///
	/// This is the equivalent of sending SIGINT/Ctrl-C to a process.
	Quit,

	/// Nudging is a No-op message.
	///
	/// Its only effect is to wake-up actors that are stuck waiting
	/// for a message.
	///
	/// This is useful to kill actors properly or for tests.
	/// Actors stuck waiting for a message do not have any timeout to
	/// check for their terimatesignal signal.
	///
	///
	/// Note: Historically, actors used to have a timeout, then
	/// the wake up logic worked using a Kill command.
	/// However, after the introduction of supervision, it became common
	/// to recycle a messagebus.
	///
	/// After a panic for instance, the supervisor of an actor might kill
	/// it by activating its terimatesignal and sending a Kill message.
	///
	/// The respawned actor would receive its predecessor messagebus and
	/// possibly end up process a Kill message as its first message.
	Nudge,
}

#[async_trait]
impl<A: Actor> Handler<Command> for A {
	type Reply = ();

	/// and its exit status will be the one defined in the error.
	async fn handle(
		&mut self,
		command: Command,
		ctx: &ActorContext<Self>,
	) -> Result<Self::Reply, ActorExitStatus> {
		match command {
			Command::Pause => {
				ctx.pause();
				Ok(())
			},
			Command::ExitWithSuccess => Err(ActorExitStatus::Success),
			Command::Quit => Err(ActorExitStatus::Quit),
			Command::Nudge => Ok(()),
			Command::Resume => {
				ctx.resume();
				Ok(())
			},
		}
	}
}

/// Asks the actor to update its ObservableState.
///
/// The observation is then available using the `ActorHandler::last_observation()`
/// method.
#[derive(Debug)]
pub struct Observe;

#[async_trait]
impl<A: Actor> Handler<Observe> for A {
	type Reply = A::ObservableState;

	async fn handle(
		&mut self,
		_observe: Observe,
		ctx: &ActorContext<Self>,
	) -> Result<A::ObservableState, ActorExitStatus> {
		Ok(ctx.observe(self))
	}
}
