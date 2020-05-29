use codespan::FileId;
use tree_sitter::{Language, Node, Parser, Query};

use crate::context::Context;
use crate::node::{Annotation, Identifier, Path};
use crate::span::S;

use super::Symbols;

extern { fn tree_sitter_lucent() -> Language; }

#[derive(Debug, Clone)]
pub struct Source<'a> {
	pub file: FileId,
	pub path: &'a std::path::Path,
	pub text: &'a str,
}

pub fn parser() -> Parser {
	let mut parser = Parser::new();
	let language = unsafe { tree_sitter_lucent() };
	parser.set_language(language).unwrap();
	parser
}

pub fn errors() -> Query {
	let language = unsafe { tree_sitter_lucent() };
	Query::new(language, "(ERROR) @error").unwrap()
}

pub fn parse(context: &Context, path: &std::path::Path) {
	let symbols = Symbols::root(context, path);
}

fn module(context: &Context, symbols: &mut Symbols,
		  source: &Source, path: &Path, node: Node) {
	let path = path.push(field_identifier(source, node).node);
	let annotations = annotations(symbols, source, node);

	let mut items = Vec::new();
	let cursor = &mut node.walk();
	for node in node.children_by_field_name("item", cursor) {
		items.push(match node.kind() {
			"function" => super::function(context, symbols, source, &path, node),
			other => panic!("invalid item type: {}", other),
		})
	}
}

fn annotations(symbols: &mut Symbols, source: &Source, node: Node) -> Vec<Annotation> {
	let cursor = &mut node.walk();
	let _annotations = node.children_by_field_name("annotation", cursor);
	// TODO
	Vec::new()
}

pub fn field_identifier(source: &Source, node: Node) -> S<Identifier> {
	identifier(source, node.child_by_field_name("identifier").unwrap())
}

pub fn identifier(source: &Source, node: Node) -> S<Identifier> {
	let text = source.text[node.byte_range()].to_string();
	S::create(Identifier(text), node.byte_range(), source.file)
}

pub fn string(source: &Source, node: Node) -> S<String> {
	let text = &source.text[node.start_byte() + 1..node.end_byte() - 1];
	S::create(text.to_string(), node.byte_range(), source.file)
}
