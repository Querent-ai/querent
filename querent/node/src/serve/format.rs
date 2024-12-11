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

use serde::{Deserialize, Serialize, Serializer};
use warp::{Filter, Rejection};

/// Body output format used for the REST API.
#[derive(Deserialize, Clone, Debug, Eq, PartialEq, Copy, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum BodyFormat {
	Json,
	#[default]
	PrettyJson,
}

impl BodyFormat {
	pub(crate) fn result_to_vec<T: serde::Serialize, E: serde::Serialize>(
		&self,
		result: &Result<T, E>,
	) -> Result<Vec<u8>, ()> {
		match result {
			Ok(value) => self.value_to_vec(value),
			Err(err) => self.value_to_vec(err),
		}
	}

	fn value_to_vec(&self, value: &impl serde::Serialize) -> Result<Vec<u8>, ()> {
		match &self {
			Self::Json => serde_json::to_vec(value).map_err(|_| {
				tracing::error!("the response serialization failed");
			}),
			Self::PrettyJson => serde_json::to_vec_pretty(value).map_err(|_| {
				tracing::error!("the response serialization failed");
			}),
		}
	}
}

impl ToString for BodyFormat {
	fn to_string(&self) -> String {
		match &self {
			Self::Json => "json".to_string(),
			Self::PrettyJson => "pretty_json".to_string(),
		}
	}
}

impl Serialize for BodyFormat {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.to_string())
	}
}

/// This struct represents a QueryString passed to
/// the REST API.
#[derive(Deserialize, Debug, Eq, PartialEq, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct FormatQueryString {
	/// The output format requested.
	#[serde(default)]
	pub format: BodyFormat,
}

pub(crate) fn extract_format_from_qs(
) -> impl Filter<Extract = (BodyFormat,), Error = Rejection> + Clone {
	serde_qs::warp::query::<FormatQueryString>(serde_qs::Config::default())
		.map(|format_qs: FormatQueryString| format_qs.format)
}
