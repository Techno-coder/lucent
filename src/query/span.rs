use std::fmt;
use std::ops::Range;

use codespan_reporting::diagnostic;

use crate::node::{FPath, Path, Symbol};
use crate::parse::TSpan;
use crate::source::File;

use super::{Label, QScope};

/// Represents a fully resolved source code location.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Span(pub Option<(File, (usize, usize))>);

impl Span {
	pub fn new(file: File, range: Range<usize>) -> Self {
		Self(Some((file, (range.start, range.end))))
	}

	pub fn internal() -> Self {
		Self(None)
	}

	pub fn label(&self) -> Label {
		ESpan::from(self.clone()).label()
	}

	pub fn other(&self) -> Label {
		ESpan::from(self.clone()).other()
	}

	pub fn offset(Self(span): Self, Self(relative): Self) -> ISpan {
		let (_, (base, _)) = span.expect("cannot offset on internal span");
		ISpan(relative.map(|(_, (start, end))| {
			let start = start as isize - base as isize;
			let end = end as isize - base as isize;
			(start, end)
		}))
	}

	pub fn lift(Self(span): Self, ISpan(relative): ISpan) -> Self {
		let (file, (base, _)) = span.expect("cannot lift with internal span");
		Self(relative.map(|(start, end)| (file, {
			let start = (base as isize + start) as usize;
			let end = (base as isize + end) as usize;
			(start, end)
		})))
	}
}

/// Represents a span relative to an item declaration.
/// Designed to be used in tree nodes and item specific
/// queries. Offsets may be negative in locations such
/// as annotations.
///
/// `ISpan`s do not change if the entire item is moved.
/// This makes them ideal for incremental compilation.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ISpan(Option<(isize, isize)>);

impl ISpan {
	pub fn internal() -> Self {
		Self(None)
	}
}

/// Contains a span usable in general query
/// errors irrespective of origin.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ESpan {
	Span(Span),
	Item(Symbol, ISpan),
}

impl ESpan {
	pub fn label(&self) -> Label {
		Label::new(diagnostic::LabelStyle::Primary, self.clone())
	}

	pub fn other(&self) -> Label {
		Label::new(diagnostic::LabelStyle::Secondary, self.clone())
	}

	pub fn lift(self, scope: QScope) -> Span {
		match self {
			ESpan::Span(span) => span,
			ESpan::Item(symbol, span) => {
				let module = match &symbol {
					Symbol::Module(path) => path,
					Symbol::Function(FPath(Path::Node(parent, _), _)) => parent,
					Symbol::Structure(Path::Node(parent, _)) => parent,
					Symbol::Static(Path::Node(parent, _)) => parent,
					Symbol::Library(Path::Node(parent, _)) => parent,
					other => panic!("invalid symbol: {:?}", other),
				};

				let table = crate::parse::symbols(scope, module);
				table.map(|table| TSpan::lift(match &symbol {
					Symbol::Module(_) => &table.span,
					Symbol::Function(FPath(Path::Node(_, name), index)) =>
						&table.functions[name][*index],
					Symbol::Structure(Path::Node(_, name)) => &table.structures[name],
					Symbol::Static(Path::Node(_, name)) => &table.statics[name],
					Symbol::Library(Path::Node(_, name)) => &table.libraries[name],
					other => panic!("invalid symbol: {:?}", other),
				}, span)).unwrap_or_else(|_| Span::internal())
			}
		}
	}
}

impl From<Span> for ESpan {
	fn from(span: Span) -> Self {
		Self::Span(span)
	}
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct S<T> {
	pub span: ISpan,
	pub node: T,
}

impl<T> S<T> {
	pub fn new(node: T, span: ISpan) -> Self {
		S { node, span }
	}
}

impl<T> fmt::Debug for S<T> where T: fmt::Debug {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.node.fmt(f)
	}
}
