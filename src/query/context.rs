use std::collections::HashSet;

use crate::FilePath;

use super::{E, key, Key, Scope, Table};

/// A cache of values for a single
/// compilation target.
///
/// `Context`s cannot be shared between
/// compilation targets as two different items
/// may resolve to the same path.
// TODO: generate Context from macro
#[derive(Debug, Default)]
pub struct Context {
	pub root: FilePath,
	pub files: crate::source::FileCache,
	pub compile: Table<key::Compile>,
	pub file_table: Table<key::FileTable>,
	pub symbols: Table<key::Symbols>,
	pub item_table: Table<key::ItemTable>,
	pub globals: Table<key::GlobalAnnotations>,
	pub functions: Table<key::Functions>,
	pub statics: Table<key::Static>,
	pub structure: Table<key::Structure>,
	pub library: Table<key::Library>,
	pub module: Table<key::Module>,
	pub types: Table<key::Types>,
}

impl Context {
	pub fn new(root: FilePath) -> Self {
		Self { root, ..Self::default() }
	}

	pub fn errors(&self, scope: Scope) -> Vec<E> {
		let mut errors = scope.errors;
		let visited = &mut HashSet::new();
		scope.dependencies.into_iter().for_each(|key|
			self.key_errors(visited, &mut errors, key));
		errors
	}

	fn key_errors(&self, visited: &mut HashSet<Key>,
				  errors: &mut Vec<E>, key: Key) {
		let (other, dependencies) = match &key {
			Key::Read(key) => self.files.errors(key),
			Key::Compile(key) => self.compile.errors(key),
			Key::FileTable(key) => self.file_table.errors(key),
			Key::Symbols(key) => self.symbols.errors(key),
			Key::ItemTable(key) => self.item_table.errors(key),
			Key::GlobalAnnotations(key) => self.globals.errors(key),
			Key::Functions(key) => self.functions.errors(key),
			Key::Static(key) => self.statics.errors(key),
			Key::Structure(key) => self.structure.errors(key),
			Key::Library(key) => self.library.errors(key),
			Key::Module(key) => self.module.errors(key),
			Key::Types(key) => self.types.errors(key),
		};

		match visited.contains(&key) {
			false => visited.insert(key),
			true => return,
		};

		errors.extend(other.into_iter());
		dependencies.into_iter().for_each(|key|
			self.key_errors(visited, errors, key));
	}

	// TODO: recompute and compare for parse queries
	pub fn invalidate(&self, key: &Key) {
		match key {
			Key::Read(key) => self.files.invalidate(key),
			Key::Compile(key) => self.compile.invalidate(key),
			Key::FileTable(key) => self.file_table.invalidate(key),
			Key::Symbols(key) => self.symbols.invalidate(key),
			Key::ItemTable(key) => self.item_table.invalidate(key),
			Key::GlobalAnnotations(key) => self.globals.invalidate(key),
			Key::Functions(key) => self.functions.invalidate(key),
			Key::Static(key) => self.statics.invalidate(key),
			Key::Structure(key) => self.structure.invalidate(key),
			Key::Library(key) => self.library.invalidate(key),
			Key::Module(key) => self.module.invalidate(key),
			Key::Types(key) => self.types.invalidate(key),
		}.iter().for_each(|key| self.invalidate(key))
	}
}
