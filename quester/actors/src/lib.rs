#![deny(clippy::disallowed_methods)]

//! actors is a simplified actor framework for quester.
//!
//! It solves the following problem:
//! - have sync and async tasks communicate together.
//! - make these task observable
//! - make these task modular and testable
//! - detect when some task is stuck and does not progress anymore

use std::{fmt, num::NonZeroU64};

use once_cell::sync::Lazy;
use tokio::time::Duration;
mod actor;
mod actor_context;
mod actor_handle;
mod actor_state;
#[doc(hidden)]
pub mod channel_with_priority;
mod command;
mod envelope;
mod messagebus;
mod observation;
mod registry;
pub(crate) mod scheduler;
mod spawn_builder;
mod supervisor;

pub use scheduler::{start_scheduler, SchedulerClient};

mod quester;
#[cfg(test)]
pub(crate) mod tests;

pub use actor::{Actor, ActorExitStatus, DeferableReplyHandler, Handler};
pub use actor_handle::{ActorHandle, Health, Healthz, Supervisable};
pub use command::{Command, Observe};
use common::TerimateSignal;
pub use observation::{Observation, ObservationType};
pub use quester::Quester;
pub use spawn_builder::SpawnContext;
use thiserror::Error;
use tracing::{info, warn};

pub use self::{
	actor_context::ActorContext,
	actor_state::ActorState,
	channel_with_priority::{QueueCapacity, RecvError, SendError, TrySendError},
	messagebus::{Inbox, MessageBus, WeakMessagebus},
	registry::ActorObservation,
	supervisor::{Supervisor, SupervisorMetrics, SupervisorState},
};

/// Heartbeat used to verify that actors are progressing.
///
/// If an actor does not advertise a progress within an interval of duration `HEARTBEAT`,
/// its supervisor will consider it as blocked and will proceed to kill it, as well
/// as all of the actors all the actors that share the terimatesignal.
pub static HEARTBEAT: Lazy<Duration> = Lazy::new(heartbeat_from_env_or_default);

/// Returns the actor's heartbeat duration:
/// - Derived from `QW_ACTOR_HEARTBEAT_SECS` if set and valid.
/// - Defaults to 30 seconds or 500ms for tests.
fn heartbeat_from_env_or_default() -> Duration {
	if cfg!(any(test, feature = "testsuite")) {
		// Right now some unit test end when we detect that a
		// pipeline has terminated, which can require waiting
		// for a heartbeat.
		//
		// We use a shorter heartbeat to reduce the time running unit tests.
		return Duration::from_millis(500);
	}
	match std::env::var("QW_ACTOR_HEARTBEAT_SECS") {
		Ok(actor_hearbeat_secs_str) => {
			if let Ok(actor_hearbeat_secs) = actor_hearbeat_secs_str.parse::<NonZeroU64>() {
				info!("set the actor heartbeat to {actor_hearbeat_secs} seconds");
				return Duration::from_secs(actor_hearbeat_secs.get());
			} else {
				warn!(
					"failed to parse `QW_ACTOR_HEARTBEAT_SECS={actor_hearbeat_secs_str}` in \
                     seconds > 0, using default heartbeat (30 seconds)"
				);
			};
		},
		Err(std::env::VarError::NotUnicode(os_str)) => {
			warn!(
				"failed to parse `QW_ACTOR_HEARTBEAT_SECS={os_str:?}` in a valid unicode string, \
                 using default heartbeat (30 seconds)"
			);
		},
		Err(std::env::VarError::NotPresent) => {},
	}
	Duration::from_secs(30)
}

/// Time we accept to wait for a new observation.
///
/// Once this time is elapsed, we just return the last observation.
const OBSERVE_TIMEOUT: Duration = Duration::from_secs(3);

/// Error that occurred while calling `ActorContext::ask(..)` or `Quester::ask`
#[derive(Error, Debug)]
pub enum AskError<E: fmt::Debug> {
	#[error("message could not be delivered")]
	MessageNotDelivered,
	#[error("error while the message was being processed")]
	ProcessMessageError,
	#[error("the handler returned an error: `{0:?}`")]
	ErrorReply(#[from] E),
}
