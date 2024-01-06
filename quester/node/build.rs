use std::{env, process::Command};

use time::{macros::format_description, OffsetDateTime};

fn main() {
	println!(
		"cargo:rustc-env=BUILD_DATE={}",
		OffsetDateTime::now_utc()
			.format(format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z"))
			.unwrap()
	);
	println!("cargo:rustc-env=BUILD_PROFILE={}", env::var("PROFILE").unwrap());
	println!("cargo:rustc-env=BUILD_TARGET={}", env::var("TARGET").unwrap());
	commit_info();
}

/// Extracts commit date, hash, and tags
fn commit_info() {
	// Extract commit date and hash.
	let output_bytes = match Command::new("git")
		.arg("log")
		.arg("-1")
		.arg("--format=%cd %H")
		.arg("--date=format-local:%Y-%m-%dT%H:%M:%SZ")
		.env("TZ", "UTC0")
		.output()
	{
		Ok(output) if output.status.success() => output.stdout,
		_ => Vec::new(),
	};
	let output = String::from_utf8(output_bytes).unwrap();
	let mut parts = output.split_whitespace();

	if let Some(commit_date) = parts.next() {
		println!("cargo:rustc-env=QUERENT_COMMIT_DATE={commit_date}");
	}
	if let Some(commit_hash) = parts.next() {
		println!("cargo:rustc-env=QUERENT_COMMIT_HASH={commit_hash}");
	}

	// Extract commit tags.
	let output_bytes = match Command::new("git").arg("tag").arg("--points-at").arg("HEAD").output()
	{
		Ok(output) if output.status.success() => output.stdout,
		_ => Vec::new(),
	};
	let output = String::from_utf8(output_bytes).unwrap();
	let tags = output.lines().collect::<Vec<_>>();
	if !tags.is_empty() {
		println!("cargo:rustc-env=QUERENT_COMMIT_TAGS={}", tags.join(","));
	}
}
