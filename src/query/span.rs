use std::fmt;
use std::ops::Range;

use codespan_reporting::diagnostic::LabelStyle;

use crate::node::{FPath, Identifier, Path, Symbol};
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

	pub fn label(&self) -> Label<ESpan> {
		ESpan::from(self.clone()).label()
	}

	pub fn other(&self) -> Label<ESpan> {
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
	pub fn label(&self) -> Label<Self> {
		Label::new(LabelStyle::Primary, *self)
	}

	pub fn other(&self) -> Label<Self> {
		Label::new(LabelStyle::Secondary, *self)
	}

	pub const fn internal() -> Self {
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
	pub fn label(&self) -> Label<Self> {
		Label::new(LabelStyle::Primary, self.clone())
	}

	pub fn other(&self) -> Label<Self> {
		Label::new(LabelStyle::Secondary, self.clone())
	}

	pub fn lift(self, scope: QScope) -> Span {
		match self {
			ESpan::Span(span) => span,
			ESpan::Item(symbol, span) => {
				if let Symbol::Global(name) = symbol {
					let table = crate::parse::global_annotations(scope).ok();
					let annotation = table.as_ref().and_then(|table| table.get(&name));
					let span = annotation.map(|annotation| annotation.span);
					return span.unwrap_or_else(|| Span::internal());
				}

				let module = match &symbol {
					Symbol::Global(_) => unreachable!(),
					Symbol::Module(path) => Some(path),
					Symbol::Function(FPath(path, _)) => path.parent(),
					Symbol::Structure(path) => path.parent(),
					Symbol::Static(path) => path.parent(),
					Symbol::Library(path) => path.parent(),
				}.unwrap_or_else(|| panic!("invalid symbol: {:?}", symbol));

				fn name(path: &Path) -> &Identifier {
					match path {
						Path::Node(_, name) => name,
						Path::Root => panic!("invalid symbol: {:?}", path),
					}
				}

				let table = crate::parse::symbols(scope, module);
				table.map(|table| TSpan::lift(match &symbol {
					Symbol::Global(_) => unreachable!(),
					Symbol::Module(_) => &table.span,
					Symbol::Function(FPath(path, index)) =>
						&table.functions[name(path)][*index],
					Symbol::Structure(path) => &table.structures[name(path)],
					Symbol::Static(path) => &table.statics[name(path)],
					Symbol::Library(path) => &table.libraries[name(path)],
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

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct S<T> {
	pub span: ISpan,
	pub node: T,
}

impl<T> S<T> {
	pub const fn new(node: T, span: ISpan) -> Self {
		S { node, span }
	}
}

impl<T> fmt::Debug for S<T> where T: fmt::Debug {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.node.fmt(f)
	}
}
