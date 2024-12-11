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

mod box_layer;
mod box_service;
mod buffer;
mod change;
mod estimate_rate;
mod event_listener;
mod metrics;
mod pool;
mod rate;
mod rate_estimator;
mod rate_limit;
mod retry;
mod transport;

use std::{error, pin::Pin};

pub use box_layer::BoxLayer;
pub use box_service::BoxService;
pub use buffer::{Buffer, BufferError, BufferLayer};
pub use change::Change;
pub use estimate_rate::{EstimateRate, EstimateRateLayer};
pub use event_listener::{EventListener, EventListenerLayer};
use futures::Future;
pub use metrics::*;
pub use pool::Pool;
pub use rate::{ConstantRate, Rate};
pub use rate_estimator::{RateEstimator, SmaRateEstimator};
pub use rate_limit::{RateLimit, RateLimitLayer};
pub use retry::{RetryLayer, RetryPolicy};
pub use transport::{make_channel, warmup_channel, BalanceChannel};

pub type BoxError = Box<dyn error::Error + Send + Sync + 'static>;

pub type BoxFuture<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>;

pub type BoxFutureInfaillible<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub trait Cost {
	fn cost(&self) -> u64;
}
