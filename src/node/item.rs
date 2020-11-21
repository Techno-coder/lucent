use std::fmt;
use std::sync::Arc;

use crate::query::S;

use super::Path;

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

/// Identifies a function by their path and overload index.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FPath(pub Path, pub FIndex);

/// Uniquely references an item.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Symbol {
	Module(Path),
	Function(FPath),
	Structure(Path),
	Static(Path),
	Library(Path),
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
