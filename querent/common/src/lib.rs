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

pub mod metrics;
pub mod terminate_sig;
pub use terminate_sig::TerimateSignal;
pub mod progress;
pub use progress::{Progress, ProtectedZoneGuard};
pub mod quid;
pub use quid::*;
pub mod runtimes;
pub use runtimes::*;
pub mod event_payload_types;
pub use event_payload_types::*;
pub mod pubsub;
pub use pubsub::*;
pub mod type_map;
pub use type_map::*;
pub mod utils;
pub use utils::*;
pub mod tower;
pub use tower::*;
pub mod retry;
pub use retry::*;
pub mod stream_utils;
pub use stream_utils::*;
pub mod sorted_iter;
pub use sorted_iter::*;
pub mod net;
pub use net::*;
pub mod error;
pub use error::*;
pub mod types;
pub use types::*;
pub mod memory;
pub use memory::*;
pub mod schemas;
pub use schemas::*;
pub mod streaming;
pub mod tools;
use std::path::PathBuf;
pub use streaming::*;

pub fn get_querent_data_path() -> PathBuf {
	let data_path = dirs::data_dir().expect("Failed to get Querent data directory");
	data_path.join("querent_data")
}
