use std::{
	borrow::Borrow,
	convert::Infallible,
	fmt::{Display, Formatter},
	ops::{Add, Deref, Mul, Sub},
	str::FromStr,
};

use serde::{Deserialize, Serialize};

#[derive(
	Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Ord, PartialOrd, utoipa::ToSchema,
)]
#[serde(into = "CpuCapacityForSerialization", try_from = "CpuCapacityForSerialization")]
pub struct CpuCapacity(u32);

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum CpuCapacityForSerialization {
	Float(f32),
	MilliCpuWithUnit(String),
}

impl TryFrom<CpuCapacityForSerialization> for CpuCapacity {
	type Error = String;

	fn try_from(
		cpu_capacity_for_serialization: CpuCapacityForSerialization,
	) -> Result<CpuCapacity, Self::Error> {
		match cpu_capacity_for_serialization {
			CpuCapacityForSerialization::Float(cpu_capacity) =>
				Ok(CpuCapacity((cpu_capacity * 1000.0f32) as u32)),
			CpuCapacityForSerialization::MilliCpuWithUnit(cpu_capacity_str) =>
				Self::from_str(&cpu_capacity_str),
		}
	}
}

impl FromStr for CpuCapacity {
	type Err = String;

	fn from_str(cpu_capacity_str: &str) -> Result<Self, Self::Err> {
		let Some(milli_cpus_without_unit_str) = cpu_capacity_str.strip_suffix('m') else {
			return Err(format!(
				"invalid cpu capacity: `{cpu_capacity_str}`. String format expects a trailing 'm'."
			));
		};
		let milli_cpus: u32 = milli_cpus_without_unit_str
			.parse::<u32>()
			.map_err(|_err| format!("invalid cpu capacity: `{cpu_capacity_str}`."))?;
		Ok(CpuCapacity(milli_cpus))
	}
}

impl From<CpuCapacity> for CpuCapacityForSerialization {
	fn from(cpu_capacity: CpuCapacity) -> CpuCapacityForSerialization {
		CpuCapacityForSerialization::MilliCpuWithUnit(format!("{}m", cpu_capacity.0))
	}
}

impl CpuCapacity {
	#[inline(always)]
	pub const fn from_cpu_millis(cpu_millis: u32) -> CpuCapacity {
		CpuCapacity(cpu_millis)
	}

	#[inline(always)]
	pub fn cpu_millis(self) -> u32 {
		self.0
	}

	#[inline(always)]
	pub fn zero() -> CpuCapacity {
		CpuCapacity::from_cpu_millis(0u32)
	}

	#[inline(always)]
	pub fn one_cpu_thread() -> CpuCapacity {
		CpuCapacity::from_cpu_millis(1_000u32)
	}
}

impl Sub<CpuCapacity> for CpuCapacity {
	type Output = CpuCapacity;

	#[inline(always)]
	fn sub(self, rhs: CpuCapacity) -> Self::Output {
		CpuCapacity::from_cpu_millis(self.0 - rhs.0)
	}
}

impl Add<CpuCapacity> for CpuCapacity {
	type Output = CpuCapacity;

	#[inline(always)]
	fn add(self, rhs: CpuCapacity) -> Self::Output {
		CpuCapacity::from_cpu_millis(self.0 + rhs.0)
	}
}

impl Mul<u32> for CpuCapacity {
	type Output = CpuCapacity;

	#[inline(always)]
	fn mul(self, rhs: u32) -> CpuCapacity {
		CpuCapacity::from_cpu_millis(self.0 * rhs)
	}
}

impl Mul<f32> for CpuCapacity {
	type Output = CpuCapacity;

	#[inline(always)]
	fn mul(self, scale: f32) -> CpuCapacity {
		CpuCapacity::from_cpu_millis((self.0 as f32 * scale) as u32)
	}
}

impl Display for CpuCapacity {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		write!(f, "{}m", self.0)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(String);

// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub struct GenerationId(u64);

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub struct NodeUid(NodeId, GenerationId);

impl NodeId {
	/// Constructs a new [`NodeId`].
	pub const fn new(node_id: String) -> Self {
		Self(node_id)
	}

