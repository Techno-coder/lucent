use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use codespan::FileId;
use indexmap::IndexMap;
use tree_sitter::{Language, Node, Parser, Query};

use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::*;
use crate::parse::Include;
use crate::span::S;

use super::{Symbols, Unit};

extern { fn tree_sitter_lucent() -> Language; }

#[derive(Debug, Clone)]
pub struct Source {
	pub file: FileId,
	pub path: PathBuf,
	pub text: Arc<str>,
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

pub fn parse(context: &Context, path: &std::path::Path) -> crate::Result<()> {
	let (mut symbols, units) = Symbols::root(context, path)?;
	units.iter().map(|unit| match unit {
		Unit::Item(path, source, node) => self::item(context,
			&mut symbols, path.clone(), source, *node),
		Unit::ModuleEnd => {
			context.items.write().push(Item::ModuleEnd);
			Ok(symbols.pop())
		}
	}).filter(Result::is_err).last().unwrap_or(Ok(()))
}

pub fn item(context: &Context, symbols: &mut Symbols, path: Path,
			source: &Source, node: Node) -> crate::Result<()> {
	Ok(match node.kind() {
		"function" => {
			let function = super::function(context, symbols, source, node)?;
			let functions = &mut context.functions.entry(path.clone()).or_default();
			let symbol = Symbol::Function(FunctionPath(path, functions.len()));
			context.items.write().push(Item::Symbol(symbol));
			functions.push(Arc::new(function));
		}
		"static" => {
			let identifier = field_identifier(source, node);
			let node_type = node.child_by_field_name("type").map(|node|
				super::node_type(context, symbols, source, node)).transpose()?;
			let value = node.child_by_field_name("value").map(|node|
				super::value(context, symbols, source, node)).transpose()?;
			let annotations = annotations(context, symbols, source, node);
			let variable = Static { annotations, identifier, node_type, value };

			let symbol = Symbol::Variable(path.clone());
			context.items.write().push(Item::Symbol(symbol));
			context.statics.insert(path, variable);
		}
		"use" => {
			let node_as = node.child_by_field_name("as");
			let S { node: Path(mut elements), span } = self::path(source,
				node.child_by_field_name("path").unwrap());

			symbols.include(S::create(match (elements.last().unwrap(), node_as) {
				(Identifier(string), Some(_)) if string == "*" => return
					context.pass(Diagnostic::error().label(span.label())
						.message("wildcard imports cannot be aliased")),
				(Identifier(string), None) if string == "*" => {
					elements.pop();
					Include::Wild(Path(elements))
				}
				(_, Some(node_as)) => Include::As(Path(elements),
					identifier(source, node_as).node),
				(_, None) => Include::Item(Path(elements)),
			}, node.byte_range(), source.file));
		}
		"data" => {
			let cursor = &mut node.walk();
			let mut fields = IndexMap::new();
			for node in node.children_by_field_name("field", cursor) {
				let identifier = field_identifier(source, node);
				let node_type = super::node_type(context, symbols,
					source, node.child_by_field_name("type").unwrap())?;
				match fields.get(&identifier.node) {
					None => fields.insert(identifier.node, node_type),
					Some(other) => return context.pass(Diagnostic::error()
						.label(other.span.label()).label(identifier.span.label())
						.message("duplicate field")),
				};
			}

			let annotations = annotations(context, symbols, source, node);
			context.structures.insert(path, Structure { annotations, fields });
		}
		"module" => {
			let symbol = Symbol::Module(path.clone());
			context.items.write().push(Item::Symbol(symbol));

			let identifier = field_identifier(source, node);
			let annotations = annotations(context, symbols, source, node);
			let module = Module { annotations, identifier, first: None, last: None };
			context.modules.insert(path.clone(), module);

			symbols.push();
			symbols.include(S::create(Include::Wild(path),
				node.byte_range(), source.file));
		}
		"annotation" => (), // TODO: implement global annotations
		other => panic!("invalid item kind: {}", other),
	})
}

pub fn annotations(context: &Context, symbols: &Symbols,
				   source: &Source, node: Node) -> Annotations {
	let cursor = &mut node.walk();
	let mut annotations: Annotations = HashMap::new();
	for node in node.children_by_field_name("annotation", cursor) {
		let name = node.child_by_field_name("name")
			.map(|node| identifier(source, node)).unwrap();
		if let Some(other) = annotations.get(&name.node) {
			context.emit(Diagnostic::error().message("duplicate annotation")
				.label(other.span.label()).label(name.span.label()));
		}

		let value = super::value(context, symbols, source,
			node.child_by_field_name("value").unwrap());
		if let Ok(value) = value {
			let value = S::new(value, name.span);
			annotations.insert(name.node, value);
		}
	}
	annotations
}

pub fn path(source: &Source, node: Node) -> S<Path> {
	let cursor = &mut node.walk();
	S::create(Path(node.named_children(cursor)
		.map(|node| identifier(source, node).node)
		.collect()), node.byte_range(), source.file)
}

pub fn field_identifier(source: &Source, node: Node) -> S<Identifier> {
	identifier(source, node.child_by_field_name("identifier").unwrap())
}

pub fn identifier(source: &Source, node: Node) -> S<Identifier> {
	let text = source.text[node.byte_range()].to_string();
	S::create(Identifier(text), node.byte_range(), source.file)
}

pub fn register(source: &Source, node: Node) -> S<Identifier> {
	let text = source.text[node.byte_range()][1..].to_string();
	S::create(Identifier(text), node.byte_range(), source.file)
}

pub fn string(source: &Source, node: Node) -> S<String> {
	let text = &source.text[node.start_byte() + 1..node.end_byte() - 1];
	S::create(text.to_string(), node.byte_range(), source.file)
}
