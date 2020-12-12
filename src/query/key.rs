use std::fmt::Debug;
use std::hash::Hash;

use crate::FilePath;
use crate::node::{Path, VPath, FLocal};

pub trait QueryKey: Debug + Clone + Hash + Eq + PartialEq + Into<Key> {
	type Value: Debug;
}

macro_rules! query {
    ($name:ident ($field:ty) -> $value:ty) => {
    	query!($name ($field,) -> $value);

    	impl From<$name> for $field {
			fn from($name(key): $name) -> $field { key }
    	}

    	impl From<$field> for $name {
			fn from(key: $field) -> $name { Self(key) }
    	}
    };

    ($name:ident ($($field:ty),* $(,)?) -> $value:ty) => {
		#[derive(Debug, Clone, Hash, Eq, PartialEq)]
		pub struct $name($(pub $field,)*);

		impl From<$name> for Key {
			fn from(key: $name) -> Key {
				Key::$name(key)
			}
		}

		impl QueryKey for $name {
			type Value = $value;
		}
    };
}

macro_rules! queries {
    ($($name:ident ($($field:ty),*) -> $value:ty;)*) => {
    	$(query!($name ($($field),*) -> $value);)*

		#[derive(Debug, Clone, Hash, Eq, PartialEq)]
    	pub enum Key {
    		$($name($name),)*
    	}
    };
}

queries! {
	Compile(()) -> ();
	Read(FilePath) -> crate::source::File;
	FileTable(FilePath) -> crate::server::FileTable;
	Symbols(Path) -> crate::parse::SymbolTable;

	ItemTable(Path) -> crate::parse::ItemTable;
	GlobalAnnotations(()) -> crate::node::GlobalAnnotations;
	Functions(Path) -> Vec<crate::parse::PFunction>;
	Static(Path) -> crate::parse::PStatic;
	Structure(Path) -> crate::node::HData;
	Library(Path) -> crate::node::HLibrary;
	Module(Path) -> crate::node::HModule;

	Types(VPath) -> crate::inference::Types;
	TypesFunction(FLocal) -> crate::inference::Types;
	LowFunction(FLocal) -> crate::node::LFunction;
	Low(VPath) -> crate::node::LNode;
}
