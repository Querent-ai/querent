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

use std::collections::HashMap;

pub fn mean(values: &Vec<f32>) -> f32 {
	let s = values.iter().sum::<f32>();

	s / values.len() as f32
}

pub fn stdev(values: &Vec<f32>) -> f32 {
	let u = mean(values);
	let n = values.len();

	values.iter().map(|x| (x - u).powi(2) as f32 / n as f32).sum::<f32>()
}

pub fn mode(values: &Vec<f32>) -> f32 {
	let mut counter = HashMap::new();

	for value in values {
		let key = value.to_string();
		counter.entry(key).and_modify(|count| *count += 1).or_insert(0);
	}
	let mut counter_vec: Vec<_> = counter.iter().collect();
	counter_vec.sort_by(|a, b| a.1.cmp(b.1));

	match counter_vec.last() {
		Some(item) => item.0.parse::<f32>().unwrap(),
		None => 0.0,
	}
}
