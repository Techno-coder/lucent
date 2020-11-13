use std::fmt;
use std::sync::Arc;

use crate::query::S;

pub type Register = Identifier;
/// The calling convention for a function or call.
pub type Convention = Option<S<Identifier>>;
/// The overload index for functions with the same path.
pub type FIndex = usize;

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Identifier(pub Arc<str>);

impl fmt::Display for Identifier {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let Identifier(identifier) = self;
		write!(f, "{}", identifier)
	}
}

/// Represents a sequence of identifiers that uniquely
/// references an item. Note that the order is reversed
/// as the outermost module is the deepest path element.
#[derive(Clone, Hash, Eq, PartialEq)]
pub enum Path { Root, Node(Arc<Path>, Identifier) }

impl Path {
	/// Returns the first (deepest) identifier in this
	/// path. Note that this operation is `O(n)`.
	pub fn head(&self) -> Option<Identifier> {
		match self {
			Path::Root => None,
			Path::Node(parent, name) => match parent.as_ref() {
				Path::Root => Some(name.clone()),
				_ => parent.head(),
			}
		}
	}

	/// Returns this path excluding the head (deepest)
	/// identifier. Note that this operation is `O(n)`.
	pub fn tail(&self) -> Option<Path> {
		match self {
			Path::Root => None,
			Path::Node(parent, name) => Some(match parent.as_ref() {
				Path::Root => Path::Root,
				Path::Node(_, _) => {
					let parent = Arc::new(parent.tail().unwrap());
					Path::Node(parent, name.clone())
				}
			})
		}
	}

	pub fn append(&self, other: &Path) -> Path {
		match other {
			Path::Root => self.clone(),
			Path::Node(parent, name) => {
				let path = Arc::new(self.append(parent));
				Path::Node(path, name.clone())
			}
		}
	}
}

impl fmt::Display for Path {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Path::Root => Ok(()),
			Path::Node(parent, name) => match parent.as_ref() {
				Path::Node(_, _) => write!(f, "{}.{}", parent, name),
				Path::Root => write!(f, "{}", name),
			}
		}
	}
}

impl fmt::Debug for Path {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self)
	}
}

/// Identifies a function by their path and overload index.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FPath(pub Path, pub FIndex);

/// Uniquely references an item.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Symbol {
	Module(Path),
	Function(FPath),
	Static(Path),
	Library(Path),
}

impl Symbol {
	/// Returns the module containing this symbol.
	pub fn module(&self) -> &Path {
		match self {
			Symbol::Function(FPath(Path::Node(module, _), _)) => module,
			Symbol::Module(Path::Node(module, _)) => module,
			Symbol::Static(Path::Node(module, _)) => module,
			Symbol::Library(Path::Node(module, _)) => module,
			_ => panic!("invalid symbol: {:?}", self),
		}
	}
}

/// Represents a variable that may be shadowed.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Variable(pub Identifier, pub usize);

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Sign {
	Unsigned,
	Signed,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Unary {
	Not,
	Negate,
	Reference,
	Dereference,
}

/// The size of a value in bytes.
#[derive(Debug, Copy, Clone)]
pub struct Size(pub usize);

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Width {
	/// Byte (1 byte or 8 bits)
	B = 1,
	/// Word (2 bytes or 16 bits)
	W = 2,
	/// Double (4 bytes or 32 bits)
	D = 4,
	/// Quad (8 bytes or 64 bits)
	Q = 8,
}

impl Width {
	pub fn bytes(self) -> usize {
		self as usize
	}

	pub fn bits(self) -> usize {
		self.bytes() * 8
	}
}

impl fmt::Display for Width {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.bits())
	}
}

/// References a symbol in a loaded library.
#[derive(Debug, PartialEq)]
pub enum LoadReference {
	Name(Identifier),
	Address(usize),
}

/// References the address of a function call.
pub enum Receiver<I> {
	Path(S<FPath>),
	Method(Convention, I),
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn path_head() {
		let head = Identifier("1".into());
		let path = Path::Node(Arc::new(Path::Root), head.clone());
		assert_eq!(path.head(), Some(head));
	}

	#[test]
	fn path_tail() {
		let root = Arc::new(Path::Root);
		let path = Path::Node(root.clone(), Identifier("1".into()));
		let path = Path::Node(Arc::new(path), Identifier("2".into()));
		let tail = Path::Node(root, Identifier("2".into()));
		assert_eq!(path.tail(), Some(tail));
	}
}
