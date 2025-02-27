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

use std::{collections::HashMap, fmt::Debug, path::PathBuf, pin::Pin};

use serde::{Deserialize, Serialize};
use tokio::io::AsyncRead;

pub struct CollectedBytes {
	pub data: Option<Pin<Box<dyn AsyncRead + Send>>>,
	pub file: Option<PathBuf>,
	pub eof: bool,
	pub doc_source: Option<String>,
	pub extension: Option<String>,
	pub size: Option<usize>,
	pub source_id: String,
	pub _owned_permit: Option<tokio::sync::OwnedSemaphorePermit>,
	pub image_id: Option<String>,
}

impl Debug for CollectedBytes {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CollectedBytes")
			.field("file", &self.file)
			.field("eof", &self.eof)
			.field("doc_source", &self.doc_source)
			.field("extension", &self.extension)
			.field("size", &self.size)
			.field("source_id", &self.source_id)
			.finish()
	}
}

impl CollectedBytes {
	pub fn new(
		file: Option<PathBuf>,
		data: Option<Pin<Box<dyn AsyncRead + Send>>>,
		eof: bool,
		doc_source: Option<String>,
		size: Option<usize>,
		source_id: String,
		_permit: Option<tokio::sync::OwnedSemaphorePermit>,
	) -> Self {
		let extension = file
			.as_ref()
			.and_then(|file| file.extension().and_then(|ext| ext.to_str().map(String::from)));
		CollectedBytes {
			data,
			file,
			eof,
			doc_source,
			extension,
			size,
			source_id,
			_owned_permit: _permit,
			image_id: None,
		}
	}

	pub fn is_eof(&self) -> bool {
		self.eof
	}

	pub fn get_file_path(&self) -> Option<&PathBuf> {
		self.file.as_ref()
	}

	pub fn get_extension(&self) -> Option<&String> {
		self.extension.as_ref()
	}

	pub fn unwrap(self) -> Pin<Box<dyn AsyncRead + Send>> {
		match self.data {
			Some(data) => data,
			None => panic!("Tried to unwrap an error CollectedBytes"),
		}
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Acl {
	pub viewers: Vec<String>,
	pub owners: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Legal {
	pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
	pub id: String,
	pub version: u32,
	pub kind: String,
	pub acl: Acl,
	pub legal: Legal,
	pub data: HashMap<String, serde_json::Value>,
	pub ancestry: HashMap<String, Vec<String>>,
	pub meta: Vec<HashMap<String, serde_json::Value>>,
	pub tags: HashMap<String, String>,
	pub create_user: String,
	pub create_time: String,
	pub modify_user: String,
	pub modify_time: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OsduFileGeneric {
	pub id: String,
	pub kind: String,
	pub legal: Legal,
	pub data: FileData,
	pub ancestry: Ancestry,
	#[serde(default)]
	pub meta: Vec<HashMap<String, serde_json::Value>>,
	#[serde(default)]
	pub tags: HashMap<String, String>,
	pub acl: AccessControlList,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileData {
	#[serde(rename = "Name")]
	pub name: String,
	#[serde(rename = "Description")]
	pub description: String,
	#[serde(rename = "TotalSize")]
	pub total_size: String,
	#[serde(rename = "EncodingFormatTypeID")]
	pub encoding_format_type_id: String,
	#[serde(rename = "SchemaFormatTypeID")]
	pub schema_format_type_id: String,
	#[serde(rename = "ResourceHomeRegionID")]
	pub resource_home_region_id: String,
	#[serde(rename = "ResourceHostRegionIDs")]
	pub resource_host_region_ids: Vec<String>,
	#[serde(rename = "ResourceCurationStatus")]
	pub resource_curation_status: String,
	#[serde(rename = "ResourceLifecycleStatus")]
	pub resource_lifecycle_status: String,
	#[serde(rename = "ResourceSecurityClassification")]
	pub resource_security_classification: String,
	#[serde(rename = "Source")]
	pub source: String,
	#[serde(rename = "DatasetProperties")]
	pub dataset_properties: DatasetProperties,
	#[serde(rename = "ExistenceKind")]
	pub existence_kind: String,
	#[serde(rename = "Endian")]
	pub endian: String,
	#[serde(rename = "Checksum")]
	pub checksum: String,
	#[serde(rename = "ExtensionProperties", default)]
	pub extension_properties: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatasetProperties {
	#[serde(rename = "FileSourceInfo")]
	pub file_source_info: FileSourceInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileSourceInfo {
	#[serde(rename = "FileSource")]
	pub file_source: String,
	#[serde(rename = "PreloadFilePath")]
	pub preload_file_path: String,
	#[serde(rename = "PreloadFileCreateUser")]
	pub preload_file_create_user: String,
	#[serde(rename = "PreloadFileCreateDate")]
	pub preload_file_create_date: String,
	#[serde(rename = "PreloadFileModifyUser")]
	pub preload_file_modify_user: String,
	#[serde(rename = "PreloadFileModifyDate")]
	pub preload_file_modify_date: String,
	#[serde(rename = "Name")]
	pub name: String,
	#[serde(rename = "FileSize")]
	pub file_size: String,
	#[serde(rename = "EncodingFormatTypeID")]
	pub encoding_format_type_id: String,
	#[serde(rename = "Checksum")]
	pub checksum: String,
	#[serde(rename = "ChecksumAlgorithm")]
	pub checksum_algorithm: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ancestry {
	pub parents: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessControlList {
	pub viewers: Vec<String>,
	pub owners: Vec<String>,
}
