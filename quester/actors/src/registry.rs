use std::{
	any::{Any, TypeId},
	collections::HashMap,
	pin::Pin,
	sync::{Arc, RwLock},
	time::Duration,
};

use async_trait::async_trait;
use futures::{
	future::{self, Shared},
	Future, FutureExt,
};
use serde::Serialize;
use serde_json::Value as JsonValue;
use tokio::task::JoinHandle;

use crate::{
	command::Observe, messagebus::WeakMessagebus, Actor, ActorExitStatus, Command, MessageBus,
};

struct TypedJsonObservable<A: Actor> {
	actor_instance_id: String,
	weak_messagebus: WeakMessagebus<A>,
	join_handle: ActorJoinHandle,
}

#[async_trait]
trait JsonObservable: Sync + Send {
	fn is_disconnected(&self) -> bool;
	fn any(&self) -> &dyn Any;
	fn actor_instance_id(&self) -> &str;
	async fn observe(&self) -> Option<JsonValue>;
	async fn quit(&self) -> ActorExitStatus;
	async fn join(&self) -> ActorExitStatus;
}

#[async_trait]
impl<A: Actor> JsonObservable for TypedJsonObservable<A> {
	fn is_disconnected(&self) -> bool {
		self.weak_messagebus
			.upgrade()
			.map(|messagebus| messagebus.is_disconnected())
			.unwrap_or(true)
	}
	fn any(&self) -> &dyn Any {
		&self.weak_messagebus
	}
	fn actor_instance_id(&self) -> &str {
		self.actor_instance_id.as_str()
	}
	async fn observe(&self) -> Option<JsonValue> {
		let messagebus = self.weak_messagebus.upgrade()?;
		let oneshot_rx = messagebus.send_message_with_high_priority(Observe).ok()?;
		let state: <A as Actor>::ObservableState = oneshot_rx.await.ok()?;
		serde_json::to_value(&state).ok()
	}

	async fn quit(&self) -> ActorExitStatus {
		if let Some(messagebus) = self.weak_messagebus.upgrade() {
			let _ = messagebus.send_message_with_high_priority(Command::Quit);
		}
		self.join().await
	}

	async fn join(&self) -> ActorExitStatus {
		self.join_handle.join().await
	}
}

#[derive(Default, Clone)]
pub(crate) struct ActorRegistry {
	actors: Arc<RwLock<HashMap<TypeId, ActorRegistryForSpecificType>>>,
}

struct ActorRegistryForSpecificType {
	type_name: &'static str,
	observables: Vec<Arc<dyn JsonObservable>>,
}

impl ActorRegistryForSpecificType {
	fn for_type<A>() -> ActorRegistryForSpecificType {
		ActorRegistryForSpecificType {
			type_name: std::any::type_name::<A>(),
			observables: Vec::new(),
		}
	}

	fn gc(&mut self) {
		let mut i = 0;
		while i < self.observables.len() {
			if self.observables[i].is_disconnected() {
				self.observables.swap_remove(i);
			} else {
				i += 1;
			}
		}
	}
}

#[derive(Serialize, Debug)]
pub struct ActorObservation {
	pub type_name: &'static str,
	pub instance_id: String,
	pub obs: Option<JsonValue>,
}

impl ActorRegistry {
	pub fn register<A: Actor>(&self, messagebus: &MessageBus<A>, join_handle: ActorJoinHandle) {
		let typed_id = TypeId::of::<A>();
		let actor_instance_id = messagebus.actor_instance_id().to_string();
		let weak_messagebus = messagebus.downgrade();
		self.actors
			.write()
			.unwrap()
			.entry(typed_id)
			.or_insert_with(|| ActorRegistryForSpecificType::for_type::<A>())
			.observables
			.push(Arc::new(TypedJsonObservable {
				weak_messagebus,
				actor_instance_id,
				join_handle,
			}));
	}

	pub async fn observe(&self, timeout: Duration) -> Vec<ActorObservation> {
		self.gc();
		let mut obs_futures = Vec::new();
		for registry_for_type in self.actors.read().unwrap().values() {
			for obs in &registry_for_type.observables {
				if obs.is_disconnected() {
					continue;
				}
				let obs_clone = obs.clone();
				let type_name = registry_for_type.type_name;
				let instance_id = obs.actor_instance_id().to_string();
				obs_futures.push(async move {
					let obs =
						tokio::time::timeout(timeout, obs_clone.observe()).await.unwrap_or(None);
					ActorObservation { type_name, instance_id, obs }
				});
			}
		}
		future::join_all(obs_futures.into_iter()).await
	}

