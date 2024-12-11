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

use std::{
	fmt,
	task::{Context, Poll},
};

use tower::{Service, ServiceExt};

use super::BoxFuture;

trait CloneService<R, T, E>:
	Service<R, Response = T, Error = E, Future = BoxFuture<T, E>>
	+ dyn_clone::DynClone
	+ Send
	+ Sync
	+ 'static
{
}

dyn_clone::clone_trait_object!(<R, T, E> CloneService<R, T, E>);

impl<S, R, T, E> CloneService<R, T, E> for S where
	S: Service<R, Response = T, Error = E, Future = BoxFuture<T, E>>
		+ Clone
		+ Send
		+ Sync
		+ 'static
{
}

pub struct BoxService<R, T, E> {
	inner: Box<dyn CloneService<R, T, E>>,
}

impl<R, T, E> Clone for BoxService<R, T, E> {
	fn clone(&self) -> Self {
		Self { inner: self.inner.clone() }
	}
}

impl<R, T, E> BoxService<R, T, E> {
	pub fn new<S>(inner: S) -> Self
	where
		S: Service<R, Response = T, Error = E> + Clone + Send + Sync + 'static,
		S::Future: Send + 'static,
	{
		let inner = Box::new(inner.map_future(|fut| Box::pin(fut) as _));
		BoxService { inner }
	}
}

impl<R, T, E> Service<R> for BoxService<R, T, E> {
	type Response = T;
	type Error = E;
	type Future = BoxFuture<T, E>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), E>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, request: R) -> BoxFuture<T, E> {
		self.inner.call(request)
	}
}

impl<T, U, E> fmt::Debug for BoxService<T, U, E> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		fmt.debug_struct("BoxService").finish()
	}
}
