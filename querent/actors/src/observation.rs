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

use std::{fmt, ops::Deref};

#[derive(Debug)]
pub struct Observation<ObservableState> {
	pub obs_type: ObservationType,
	pub state: ObservableState,
}

impl<ObservableState> Deref for Observation<ObservableState> {
	type Target = ObservableState;

	fn deref(&self) -> &Self::Target {
		&self.state
	}
}

// Describes the actual outcome of observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationType {
	/// The actor is alive and was able to snapshot its state within `HEARTBEAT`
	Alive,
	/// An observation could not be made with HEARTBEAT, because
	/// the actor had too much work. In that case, in a best effort fashion, the
	/// last observed state is returned. The actor will still update its state,
	/// as soon as it has finished processing the current message.
	Timeout,
	/// The actor has exited. The post-mortem state is joined.
	PostMortem,
}

impl<State: fmt::Debug + PartialEq> PartialEq for Observation<State> {
	fn eq(&self, other: &Self) -> bool {
		self.obs_type.eq(&other.obs_type) && self.state.eq(&other.state)
	}
}

impl<State: fmt::Debug + PartialEq + Eq> Eq for Observation<State> {}
