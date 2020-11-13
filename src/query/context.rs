use crate::FilePath;

use super::{E, key, Key, Scope, Table};

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
	pub item_table: Table<key::ItemTable>,
	pub functions: Table<key::Functions>,
	pub statics: Table<key::Static>,
	pub structure: Table<key::Structure>,
	pub library: Table<key::Library>,
	pub module: Table<key::Module>,
}

impl Context {
	pub fn new(root: FilePath) -> Self {
		Self { root, ..Self::default() }
	}

	pub fn errors(&self, scope: Scope) -> Vec<E> {
		let mut errors = scope.errors;
		scope.dependencies.iter().for_each(|key|
			self.key_errors(&mut errors, key));
		errors
	}

	fn key_errors(&self, errors: &mut Vec<E>, key: &Key) {
		let (other, dependencies) = match key {
			Key::Compile(key) => self.compile.errors(key),
			Key::Source(key) => self.source.errors(key),
			Key::Symbols(key) => self.symbols.errors(key),
			Key::ItemTable(key) => self.item_table.errors(key),
			Key::Functions(key) => self.functions.errors(key),
			Key::Static(key) => self.statics.errors(key),
			Key::Structure(key) => self.structure.errors(key),
			Key::Library(key) => self.library.errors(key),
			Key::Module(key) => self.module.errors(key),
		};

		errors.extend(other.into_iter());
		dependencies.iter().for_each(|key|
			self.key_errors(errors, key));
	}

	fn key_invalidate(&self, key: &Key) -> Vec<Key> {
		match key {
			Key::Compile(key) => self.compile.invalidate(key),
			Key::Source(key) => self.source.invalidate(key),
			Key::Symbols(key) => self.symbols.invalidate(key),
			Key::ItemTable(key) => self.item_table.invalidate(key),
			Key::Functions(key) => self.functions.invalidate(key),
			Key::Static(key) => self.statics.invalidate(key),
			Key::Structure(key) => self.structure.invalidate(key),
			Key::Library(key) => self.library.invalidate(key),
			Key::Module(key) => self.module.invalidate(key),
		}
	}
}
