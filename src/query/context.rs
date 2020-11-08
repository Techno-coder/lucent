use crate::FilePath;
use crate::query::{key, Key, Table};

/// A cache of values for a single
/// compilation target.
///
/// `Context`s cannot be shared between
/// compilation targets as two different items
/// may resolve to the same path.
#[derive(Debug, Default)]
pub struct Context {
	pub root: FilePath,
	pub compile: Table<key::Compile>,
	pub source: crate::parse::Sources,
	pub symbols: Table<key::Symbols>,
}

impl Context {
	pub fn new(root: FilePath) -> Self {
		Self { root, ..Self::default() }
	}

	pub fn invalidate(&self, key: &Key) -> Vec<Key> {
		match key {
			Key::Compile(key) => self.compile.invalidate(key),
			Key::Source(key) => self.source.invalidate(key),
			Key::Symbols(key) => self.symbols.invalidate(key),
		}
	}
}
