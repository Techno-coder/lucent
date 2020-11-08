use std::ops::Index;

use crate::query::S;

/// An identifier for nodes inside a `Value`.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct VIndex<T>(usize, std::marker::PhantomData<T>);

/// A container for spanned nodes that are self enclosed
/// meaning all nodes referenced are within the value.
/// Hence, any analysis performed on the value is not
/// dependent on where it is positioned.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Value<T> {
	pub root: VIndex<T>,
	nodes: Vec<S<T>>,
}

impl<T> Value<T> {
	pub fn insert(&mut self, node: S<T>) -> VIndex<T> {
		self.nodes.push(node);
		let index = self.nodes.len() - 1;
		VIndex(index, Default::default())
	}
}

impl<T> Index<VIndex<T>> for Value<T> {
	type Output = S<T>;

	fn index(&self, VIndex(index, _): VIndex<T>) -> &Self::Output {
		self.nodes.get(index).unwrap_or_else(||
			panic!("node index: {}, is invalid", index))
	}
}

impl<'a, T> IntoIterator for &'a Value<T> {
	type Item = &'a S<T>;
	type IntoIter = std::slice::Iter<'a, S<T>>;

	fn into_iter(self) -> Self::IntoIter {
		self.nodes.iter()
	}
}
