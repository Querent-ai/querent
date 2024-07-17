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
