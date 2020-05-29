use std::collections::HashMap;

use tree_sitter::{Node, Query, QueryCursor, Tree};

use crate::arena::OwnedArena;
use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::{Identifier, Path};
use crate::query::Key;
use crate::span::{S, Span};

use super::Source;

type Table = HashMap<Path, S<SymbolKind>>;

#[derive(Debug, PartialEq)]
pub enum SymbolKind {
	Function,
	Variable,
	Structure,
	Library,
}

pub struct Symbols<'a> {
	stack: Vec<Table>,
	trees: OwnedArena<'a, Tree>,
	pub items: Vec<(Path, Node<'a>)>,
}

impl<'a> Symbols<'a> {
	pub fn root(context: &Context, path: &std::path::Path) -> crate::Result<Self> {
		let (trees, items) = (OwnedArena::default(), Vec::new());
		let mut symbols = Symbols { stack: vec![Table::new()], trees, items };
		file(context, &mut symbols, path, &Path::default(), None)?;
		Ok(symbols)
	}
}

fn file(context: &Context, symbols: &mut Symbols, file: &std::path::Path,
		path: &Path, span: Option<Span>) -> crate::Result<()> {
	let canonical = file.canonicalize().ok();
	let (source, text) = {
		let mut files = context.files.write();
		let canonical = canonical.as_ref()
			.and_then(|file| files.query(file));
		match canonical {
			Some(source) => source,
			None => {
				let mut diagnostic = Diagnostic::error()
					.message(format!("invalid file: {}", file.display()));
				if let Some(span) = span {
					let label = span.label().with_message("referenced here");
					diagnostic = diagnostic.label(label);
				}

				context.emit(diagnostic);
				return Ok(());
			}
		}
	};

	let key = Key::SymbolFile(canonical.unwrap());
	context.symbol_files.scope(None, key, span.clone(), || {
		let file = file.parent().unwrap();
		let source = super::Source { file: source, path: file, text: &text };
		let tree = super::parser().parse(text.as_bytes(), None).unwrap();
		let root_node = symbols.trees.push(tree).root_node();
		traverse(context, symbols, &source, path, root_node)
	}).map(std::mem::drop)
}

fn traverse<'a>(context: &Context, symbols: &mut Symbols<'a>, source: &Source,
				path: &Path, node: Node<'a>) -> crate::Result<()> {
	let (cursor, errors) = (&mut node.walk(), &super::errors());
	let mut nodes = node.named_children(cursor);
	nodes.try_for_each(|node: Node| Ok(match node.kind() {
		"ERROR" => syntax_error(context, source, node),
		"module" => traverse(context, symbols, source, &path
			.push(super::field_identifier(source, node).node), node)?,
		_ if !valid(context, source, errors, node) => (),
		"function" => function(context, symbols, path, node,
			super::field_identifier(source, node)),
		"static" => duplicate(context, symbols, path, node,
			SymbolKind::Variable, super::field_identifier(source, node)),
		"data" => duplicate(context, symbols, path, node,
			SymbolKind::Structure, super::field_identifier(source, node)),
		"use" => import(context, symbols, source, path, node)?,
		_ => (),
	}))
}

fn valid(context: &Context, source: &super::Source,
		 errors: &Query, node: Node) -> bool {
	let mut error_cursor = QueryCursor::new();
	let mut captures = error_cursor.captures(errors, node,
		|node| &source.text[node.byte_range()]);
	captures.flat_map(|(capture, _)| capture.captures)
		.map(|capture| syntax_error(context, source, capture.node))
		.last().is_none()
}

fn syntax_error(context: &Context, source: &Source, node: Node) {
	let label = Span::new(node.byte_range(), source.file).label();
	context.emit(Diagnostic::error().message("syntax error").label(label));
}

fn import<'a>(context: &Context, symbols: &mut Symbols<'a>, source: &Source,
			  path: &Path, node: Node<'a>) -> crate::Result<()> {
	let node_path = node.child_by_field_name("path").unwrap();
	Ok(match node_path.kind() {
		"string" => {
			let other = super::string(source, node_path);
			let file = &source.path.join(other.node);
			let extension = file.extension()
				.and_then(std::ffi::OsStr::to_str);
			let path_as = node.child_by_field_name("as")
				.map(|node| super::identifier(source, node));

			if extension == Some("lc") {
				let path = path_as.map(|node| path.push(node.node))
					.unwrap_or_else(|| path.clone());
				self::file(context, symbols, file,
					&path, Some(other.span))?;
			} else if let Some(identifier) = path_as {
				duplicate(context, symbols, path, node,
					SymbolKind::Library, identifier);
			}
		}
		"path" => {
			let node_as = node.child_by_field_name("as");
			node_as.into_iter().for_each(|node_as| match node_as.kind() {
				"signature" => node_as.child_by_field_name("identifier")
					.into_iter().for_each(|node_as| function(context, symbols,
					path, node, super::identifier(source, node_as))),
				"static" => duplicate(context, symbols, path, node,
					SymbolKind::Variable, super::field_identifier(source, node_as)),
				_ => (),
			})
		}
		_ => (),
	})
}

fn function<'a>(context: &Context, symbols: &mut Symbols<'a>,
				path: &Path, node: Node<'a>, identifier: S<Identifier>) {
	let table = symbols.stack.first_mut().unwrap();
	let path = path.push(identifier.node);
	if let Some(other) = table.get(&path) {
		if other.node != SymbolKind::Function {
			context.emit(Diagnostic::error()
				.message("duplicate symbol")
				.label(identifier.span.label())
				.label(other.span.label()));
		}
	} else {
		symbols.items.push((path.clone(), node));
		let symbol = S::new(SymbolKind::Function, identifier.span);
		table.insert(path, symbol);
	}
}

fn duplicate<'a>(context: &Context, symbols: &mut Symbols<'a>, path: &Path,
				 node: Node<'a>, symbol: SymbolKind, identifier: S<Identifier>) {
	let table = symbols.stack.first_mut().unwrap();
	let path = path.push(identifier.node);
	match table.get(&path) {
		None => {
			symbols.items.push((path.clone(), node));
			table.insert(path, S::new(symbol, identifier.span));
		}
		Some(other) => {
			context.emit(Diagnostic::error()
				.message("duplicate symbol")
				.label(identifier.span.label())
				.label(other.span.label()));
		}
	}
}
