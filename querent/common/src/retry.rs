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

use std::{fmt::Debug, time::Duration};

use async_trait::async_trait;
use futures::Future;
use rand::Rng;
use tracing::{debug, warn};

const DEFAULT_MAX_ATTEMPTS: usize = 30;
const DEFAULT_BASE_DELAY: Duration = Duration::from_millis(250);
const DEFAULT_MAX_DELAY: Duration = Duration::from_secs(20);

pub trait Retryable {
	fn is_retryable(&self) -> bool {
		false
	}
}

#[derive(Debug, Eq, PartialEq)]
pub enum Retry<E> {
	Permanent(E),
	Transient(E),
}

impl<E> Retry<E> {
	pub fn into_inner(self) -> E {
		match self {
			Self::Transient(error) => error,
			Self::Permanent(error) => error,
		}
	}
}

impl<E> Retryable for Retry<E> {
	fn is_retryable(&self) -> bool {
		match self {
			Retry::Permanent(_) => false,
			Retry::Transient(_) => true,
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct RetryParams {
	pub base_delay: Duration,
	pub max_delay: Duration,
	pub max_attempts: usize,
}

impl Default for RetryParams {
	fn default() -> Self {
		Self {
			base_delay: DEFAULT_BASE_DELAY,
			max_delay: DEFAULT_MAX_DELAY,
			max_attempts: DEFAULT_MAX_ATTEMPTS,
		}
	}
}

impl RetryParams {
	/// Computes the delay after which a new attempt should be performed. The randomized delay
	/// increases after each attempt (exponential backoff and full jitter). Implementation and
	/// default values originate from the Java SDK. See also: <https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/>.
	///
	/// The caller should pass the number of attempts that have been performed so far. Not to be
	/// confused with the number of retries, which is one less than the number of attempts.
	///
	/// # Panics
	///
	/// Panics if `num_attempts` is zero.
	pub fn compute_delay(&self, num_attempts: usize) -> Duration {
		assert!(num_attempts > 0, "num_attempts should be greater than zero");

		let delay_ms = self.base_delay.as_millis() as u64 * 2u64.pow(num_attempts as u32 - 1);
		let ceil_delay_ms = delay_ms.min(self.max_delay.as_millis() as u64);
		let half_delay_ms = ceil_delay_ms / 2;
		let jitter_range = 0..half_delay_ms + 1;
		let jittered_delay_ms = half_delay_ms + rand::thread_rng().gen_range(jitter_range);
		Duration::from_millis(jittered_delay_ms)
	}

	#[cfg(any(test, feature = "testsuite"))]
	pub fn for_test() -> Self {
		Self {
			base_delay: Duration::from_millis(1),
			max_delay: Duration::from_millis(2),
			..Default::default()
		}
	}

	/// Creates a new [`RetryParams`] instance using settings that are more aggressive than those of
	/// the standard policy for services that are more resilient to retries, usually managed
	/// cloud services.
	pub fn aggressive() -> Self {
		Self {
			base_delay: Duration::from_millis(250),
			max_delay: Duration::from_secs(20),
			max_attempts: 5,
		}
	}
}

#[async_trait]
pub trait MockableSleep {
	async fn sleep(&self, duration: Duration);
}

pub struct TokioSleep;

#[async_trait]
impl MockableSleep for TokioSleep {
	async fn sleep(&self, duration: Duration) {
		tokio::time::sleep(duration).await;
	}
}

pub async fn retry_with_mockable_sleep<U, E, Fut>(
	retry_params: &RetryParams,
	f: impl Fn() -> Fut,
	mockable_sleep: impl MockableSleep,
) -> Result<U, E>
where
	Fut: Future<Output = Result<U, E>>,
	E: Retryable + Debug + 'static,
{
	let mut num_attempts = 0;

	loop {
		let response = f().await;

		let error = match response {
			Ok(response) => {
				return Ok(response);
			},
			Err(error) => error,
		};
		if !error.is_retryable() {
			return Err(error);
		}
		num_attempts += 1;

		if num_attempts >= retry_params.max_attempts {
			warn!(
				num_attempts=%num_attempts,
				"request failed"
			);
			return Err(error);
		}
		let delay = retry_params.compute_delay(num_attempts);
		debug!(
			num_attempts=%num_attempts,
			delay_ms=%delay.as_millis(),
			error=?error,
			"request failed, retrying"
		);
		mockable_sleep.sleep(delay).await;
	}
}

pub async fn retry<U, E, Fut>(retry_params: &RetryParams, f: impl Fn() -> Fut) -> Result<U, E>
where
	Fut: Future<Output = Result<U, E>>,
	E: Retryable + Debug + 'static,
{
	retry_with_mockable_sleep(retry_params, f, TokioSleep).await
}

#[cfg(test)]
mod tests {
	use std::{sync::RwLock, time::Duration};

	use futures::future::ready;

	use super::{retry_with_mockable_sleep, MockableSleep, RetryParams, Retryable};

	#[derive(Debug, Eq, PartialEq)]
	pub enum Retry<E> {
		Permanent(E),
		Transient(E),
	}

	impl<E> Retryable for Retry<E> {
		fn is_retryable(&self) -> bool {
			match self {
				Retry::Permanent(_) => false,
				Retry::Transient(_) => true,
			}
		}
	}

	struct NoopSleep;

	#[async_trait::async_trait]
	impl MockableSleep for NoopSleep {
		async fn sleep(&self, _duration: Duration) {
			// This is a no-op implementation, so we do nothing here.
		}
	}

	async fn simulate_retries<T>(values: Vec<Result<T, Retry<usize>>>) -> Result<T, Retry<usize>> {
		let noop_mock = NoopSleep;
		let values_it = RwLock::new(values.into_iter());
		retry_with_mockable_sleep(
			&RetryParams::default(),
			|| ready(values_it.write().unwrap().next().unwrap()),
			noop_mock,
		)
		.await
	}

	#[tokio::test]
	async fn test_retry_accepts_ok() {
		assert_eq!(simulate_retries(vec![Ok(())]).await, Ok(()));
	}

	#[tokio::test]
	async fn test_retry_does_retry() {
		assert_eq!(simulate_retries(vec![Err(Retry::Transient(1)), Ok(())]).await, Ok(()));
	}

	#[tokio::test]
	async fn test_retry_stops_retrying_on_non_retryable_error() {
		assert_eq!(
			simulate_retries(vec![Err(Retry::Permanent(1)), Ok(())]).await,
			Err(Retry::Permanent(1))
		);
	}

	#[tokio::test]
	async fn test_retry_retries_up_at_most_attempts_times() {
		let retry_sequence: Vec<_> = (0..30)
			.map(|retry_id| Err(Retry::Transient(retry_id)))
			.chain(Some(Ok(())))
			.collect();
		assert_eq!(simulate_retries(retry_sequence).await, Err(Retry::Transient(29)));
	}

	#[tokio::test]
	async fn test_retry_retries_up_to_max_attempts_times() {
		let retry_sequence: Vec<_> = (0..29)
			.map(|retry_id| Err(Retry::Transient(retry_id)))
			.chain(Some(Ok(())))
			.collect();
		assert_eq!(simulate_retries(retry_sequence).await, Ok(()));
	}
}
