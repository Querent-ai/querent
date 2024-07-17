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
