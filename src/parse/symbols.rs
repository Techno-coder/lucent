use std::collections::HashMap;
use std::path::PathBuf;

use tree_sitter::Node;

use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::{Identifier, Path};
use crate::query::Key;
use crate::span::{S, Span};

type Table = HashMap<Path, S<SymbolKind>>;

#[derive(Debug, PartialEq)]
pub enum SymbolKind {
	Function,
	Variable,
	Structure,
	Library,
}

#[derive(Debug)]
pub struct Symbols {
	stack: Vec<Table>,
}

impl Symbols {
	pub fn root(context: &Context, path: &std::path::Path) -> crate::Result<Symbols> {
		let mut table = HashMap::new();
		file(context, &mut table, path, &Path::default(), None)?;
		Ok(Symbols { stack: vec![table] })
	}
}

fn file(context: &Context, table: &mut Table, file: &std::path::Path,
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
		traverse(context, table, &source, path, tree.root_node())
	}).map(std::mem::drop)
}

fn traverse(context: &Context, table: &mut Table, source: &super::Source,
			path: &Path, node: Node) -> crate::Result<()> {
	let cursor = &mut node.walk();
	let mut nodes = node.children_by_field_name("item", cursor);
	nodes.try_for_each(|node| Ok(match node.kind() {
		"module" => traverse(context, table, source, &path
			.push(super::field_identifier(source, node).node), node)?,
		"function" => function(context, table, path,
			super::field_identifier(source, node)),
		"use" => {
			let node_path = node.child_by_field_name("path").unwrap();
			match node_path.kind() {
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
						self::file(context, table, file,
							&path, Some(other.span))?;
					} else if let Some(identifier) = path_as {
						duplicate(context, table, path,
							SymbolKind::Library, identifier);
					}
				}
				"path" => {
					let node = node.child_by_field_name("as");
					node.into_iter().for_each(|node| match node.kind() {
						"signature" => node.child_by_field_name("identifier")
							.into_iter().for_each(|node| function(context, table,
							path, super::identifier(source, node))),
						"static" => duplicate(context, table, path, SymbolKind::Variable,
							super::field_identifier(source, node)),
						_ => (),
					})
				}
				_ => (),
			}
		}
		"static" => duplicate(context, table, path,
			SymbolKind::Variable, super::field_identifier(source, node)),
		"data" => duplicate(context, table, path,
			SymbolKind::Structure, super::field_identifier(source, node)),
		_ => (),
	}))
}

fn function(context: &Context, table: &mut Table,
			path: &Path, identifier: S<Identifier>) {
	let path = path.push(identifier.node);
	if let Some(other) = table.get(&path) {
		if other.node != SymbolKind::Function {
			context.emit(Diagnostic::error()
				.message("duplicate symbol")
				.label(identifier.span.label())
				.label(other.span.label()));
		}
	} else {
		let symbol = S::new(SymbolKind::Function, identifier.span);
		table.insert(path, symbol);
	}
}

fn duplicate(context: &Context, table: &mut Table, path: &Path,
			 symbol: SymbolKind, identifier: S<Identifier>) {
	let path = path.push(identifier.node);
	match table.get(&path) {
		None => table.insert(path, S::new(symbol,
			identifier.span)).unwrap_none(),
		Some(other) => {
			context.emit(Diagnostic::error()
				.message("duplicate symbol")
				.label(identifier.span.label())
				.label(other.span.label()));
		}
	}
}
