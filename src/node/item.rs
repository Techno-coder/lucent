use std::fmt;
use std::sync::Arc;

use crate::generate::Target;
use crate::query::S;

use super::Path;

pub type Register = Identifier;
/// The calling convention for a function or call.
pub type Convention = Option<S<Identifier>>;
/// The overload index for functions with the same path.
pub type FIndex = usize;
/// A fully resolved and evaluated type.
pub type RType = Type<Arc<Path>, Signature, usize, Option<Target>>;

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Identifier(pub Arc<str>);

impl AsRef<str> for Identifier {
	fn as_ref(&self) -> &str {
		let Identifier(string) = self;
		string.as_ref()
	}
}

impl fmt::Display for Identifier {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.as_ref())
	}
}

/// Identifies a function by their path and overload index.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FPath(pub Arc<Path>, pub FIndex);

impl fmt::Display for FPath {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let FPath(path, index) = self;
		write!(f, "{}:{}", path, index)
	}
}

/// Identifies a local function.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FLocal(pub FPath);

impl AsRef<FPath> for FLocal {
	fn as_ref(&self) -> &FPath {
		let FLocal(path) = self;
		path
	}
}

/// Uniquely references an item.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Symbol {
	Module(Arc<Path>),
	Library(Arc<Path>),
	Static(Arc<Path>),
	Function(FPath),
	Structure(Arc<Path>),
	/// Global annotation.
	Global(Identifier),
}

/// Represents a variable that may be shadowed.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Variable(pub Identifier, pub usize);

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Sign {
	Unsigned,
	Signed,
}

impl fmt::Display for Sign {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", match self {
			Sign::Unsigned => "u",
			Sign::Signed => "i",
		})
	}
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
#[derive(Debug)]
pub enum Receiver<N> {
	Path(S<FPath>),
	Method(Convention, N),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Signature {
	pub target: Option<Target>,
	pub convention: Convention,
	pub parameters: Vec<S<RType>>,
	pub return_type: S<RType>,
}

impl fmt::Display for Signature {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		prefix(f, &self.target)?;
		self.convention.iter().try_for_each(|convention|
			write!(f, "{} ", convention.node))?;
		write!(f, "fn(")?;

		if let Some((last, parameters)) = self.parameters.split_last() {
			parameters.iter().try_for_each(|parameter|
				write!(f, "{}, ", parameter.node))?;
			write!(f, "{}", last.node)?;
		}

		write!(f, ") {}", self.return_type.node)
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type<P, F, V, T> {
	Void,
	Rune,
	Truth,
	Never,
	Structure(P),
	Integral(Sign, Width),
	IntegralSize(T, Sign),
	Function(Box<F>),
	Pointer(T, Box<S<Self>>),
	Slice(T, Box<S<Self>>),
	Array(Box<S<Self>>, V),
}

impl fmt::Display for RType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Type::Void => write!(f, "void"),
			Type::Rune => write!(f, "rune"),
			Type::Truth => write!(f, "truth"),
			Type::Never => write!(f, "never"),
			Type::Structure(path) => write!(f, "{}", path),
			Type::Integral(sign, width) => write!(f, "{}{}", sign, width),
			Type::Function(signature) => write!(f, "{}", signature),
			Type::IntegralSize(target, sign) => {
				prefix(f, target)?;
				write!(f, "{}size", sign)
			}
			Type::Pointer(target, kind) => {
				prefix(f, target)?;
				write!(f, "*{}", kind.node)
			}
			Type::Array(kind, size) =>
				write!(f, "[{}; {}]", kind.node, size),
			Type::Slice(target, kind) => {
				prefix(f, target)?;
				write!(f, "[{};]", kind.node)
			}
		}
	}
}

pub fn prefix<T>(f: &mut fmt::Formatter, value: &Option<T>)
				 -> fmt::Result where T: fmt::Display {
	value.iter().try_for_each(|value| write!(f, "{} ", value))
}
