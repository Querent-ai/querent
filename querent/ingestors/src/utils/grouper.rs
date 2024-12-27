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

use crate::document::{List, Node};

pub(crate) fn group_list_items(nodes: Vec<Node>) -> Vec<Node> {
	let mut result = vec![];
	let mut buf = vec![];
	for node in nodes {
		match node {
			Node::ListItem(_) => buf.push(node),
			_ => {
				if !buf.is_empty() {
					result.push(Node::List(
						List::builder()
							.children(buf.clone())
							.start(None)
							.position(None)
							.spread(true)
							.ordered(false)
							.build(),
					));
					result.push(Node::LineBreak);
					buf = vec![];
				}
				result.push(node);
			},
		}
	}
	if !buf.is_empty() {
		result.push(Node::List(
			List::builder()
				.children(buf.clone())
				.start(None)
				.position(None)
				.spread(true)
				.ordered(false)
				.build(),
		));
		result.push(Node::LineBreak);
	}
	result
}
