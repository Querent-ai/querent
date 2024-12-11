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

use std::time::Duration;

use bytesize::ByteSize;

pub trait Rate: Clone {
	/// Returns the amount of work per time period.
	fn work(&self) -> u64;

	/// Returns the amount of work in bytes per time period.
	fn work_bytes(&self) -> ByteSize {
		ByteSize(self.work())
	}

	/// Returns the duration of a time period.
	fn period(&self) -> Duration;
}

/// A rate of unit of work per time period.
#[derive(Debug, Copy, Clone)]
pub struct ConstantRate {
	work: u64,
	period: Duration,
}

impl ConstantRate {
	/// Creates a new constant rate.
	///
	/// # Panics
	///
	/// This function panics if `period` is 0.
	pub const fn new(work: u64, period: Duration) -> Self {
		assert!(!period.is_zero());

		Self { work, period }
	}

	pub const fn bytes_per_period(bytes: ByteSize, period: Duration) -> Self {
		let work = bytes.as_u64();
		Self::new(work, period)
	}

	pub const fn bytes_per_sec(bytes: ByteSize) -> Self {
		Self::bytes_per_period(bytes, Duration::from_secs(1))
	}

	/// Changes the scale of the rate, i.e. the duration of the time period, while keeping the rate
	/// constant.
	///
	/// # Panics
	///
	/// This function panics if `new_period` is 0.
	pub fn rescale(&self, new_period: Duration) -> Self {
		assert!(!new_period.is_zero());

		let new_work = self.work() as u128 * new_period.as_nanos() / self.period().as_nanos();
		Self::new(new_work as u64, new_period)
	}
}

impl Rate for ConstantRate {
	fn work(&self) -> u64 {
		self.work
	}

	fn period(&self) -> Duration {
		self.period
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_rescale() {
		let rate = ConstantRate::bytes_per_period(ByteSize::mib(5), Duration::from_secs(5));
		let rescaled_rate = rate.rescale(Duration::from_secs(1));
		assert_eq!(rescaled_rate.work_bytes(), ByteSize::mib(1));
		assert_eq!(rescaled_rate.period(), Duration::from_secs(1));
	}
}
