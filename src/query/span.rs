use std::fmt;
use std::ops::Range;

use codespan::FileId;
use codespan_reporting::diagnostic;

use crate::parse::SymbolPath;

use super::Label;

/// Represents a fully resolved source code location.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Span(Option<(FileId, codespan::Span)>);

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
	Item(SymbolPath, ISpan),
}

impl ESpan {
	pub fn label(&self) -> Label {
		Label::new(diagnostic::LabelStyle::Primary, self.clone())
	}

	pub fn other(&self) -> Label {
		Label::new(diagnostic::LabelStyle::Secondary, self.clone())
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
