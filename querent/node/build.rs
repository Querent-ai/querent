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

use std::{env, process::Command};

use time::{macros::format_description, OffsetDateTime};

fn main() {
	// This is just a hack to simplify Windows builds.
	println!(
		"cargo:rustc-env=BUILD_DATE={}",
		OffsetDateTime::now_utc()
			.format(format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z"))
			.unwrap()
	);
	println!("cargo:rustc-env=BUILD_PROFILE={}", env::var("PROFILE").unwrap());
	println!("cargo:rustc-env=BUILD_TARGET={}", env::var("TARGET").unwrap());
	commit_info();
	#[cfg(target_os = "windows")]
	{
		download_windows_npcap_sdk();
	}
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

#[cfg(target_os = "windows")]
fn download_windows_npcap_sdk() {
	use std::{
		fs,
		io::{self, Write},
		path::PathBuf,
	};

	use http_req::request;
	use zip::ZipArchive;

	println!("cargo:rerun-if-changed=build.rs");

	// get npcap SDK
	const NPCAP_SDK: &str = "npcap-sdk-1.13.zip";

	let npcap_sdk_download_url = format!("https://npcap.com/dist/{NPCAP_SDK}");
	let cache_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("target");
	let npcap_sdk_cache_path = cache_dir.join(NPCAP_SDK);

	let npcap_zip = match fs::read(&npcap_sdk_cache_path) {
		// use cached
		Ok(zip_data) => {
			eprintln!("Found cached npcap SDK");
			zip_data
		},
		// download SDK
		Err(_) => {
			eprintln!("Downloading npcap SDK");

			// download
			let mut zip_data = vec![];
			let _res = request::get(npcap_sdk_download_url, &mut zip_data).unwrap();

			// write cache
			fs::create_dir_all(cache_dir).unwrap();
			let mut cache = fs::File::create(npcap_sdk_cache_path).unwrap();
			cache.write_all(&zip_data).unwrap();

			zip_data
		},
	};

	// extract DLL
	let lib_path = if cfg!(target_arch = "aarch64") {
		"Lib/ARM64/Packet.lib"
	} else if cfg!(target_arch = "x86_64") {
		"Lib/x64/Packet.lib"
	} else if cfg!(target_arch = "x86") {
		"Lib/Packet.lib"
	} else {
		panic!("Unsupported target!")
	};
	let mut archive = ZipArchive::new(io::Cursor::new(npcap_zip)).unwrap();
	let mut npcap_lib = archive.by_name(lib_path).unwrap();

	// write DLL
	let lib_dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("npcap_sdk");
	let lib_path = lib_dir.join("Packet.lib");
	fs::create_dir_all(&lib_dir).unwrap();
	let mut lib_file = fs::File::create(lib_path).unwrap();
	io::copy(&mut npcap_lib, &mut lib_file).unwrap();

	println!("cargo:rustc-link-search=native={}", lib_dir.to_str().unwrap());
}
