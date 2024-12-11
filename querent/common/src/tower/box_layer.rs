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

use std::{fmt, sync::Arc};

use tower::{layer::layer_fn, Layer, Service};

use crate::tower::BoxService;

pub struct BoxLayer<S, R, T, E> {
	inner: Arc<dyn Layer<S, Service = BoxService<R, T, E>> + Send + Sync + 'static>,
}

impl<S, R, T, E> BoxLayer<S, R, T, E> {
	pub fn new<L>(inner_layer: L) -> Self
	where
		L: Layer<S> + Send + Sync + 'static,
		L::Service: Service<R, Response = T, Error = E> + Clone + Send + Sync + 'static,
		<L::Service as Service<R>>::Future: Send + 'static,
	{
		let layer = layer_fn(move |inner_svc: S| {
			let outer_layer = inner_layer.layer(inner_svc);
			BoxService::new(outer_layer)
		});

		Self { inner: Arc::new(layer) }
	}
}

impl<S, R, T, E> Layer<S> for BoxLayer<S, R, T, E> {
	type Service = BoxService<R, T, E>;

	fn layer(&self, inner: S) -> Self::Service {
		self.inner.layer(inner)
	}
}

impl<S, R, T, E> Clone for BoxLayer<S, R, T, E> {
	fn clone(&self) -> Self {
		Self { inner: self.inner.clone() }
	}
}

impl<S, R, T, E> fmt::Debug for BoxLayer<S, R, T, E> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		fmt.debug_struct("BoxLayer").finish()
	}
}
