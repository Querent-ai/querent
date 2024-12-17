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

use regex::Regex;

pub fn split_into_sentences(text: &str) -> Vec<String> {
	let terminators = vec!['.', '?', '!', ';'];

	let parts = text
		.split_inclusive(&terminators[..])
		.map(|s| s.to_owned())
		.collect::<Vec<String>>();
	let mut results: Vec<String> = vec![];
	for part in parts {
		let first_char = part.trim().chars().next();
		let mut text = match first_char {
			Some(first_char) => {
				let mut buf = String::new();
				if !first_char.is_ascii_uppercase() && !results.is_empty() {
					buf.push_str(&results.pop().unwrap());
				}
				buf
			},
			None => String::new(),
		};
		text.push_str(&part);
		results.push(text);
	}
	results
}

#[allow(dead_code)]
pub(crate) fn clean_text(text: &str) -> String {
	text.chars().filter(|c| !c.is_ascii_control()).collect()
}

#[allow(dead_code)]
pub(crate) fn all_uppercase(text: &str) -> bool {
	text.chars().filter(|x| x.is_alphabetic()).all(|x| x.is_uppercase())
}

pub(crate) fn count_word(text: &str) -> usize {
	text.split_whitespace().filter(|x| x.chars().all(|y| y.is_alphabetic())).count()
}

pub(crate) fn count_space(text: &str) -> usize {
	let re = Regex::new(r"\s").unwrap();
	re.captures_iter(&text).count()
}

pub(crate) fn count_multi_space(text: &str) -> usize {
	let re = Regex::new(r"\s{2}").unwrap();
	re.captures_iter(&text).count()
}

pub(crate) fn check_start_with_reference_number(text: &str) -> bool {
	let re = Regex::new(r"^\[\d+\]").unwrap();
	re.is_match(text)
}

pub(crate) fn check_start_with_bullet(text: &str) -> bool {
	let re = Regex::new(r"^[-|•|‣|⁃|●|○|∙]").unwrap();
	re.is_match(text)
}
