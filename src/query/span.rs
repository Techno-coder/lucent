use std::fmt;
use std::ops::Range;

use codespan::FileId;
use codespan_reporting::diagnostic;

use crate::node::{FPath, Path, Symbol};
use crate::parse::TSpan;

use super::{Label, QScope};

/// Represents a fully resolved source code location.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Span(pub(super) Option<(FileId, codespan::Span)>);

impl Span {
	pub fn new(file: codespan::FileId, range: Range<usize>) -> Self {
		let range = range.start as u32..range.end as u32;
		Self(Some((file, range.into())))
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
		let ((_, span), (_, relative)) = Option::zip(span, relative)
			.expect("cannot take offset on internal spans");
		let range: Range<usize> = relative.into();
		let base = span.start().to_usize() as isize;
		let start = range.start as isize - base;
		let end = range.end as isize - base;
		ISpan(Some((start, end)))
	}

	pub fn lift(Self(span): Self, ISpan(relative): ISpan) -> Self {
		let (file, span) = span.expect("cannot lift with internal span");
		Self(relative.map(|(start, end)| (file, {
			let base = span.start().to_usize() as isize;
			let start = (base + start) as u32;
			let end = (base + end) as u32;
			(start..end).into()
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
				let module = symbol.module();
				let symbols = crate::parse::symbols(scope, module);
				symbols.map(|table| TSpan::lift(match &symbol {
					Symbol::Module(Path::Node(_, name)) =>
						&table.modules.get(name).map(|(span, _)| span).unwrap(),
					Symbol::Function(FPath(Path::Node(_, name), index)) =>
						&table.functions[name][*index],
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
