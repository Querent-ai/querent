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

use rand::{distributions::Alphanumeric, prelude::*};

const ADJECTIVES: &[&str] = &[
	"aged",
	"ancient",
	"autumn",
	"billowing",
	"bitter",
	"black",
	"blue",
	"bold",
	"broken",
	"cold",
	"cool",
	"crimson",
	"damp",
	"dark",
	"dawn",
	"delicate",
	"divine",
	"dry",
	"empty",
	"falling",
	"floral",
	"fragrant",
	"frosty",
	"green",
	"hidden",
	"holy",
	"icy",
	"late",
	"lingering",
	"little",
	"lively",
	"long",
	"misty",
	"morning",
	"muddy",
	"nameless",
	"old",
	"patient",
	"polished",
	"proud",
	"purple",
	"quiet",
	"red",
	"restless",
	"rough",
	"shy",
	"silent",
	"small",
	"snowy",
	"solitary",
	"sparkling",
	"spring",
	"still",
	"summer",
	"throbbing",
	"twilight",
	"wandering",
	"weathered",
	"white",
	"wild",
	"winter",
	"wispy",
	"withered",
	"young",
];

/// Returns a randomly generated id
pub fn new_quid(name: &str) -> String {
	let mut rng = rand::thread_rng();
	let adjective = ADJECTIVES[rng.gen_range(0..ADJECTIVES.len())];
	let slug: String = rng.sample_iter(&Alphanumeric).take(4).map(char::from).collect();
	format!("{name}-{adjective}-{slug}")
}

#[cfg(test)]
mod tests {
	use std::collections::HashSet;

	use super::new_quid;

	#[test]
	fn test_quid() {
		let cool_ids: HashSet<String> =
			std::iter::repeat_with(|| new_quid("hello")).take(100).collect();
		assert_eq!(cool_ids.len(), 100);
	}
}
