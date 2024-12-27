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

#![deny(clippy::disallowed_methods)]

//! actors is a simplified actor framework for querent.
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

mod querent;
#[cfg(test)]
pub(crate) mod tests;

pub use actor::{Actor, ActorExitStatus, DeferableReplyHandler, Handler};
pub use actor_handle::{ActorHandle, Health, Healthz, Supervisable};
pub use command::{Command, Observe};
use common::{ServiceError, ServiceErrorCode, TerimateSignal};
pub use observation::{Observation, ObservationType};
pub use querent::Querent;
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
		return Duration::from_millis(30000);
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

/// Error that occurred while calling `ActorContext::ask(..)` or `Querent::ask`
#[derive(Error, Debug)]
pub enum AskError<E: fmt::Debug> {
	#[error("message could not be delivered")]
	MessageNotDelivered,
	#[error("error while the message was being processed")]
	ProcessMessageError,
	#[error("the handler returned an error: `{0:?}`")]
	ErrorReply(#[from] E),
}

impl<E: fmt::Debug + ServiceError> ServiceError for AskError<E> {
	fn error_code(&self) -> ServiceErrorCode {
		match self {
			AskError::MessageNotDelivered => ServiceErrorCode::Internal,
			AskError::ProcessMessageError => ServiceErrorCode::Internal,
			AskError::ErrorReply(err) => err.error_code(),
		}
	}
}
