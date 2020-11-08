use tree_sitter::{Query, QueryCursor, TreeCursor};

use crate::node::Identifier;
use crate::query::Span;

use super::PSource;

pub struct Node<'a> {
	node: tree_sitter::Node<'a>,
	source: PSource<'a>,
}

impl<'a> Node<'a> {
	pub fn new(node: tree_sitter::Node<'a>, source: PSource<'a>) -> Self {
		Self { node, source }
	}

	pub fn children(&self) -> NodeChildren {
		NodeChildren::new(self)
	}

	pub fn attribute(&self, attribute: &str) -> Option<Node<'a>> {
		let node = self.node.child_by_field_name(attribute)?;
		Some(Node::new(node, self.source))
	}

	pub fn text(&self) -> &'a str {
		&self.source.text[self.node.byte_range()]
	}

	pub fn kind(&self) -> &'a str {
		self.node.kind()
	}

	pub fn span(&self) -> Span {
		Span::new(self.source.file, self.node.byte_range())
	}
}

impl<'a> Node<'a> {
	pub fn field(&self, field: &str) -> crate::Result<Node<'a>> {
		Ok(self.attribute(field).unwrap_or_else(||
			panic!("field: {}, does not exist", field)))
	}

	pub fn identifier(&self) -> crate::Result<Identifier> {
		let text = self.field("name")?.text();
		Ok(Identifier(text.to_owned()))
	}
}

impl<'a> Node<'a> {
	pub fn captures(&'a self, cursor: &'a mut QueryCursor,
					query: &'a Query) -> impl Iterator<Item=Node<'a>> + 'a {
		let captures = cursor.captures(query, self.node,
			move |node| &self.source.text[node.byte_range()]);
		let captures = captures.flat_map(|(capture, _)| capture.captures);
		captures.map(move |capture| Node::new(capture.node, self.source))
	}
}

pub struct NodeChildren<'a, 'b> {
	cursor: TreeCursor<'a>,
	node: &'b Node<'a>,
	end: bool,
}

impl<'a, 'b> NodeChildren<'a, 'b> {
	fn new(node: &'b Node<'a>) -> Self {
		let mut cursor = node.node.walk();
		cursor.goto_first_child();
		Self { cursor, node, end: false }
	}

	fn advance(&mut self) {
		self.end = !self.cursor.goto_next_sibling();
	}
}

impl<'a, 'b> Iterator for NodeChildren<'a, 'b> {
	type Item = Node<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.end { return None; }
		let node = self.cursor.node();
		self.advance();

		match node.is_named() && !node.is_extra() {
			true => Some(Node::new(node, self.node.source)),
			false => self.next(),
		}
	}
}