	/// Takes ownership of the underlying [`String`], consuming `self`.
	pub fn take(self) -> String {
		self.0
	}
}

impl AsRef<str> for NodeId {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

impl AsRef<NodeIdRef> for NodeId {
	fn as_ref(&self) -> &NodeIdRef {
		self.deref()
	}
}

impl Borrow<str> for NodeId {
	fn borrow(&self) -> &str {
		&self.0
	}
}

impl Borrow<String> for NodeId {
	fn borrow(&self) -> &String {
		&self.0
	}
}

impl Borrow<NodeIdRef> for NodeId {
	fn borrow(&self) -> &NodeIdRef {
		self.deref()
	}
}

impl Deref for NodeId {
	type Target = NodeIdRef;

	fn deref(&self) -> &Self::Target {
		NodeIdRef::from_str(&self.0)
	}
}

impl Display for NodeId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<&'_ str> for NodeId {
	fn from(node_id: &str) -> Self {
		Self::new(node_id.to_string())
	}
}

impl From<String> for NodeId {
	fn from(node_id: String) -> Self {
		Self::new(node_id)
	}
}

impl From<NodeId> for String {
	fn from(node_id: NodeId) -> Self {
		node_id.0
	}
}

impl From<&'_ NodeIdRef> for NodeId {
	fn from(node_id: &NodeIdRef) -> Self {
		node_id.to_owned()
	}
}

impl FromStr for NodeId {
	type Err = Infallible;

	fn from_str(node_id: &str) -> Result<Self, Self::Err> {
		Ok(NodeId::new(node_id.to_string()))
	}
}

impl PartialEq<&str> for NodeId {
	fn eq(&self, other: &&str) -> bool {
		self.as_str() == *other
	}
}

impl PartialEq<String> for NodeId {
	fn eq(&self, other: &String) -> bool {
		self.as_str() == *other
	}
}

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeIdRef(str);

impl NodeIdRef {
	/// Transparently reinterprets the string slice as a strongly-typed [`NodeIdRef`].
	pub const fn from_str(node_id: &str) -> &Self {
		let ptr: *const str = node_id;
		// SAFETY: `NodeIdRef` is `#[repr(transparent)]` around a single `str` field, so a `*const
		// str` can be safely reinterpreted as a `*const NodeIdRef`
		unsafe { &*(ptr as *const Self) }
	}

	/// Transparently reinterprets the static string slice as a strongly-typed [`NodeIdRef`].
	pub const fn from_static(node_id: &'static str) -> &'static Self {
		Self::from_str(node_id)
	}

	/// Provides access to the underlying value as a string slice.
	pub const fn as_str(&self) -> &str {
		&self.0
	}
}

impl AsRef<str> for NodeIdRef {
	fn as_ref(&self) -> &str {
		&self.0
	}
}

impl Borrow<str> for NodeIdRef {
	fn borrow(&self) -> &str {
		&self.0
	}
}

impl<'a> From<&'a str> for &'a NodeIdRef {
	fn from(node_id: &'a str) -> &'a NodeIdRef {
		NodeIdRef::from_str(node_id)
	}
}

impl PartialEq<NodeIdRef> for NodeId {
	fn eq(&self, other: &NodeIdRef) -> bool {
		self.as_str() == other.as_str()
	}
}

impl PartialEq<&'_ NodeIdRef> for NodeId {
	fn eq(&self, other: &&NodeIdRef) -> bool {
		self.as_str() == other.as_str()
	}
}

impl PartialEq<NodeId> for NodeIdRef {
	fn eq(&self, other: &NodeId) -> bool {
		self.as_str() == other.as_str()
	}
}

impl PartialEq<NodeId> for &'_ NodeIdRef {
	fn eq(&self, other: &NodeId) -> bool {
		self.as_str() == other.as_str()
	}
}

impl PartialEq<NodeId> for String {
	fn eq(&self, other: &NodeId) -> bool {
		self.as_str() == other.as_str()
	}
}

impl ToOwned for NodeIdRef {
	type Owned = NodeId;

	fn to_owned(&self) -> Self::Owned {
		NodeId(self.0.to_string())
	}
}
