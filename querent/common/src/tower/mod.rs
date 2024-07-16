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
