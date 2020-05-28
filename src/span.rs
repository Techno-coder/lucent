use std::fmt;
use std::ops::Range;

use codespan::FileId;
use codespan_reporting::diagnostic::Label;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Span {
	span: codespan::Span,
	file: FileId,
}

impl Span {
	pub fn label(&self) -> Label<FileId> {
		Label::primary(self.file, self.span)
	}

	pub fn other(&self) -> Label<FileId> {
		Label::secondary(self.file, self.span)
	}
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct S<T> {
	pub span: Span,
	pub node: T,
}

impl<T> S<T> {
	pub fn new(node: T, span: Span) -> Self {
		S { node, span }
	}

	pub fn create(node: T, range: Range<usize>, file: codespan::FileId) -> Self {
		let range = range.start as u32..range.end as u32;
		S { node, span: Span { span: range.into(), file } }
	}
}

impl<T> AsRef<T> for S<T> {
	fn as_ref(&self) -> &T {
		&self.node
	}
}

impl<T> AsMut<T> for S<T> {
	fn as_mut(&mut self) -> &mut T {
		&mut self.node
	}
}

impl<T> fmt::Debug for S<T> where T: fmt::Debug {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.node.fmt(f)
	}
}

impl<T> fmt::Display for S<T> where T: fmt::Display {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.node.fmt(f)
	}
}
