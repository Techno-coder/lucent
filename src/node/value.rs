use std::ops::Index;
use std::sync::Arc;

use crate::query::S;

use super::{HNode, Symbol};

/// An identifier for nodes inside a `Value`.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct HIndex(usize);

/// A container for spanned nodes that are self enclosed
/// meaning all nodes referenced are within the value.
#[derive(Debug, PartialEq)]
pub struct Value {
	/// The index of root node of this value.
	pub root: HIndex,
	nodes: Vec<S<HNode>>,
}

impl Value {
	pub fn new<F>(function: F) -> Self
		where F: FnOnce(&mut Self) -> HIndex {
		let (root, nodes) = (HIndex(0), Vec::new());
		let mut value = Self { root, nodes };
		value.root = function(&mut value);
		value
	}

	pub fn insert(&mut self, node: S<HNode>) -> HIndex {
		let index = self.nodes.len();
		self.nodes.push(node);
		HIndex(index)
	}
}

impl Index<HIndex> for Value {
	type Output = S<HNode>;

	fn index(&self, HIndex(index): HIndex) -> &Self::Output {
		self.nodes.get(index).unwrap_or_else(||
			panic!("node index: {}, is invalid", index))
	}
}

impl<'a> IntoIterator for &'a Value {
	type Item = (HIndex, &'a S<HNode>);
	type IntoIter = ValueNodes<'a>;

	fn into_iter(self) -> Self::IntoIter {
		ValueNodes { value: self, index: 0 }
	}
}

pub struct ValueNodes<'a> {
	value: &'a Value,
	index: usize,
}

impl<'a> Iterator for ValueNodes<'a> {
	type Item = (HIndex, &'a S<HNode>);

	fn next(&mut self) -> Option<Self::Item> {
		let node = self.value.nodes.get(self.index)
			.map(|node| (HIndex(self.index), node))?;
		self.index += 1;
		Some(node)
	}
}

/// Uniquely references a `Value`.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct VPath(pub Symbol, pub VIndex);

/// Uniquely references a `Value` in a `VStore`.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct VIndex(usize);

/// A container for values. The graph
/// of `Value` references forms a tree.
///
/// The values are ordered such that values later
/// in the list will only depend on previously
/// encountered values. For convenience,
/// iteration occurs in reverse order.
#[derive(Debug, Default, PartialEq)]
pub struct VStore(Vec<Arc<Value>>);

impl VStore {
	pub fn insert(&mut self, value: Value) -> VIndex {
		let VStore(store) = self;
		let index = store.len();
		store.push(Arc::new(value));
		VIndex(index)
	}
}

impl Index<VIndex> for VStore {
	type Output = Arc<Value>;

	fn index(&self, VIndex(index): VIndex) -> &Self::Output {
		let VStore(store) = self;
		store.get(index).unwrap_or_else(||
			panic!("value index: {}, is invalid", index))
	}
}

impl<'a> IntoIterator for &'a VStore {
	type Item = (VIndex, &'a Value);
	type IntoIter = StoreValues<'a>;

	fn into_iter(self) -> Self::IntoIter {
		let VStore(store) = &self;
		let index = store.len();
		StoreValues { store, index }
	}
}

pub struct StoreValues<'a> {
	store: &'a Vec<Arc<Value>>,
	index: usize,
}

impl<'a> Iterator for StoreValues<'a> {
	type Item = (VIndex, &'a Value);

	fn next(&mut self) -> Option<Self::Item> {
		(self.index != 0).then(|| {
			self.index -= 1;
			let node = &self.store[self.index];
			(VIndex(self.index), node.as_ref())
		})
	}
}
