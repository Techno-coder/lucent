use tree_sitter::TreeCursor;

use crate::node::Identifier;
use crate::query::{E, ISpan, MScope, QueryError, S, Span};

use super::{PSource, TSpan};

pub trait Node<'a>: Sized {
	type Children: Iterator<Item=Self>;

	fn children(&self) -> Self::Children;
	fn attribute(&self, attribute: &str) -> Option<Self>;
	fn text(&self) -> &'a str;
	fn kind(&self) -> &'a str;
	fn span(&self) -> Span;

	fn field(&self, scope: MScope, field: &str) -> crate::Result<Self> {
		self.attribute(field).ok_or_else(|| E::error()
			.message(format!("missing node field: {}", field))
			.label(self.span().label()).to(scope))
	}

	fn invalid<T>(&self, scope: MScope) -> crate::Result<T> {
		let message = format!("invalid node type: {}", self.kind());
		E::error().message(message).label(self.span().label()).result(scope)
	}

	fn identifier(&self, scope: MScope) -> crate::Result<Identifier> {
		let text = self.field(scope, "name")?.text();
		Ok(Identifier(text.into()))
	}

	fn identifier_span(&self, scope: MScope, span: &TSpan)
					   -> crate::Result<S<Identifier>> {
		let field = self.field(scope, "name")?;
		let name = Identifier(field.text().into());
		Ok(S::new(name, field.offset(span)))
	}

	fn offset(&self, span: &TSpan) -> ISpan {
		TSpan::offset(span, self.span())
	}
}

pub struct TreeNode<'a> {
	node: tree_sitter::Node<'a>,
	source: PSource<'a>,
}

impl<'a> TreeNode<'a> {
	pub fn new(node: tree_sitter::Node<'a>, source: PSource<'a>) -> Self {
		Self { node, source }
	}
}

impl<'a> Node<'a> for TreeNode<'a> {
	type Children = TreeNodeChildren<'a>;

	fn children(&self) -> Self::Children {
		TreeNodeChildren::new(self)
	}

	fn attribute(&self, attribute: &str) -> Option<Self> {
		let node = self.node.child_by_field_name(attribute)?;
		Some(Self::new(node, self.source))
	}

	fn text(&self) -> &'a str {
		&self.source.text[self.node.byte_range()]
	}

	fn kind(&self) -> &'a str {
		self.node.kind()
	}

	fn span(&self) -> Span {
		Span::new(self.source.file, self.node.byte_range())
	}

	fn field(&self, _: MScope, field: &str) -> crate::Result<Self> {
		self.attribute(field).ok_or(QueryError::Failure)
	}

	fn invalid<T>(&self, _: MScope) -> crate::Result<T> {
		// Syntax tree error nodes are processed during symbol
		// table generation so no diagnostic is emitted here.
		Err(QueryError::Failure)
	}
}

pub struct TreeNodeChildren<'a> {
	cursor: TreeCursor<'a>,
	source: PSource<'a>,
	end: bool,
}

impl<'a> TreeNodeChildren<'a> {
	fn new(node: &TreeNode<'a>) -> Self {
		let mut cursor = node.node.walk();
		cursor.goto_first_child();

		Self {
			cursor,
			source: node.source,
			end: false,
		}
	}

	fn advance(&mut self) {
		self.end = !self.cursor.goto_next_sibling();
	}
}

impl<'a> Iterator for TreeNodeChildren<'a> {
	type Item = TreeNode<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.end { return None; }
		let node = self.cursor.node();
		self.advance();

		match node.is_named() && !node.is_extra() {
			true => Some(TreeNode::new(node, self.source)),
			false => self.next(),
		}
	}
}
