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

use std::{
	any::{Any, TypeId},
	collections::HashMap,
};

#[derive(Debug, Default)]
pub struct TMap(HashMap<TypeId, Box<dyn Any + Send + Sync>>);

impl TMap {
	pub fn contains<T: Any + Send + Sync>(&self) -> bool {
		self.0.contains_key(&TypeId::of::<T>())
	}

	pub fn insert<T: Any + Send + Sync>(&mut self, instance: T) {
		self.0.insert(TypeId::of::<T>(), Box::new(instance));
	}

	pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
		self.0
			.get(&TypeId::of::<T>())
			.map(|instance| instance.downcast_ref::<T>().expect("Instance should be of type T."))
	}

	pub fn get_mut<T: Any + Send + Sync>(&mut self) -> Option<&mut T> {
		self.0
			.get_mut(&TypeId::of::<T>())
			.map(|instance| instance.downcast_mut::<T>().expect("Instance should be of type T."))
	}
}