	pub fn get<A: Actor>(&self) -> Vec<MessageBus<A>> {
		let mut lock = self.actors.write().unwrap();
		get_iter::<A>(&mut lock).collect()
	}

	pub fn get_one<A: Actor>(&self) -> Option<MessageBus<A>> {
		let mut lock = self.actors.write().unwrap();
		let opt = get_iter::<A>(&mut lock).next();
		opt
	}

	fn gc(&self) {
		for registry_for_type in self.actors.write().unwrap().values_mut() {
			registry_for_type.gc();
		}
	}

	pub async fn quit(&self) -> HashMap<String, ActorExitStatus> {
		let mut obs_futures = Vec::new();
		let mut actor_ids = Vec::new();
		for registry_for_type in self.actors.read().unwrap().values() {
			for obs in &registry_for_type.observables {
				let obs_clone = obs.clone();
				obs_futures.push(async move { obs_clone.quit().await });
				actor_ids.push(obs.actor_instance_id().to_string());
			}
		}
		let res = future::join_all(obs_futures).await;
		actor_ids.into_iter().zip(res).collect()
	}

	pub fn is_empty(&self) -> bool {
		self.actors.read().unwrap().values().all(|registry_for_type| {
			registry_for_type.observables.iter().all(|obs| obs.is_disconnected())
		})
	}
}

fn get_iter<A: Actor>(
	actors: &mut HashMap<TypeId, ActorRegistryForSpecificType>,
) -> impl Iterator<Item = MessageBus<A>> + '_ {
	let typed_id = TypeId::of::<A>();
	actors
		.get(&typed_id)
		.into_iter()
		.flat_map(|registry_for_type| {
			registry_for_type
				.observables
				.iter()
				.flat_map(|box_any| box_any.any().downcast_ref::<WeakMessagebus<A>>())
				.flat_map(|weak_messagebus| weak_messagebus.upgrade())
		})
		.filter(|messagebus| !messagebus.is_disconnected())
}

/// This structure contains an optional exit handle. The handle is present
/// until the join() method is called.
#[derive(Clone)]
pub(crate) struct ActorJoinHandle {
	holder: Shared<Pin<Box<dyn Future<Output = ActorExitStatus> + Send>>>,
}

impl ActorJoinHandle {
	pub(crate) fn new(join_handle: JoinHandle<ActorExitStatus>) -> Self {
		ActorJoinHandle { holder: Self::inner_join(join_handle).boxed().shared() }
	}

	async fn inner_join(join_handle: JoinHandle<ActorExitStatus>) -> ActorExitStatus {
		join_handle.await.unwrap_or_else(|join_err| {
			if join_err.is_panic() {
				ActorExitStatus::Panicked
			} else {
				ActorExitStatus::Killed
			}
		})
	}

	/// Joins the actor and returns its exit status on the first invocation.
	/// Returns None afterwards.
	pub(crate) async fn join(&self) -> ActorExitStatus {
		self.holder.clone().await
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use crate::{tests::PingReceiverActor, Universe};

	#[tokio::test]
	async fn test_registry() {
		let test_actor = PingReceiverActor::default();
		let universe = Universe::with_accelerated_time();
		let (_messagebus, _handle) = universe.spawn_builder().spawn(test_actor);
		let _actor_messagebus = universe.get_one::<PingReceiverActor>().unwrap();
		universe.assert_quit().await;
	}

	#[tokio::test]
	async fn test_registry_killed_actor() {
		let test_actor = PingReceiverActor::default();
		let universe = Universe::with_accelerated_time();
		let (_messagebus, handle) = universe.spawn_builder().spawn(test_actor);
		handle.kill().await;
		assert!(universe.get_one::<PingReceiverActor>().is_none());
	}

	#[tokio::test]
	async fn test_registry_last_messagebus_dropped_actor() {
		let test_actor = PingReceiverActor::default();
		let universe = Universe::with_accelerated_time();
		let (messagebus, handle) = universe.spawn_builder().spawn(test_actor);
		drop(messagebus);
		handle.join().await;
		assert!(universe.get_one::<PingReceiverActor>().is_none());
	}

	#[tokio::test]
	async fn test_get_actor_states() {
		let test_actor = PingReceiverActor::default();
		let universe = Universe::with_accelerated_time();
		let (_messagebus, _handle) = universe.spawn_builder().spawn(test_actor);
		let obs = universe.observe(Duration::from_millis(1000)).await;
		assert_eq!(obs.len(), 1);
		universe.assert_quit().await;
	}
}
