/// A change enum similar to `tower::discover::Change` but cloneable.
// TODO: Remove when the next version of tower (0.4.14?) is released.
#[derive(Debug, Clone)]
pub enum Change<K, V> {
	Insert(K, V),
	Remove(K),
}
