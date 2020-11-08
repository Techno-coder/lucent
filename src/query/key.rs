use std::fmt::Debug;
use std::hash::Hash;

use crate::FilePath;
use crate::node::Path;

macro_rules! queries {
    ($($name:ident $data:tt -> $value:ty;)*) => {
    	$(
			#[derive(Debug, Clone, Hash, Eq, PartialEq)]
			pub struct $name $data;

			impl From<$name> for Key {
				fn from(key: $name) -> Key {
					Key::$name(key)
				}
			}

			impl QueryKey for $name {
				type Value = $value;
			}
    	)*

		#[derive(Debug, Clone, Hash, Eq, PartialEq)]
    	pub enum Key {
    		$($name($name),)*
    	}
    };
}

queries! {
	Compile() -> ();
	Source(pub FilePath) -> codespan::FileId;
	Symbols(pub Path) -> crate::parse::SymbolTable;
}

pub trait QueryKey: Debug + Clone + Hash + Eq + PartialEq + Into<Key> {
	type Value: Debug;
}
