use std::fmt;

pub type Register = Identifier;
/// The calling convention for a function or call.
pub type Convention = Option<Identifier>;
/// The overload index for functions with the same path.
pub type FIndex = usize;

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Identifier(pub String);

impl fmt::Display for Identifier {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let Identifier(identifier) = self;
		write!(f, "{}", identifier)
	}
}

/// Represents a sequence of identifiers that uniquely
/// references an item. An empty path is valid.
#[derive(Default, Clone, Hash, Eq, PartialEq)]
pub struct Path(pub Vec<Identifier>);

impl fmt::Display for Path {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let Path(path) = self;
		match path.split_last() {
			None => Ok(()),
			Some((last, slice)) => {
				slice.iter().try_for_each(|identifier|
					write!(f, "{}.", identifier))?;
				write!(f, "{}", last)
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
#[derive(Debug, Default, Clone, Hash, Eq, PartialEq)]
pub struct FPath(pub Path, pub FIndex);

/// Uniquely references an item.
#[derive(Debug)]
pub enum Symbol {
	Module(Path),
	Function(FPath),
	Static(Path),
	Load(Path),
}

/// Represents a variable that may be shadowed.
/// Assumes no variable will be shadowed more than
/// `u16::max_value()` times.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Variable(pub Identifier, pub u16);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum Sign {
	Unsigned,
	Signed,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum Unary {
	Not,
	Negate,
	Reference,
	Dereference,
}

/// The size of a value in bytes.
#[derive(Debug)]
pub struct Size(pub usize);

#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
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
#[derive(Debug)]
pub enum LoadReference {
	Name(Identifier),
	Address(usize),
}
