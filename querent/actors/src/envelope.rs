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

use std::{any::Any, fmt};

use async_trait::async_trait;
use tokio::sync::oneshot;

use crate::{
	actor::DeferableReplyHandler, scheduler::NoAdvanceTimeGuard, Actor, ActorContext,
	ActorExitStatus,
};

/// An `Envelope` is just a way to capture the handler
/// of a message and hide its type.
///
/// Messages can have different types but somehow need to be pushed to a
/// queue with a single type.
/// Before appending, we capture the right handler implementation
/// in the form of a `Box<dyn Envelope>`, and append that to the queue.

pub struct Envelope<A> {
	handler_envelope: Box<dyn EnvelopeT<A>>,
	_no_advance_time_guard: Option<NoAdvanceTimeGuard>,
}

impl<A: Actor> Envelope<A> {
	/// Returns the message as a boxed any.
	///
	/// This method is only useful in unit tests.
	pub fn message(&mut self) -> Box<dyn Any> {
		self.handler_envelope.message()
	}

	pub fn message_typed<M: 'static>(&mut self) -> Option<M> {
		if let Ok(boxed_msg) = self.handler_envelope.message().downcast::<M>() {
			Some(*boxed_msg)
		} else {
			None
		}
	}

	/// Execute the captured handle function.
	pub async fn handle_message(
		&mut self,
		actor: &mut A,
		ctx: &ActorContext<A>,
	) -> Result<(), ActorExitStatus> {
		self.handler_envelope.handle_message(actor, ctx).await?;
		Ok(())
	}
}

impl<A: Actor> fmt::Debug for Envelope<A> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let msg_str = self.handler_envelope.debug_msg();
		f.debug_tuple("Envelope").field(&msg_str).finish()
	}
}

#[async_trait]
trait EnvelopeT<A: Actor>: Send {
	fn debug_msg(&self) -> String;

	/// Returns the message as a boxed any.
	///
	/// This method is only useful in unit tests.
	fn message(&mut self) -> Box<dyn Any>;

	/// Execute the captured handle function.
	async fn handle_message(
		&mut self,
		actor: &mut A,
		ctx: &ActorContext<A>,
	) -> Result<(), ActorExitStatus>;
}

#[async_trait]
impl<A, M> EnvelopeT<A> for Option<(oneshot::Sender<A::Reply>, M)>
where
	A: DeferableReplyHandler<M>,
	M: fmt::Debug + Send + 'static,
{
	fn debug_msg(&self) -> String {
		#[allow(clippy::needless_option_take)]
		if let Some((_response_tx, msg)) = self.as_ref().take() {
			format!("{msg:?}")
		} else {
			"<consumed>".to_string()
		}
	}

	fn message(&mut self) -> Box<dyn Any> {
		if let Some((_, message)) = self.take() {
			Box::new(message)
		} else {
			Box::new(())
		}
	}

	async fn handle_message(
		&mut self,
		actor: &mut A,
		ctx: &ActorContext<A>,
	) -> Result<(), ActorExitStatus> {
		let (response_tx, msg) = self.take().expect("handle_message should never be called twice.");
		actor
			.handle_message(
				msg,
				|response| {
					// A SendError is fine here. The caller just did not wait
					// for our response and dropped its Receiver channel.
					let _ = response_tx.send(response);
				},
				ctx,
			)
			.await?;
		Ok(())
	}
}

pub(crate) fn wrap_in_envelope<A, M>(
	msg: M,
	no_advance_time_guard: Option<NoAdvanceTimeGuard>,
) -> (Envelope<A>, oneshot::Receiver<A::Reply>)
where
	A: DeferableReplyHandler<M>,
	M: fmt::Debug + Send + 'static,
{
	let (response_tx, response_rx) = oneshot::channel();
	let handler_envelope = Some((response_tx, msg));
	let envelope = Envelope {
		handler_envelope: Box::new(handler_envelope),
		_no_advance_time_guard: no_advance_time_guard,
	};
	(envelope, response_rx)
}
