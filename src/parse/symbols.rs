use std::collections::HashMap;

use tree_sitter::{Node, Query, QueryCursor, Tree};

use crate::arena::OwnedArena;
use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::{Identifier, Path};
use crate::query::Key;
use crate::span::{S, Span};

use super::Source;

pub enum Include {
	Wild(Path),
	Item(Path),
	As(Path, Identifier),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
	Function,
	Variable,
	Intrinsic,
	Structure,
	Library,
	Module,
}

pub enum Item<'a> {
	Item((Path, Source, Node<'a>)),
	ModuleEnd,
}

#[derive(Default)]
pub struct Symbols<'a> {
	pub items: Vec<Item<'a>>,
	table: HashMap<Path, S<SymbolKind>>,
	includes: Vec<Vec<S<Include>>>,
	trees: OwnedArena<'a, Tree>,
}

impl<'a> Symbols<'a> {
	pub fn root(context: &Context, path: &std::path::Path) -> crate::Result<Self> {
		let mut symbols = Symbols { includes: vec![Vec::new()], ..Symbols::default() };
		let internal = S::new(SymbolKind::Intrinsic, context.files.read().internal.clone());
		["size", "start", "end"].iter().map(|intrinsic| Identifier(intrinsic.to_string()))
			.map(|intrinsic| Path(vec![Identifier("Intrinsic".to_string()), intrinsic]))
			.for_each(|path| symbols.table.insert(path, internal.clone()).unwrap_none());
		file(context, &mut symbols, path, &Path::default(), None)?;
		Ok(symbols)
	}

	pub fn resolve(&self, context: &Context, path: &Path,
				   span: &Span) -> Option<(Path, &SymbolKind)> {
		let Path(elements) = path;
		let mut candidates = self.table.get(path)
			.map(|kind| ((path.clone(), &kind.node), kind.span.clone()))
			.into_iter().chain(self.includes.iter().rev()
			.flat_map(|includes| includes.iter().filter_map(|include| {
				let path = Path(match &include.node {
					Include::Wild(Path(other)) => other.iter()
						.chain(elements.iter()).cloned().collect(),
					Include::Item(Path(other)) => other.iter()
						.chain(elements.iter().skip(1)).cloned().collect(),
					Include::As(Path(other), identifier) if elements.len() == 1
						&& elements.first() == Some(identifier) => other.clone(),
					_ => return None,
				});

				self.table.get(&path).map(|kind|
					((path, &kind.node), include.span.clone()))
			}))).peekable();

		let (value, other) = candidates.next()?;
		match candidates.peek().is_some() {
			false => Some(value),
			true => {
				let diagnostic = Diagnostic::error()
					.message("ambiguous symbol resolution")
					.label(span.label()).label(other.other());
				context.pass(candidates.fold(diagnostic, |diagnostic, (_, other)|
					diagnostic.label(other.other()))).ok()
			}
		}
	}

	pub fn include(&mut self, include: S<Include>) {
		self.includes.last_mut().expect("symbol stack is empty").push(include);
	}

	pub fn push(&mut self) {
		self.includes.push(Vec::new());
	}

	pub fn pop(&mut self) {
		self.includes.pop().expect("symbol stack is empty");
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
	context.symbol_files.ephemeral(None, key, span.clone(), || {
		let file = file.parent().unwrap().to_owned();
		let tree = super::parser().parse(text.as_bytes(), None).unwrap();
		let source = super::Source { file: source, path: file, text };
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
		"module" => {
			let element = super::field_identifier(source, node);
			let path = path.push(element.node.clone());
			match symbols.table.get(&path) {
				None | Some(S { node: SymbolKind::Structure, .. }) => {
					symbols.items.push(Item::Item((path.clone(), source.clone(), node)));
					let symbol = S::new(SymbolKind::Module, element.span);
					symbols.table.insert(path.clone(), symbol);
					traverse(context, symbols, source, &path, node)?;
					symbols.items.push(Item::ModuleEnd);
				}
				Some(other) => context.emit(Diagnostic::error().message("duplicate symbol")
					.label(element.span.label()).label(other.span.label())),
			}
		}
		_ if !valid(context, source, errors, node) => (),
		"annotation" => symbols.items
			.push(Item::Item((path.clone(), source.clone(), node))),
		"function" => function(context, symbols, source, path,
			node, super::field_identifier(source, node)),
		"static" => duplicate(context, symbols, source, path, node,
			SymbolKind::Variable, super::field_identifier(source, node)),
		"data" => duplicate(context, symbols, source, path, node,
			SymbolKind::Structure, super::field_identifier(source, node)),
		"use" => import(context, symbols, source, path, node)?,
		_ => (),
	}))
}

fn valid(context: &Context, source: &super::Source,
		 errors: &Query, node: Node) -> bool {
	let mut error_cursor = QueryCursor::new();
	let captures = error_cursor.captures(errors, node,
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
				duplicate(context, symbols, source, path,
					node, SymbolKind::Library, identifier);
			}
		}
		"path" => match node.child_by_field_name("as") {
			Some(node_as) if node_as.kind() == "signature" =>
				node_as.child_by_field_name("identifier")
					.into_iter().for_each(|node_as| function(context, symbols,
					source, path, node, super::identifier(source, node_as))),
			Some(node_as) if node_as.kind() == "static" =>
				duplicate(context, symbols, source, path, node,
					SymbolKind::Variable, super::field_identifier(source, node_as)),
			_ => symbols.items.push(Item::Item((path.clone(), source.clone(), node))),
		},
		_ => (),
	})
}

fn function<'a>(context: &Context, symbols: &mut Symbols<'a>, source: &Source,
				path: &Path, node: Node<'a>, identifier: S<Identifier>) {
	let path = path.push(identifier.node);
	if let Some(other) = symbols.table.get(&path) {
		if other.node != SymbolKind::Function {
			context.emit(Diagnostic::error().message("duplicate symbol")
				.label(identifier.span.label()).label(other.span.label()));
		}
	} else {
		symbols.items.push(Item::Item((path.clone(), source.clone(), node)));
		let symbol = S::new(SymbolKind::Function, identifier.span);
		symbols.table.insert(path, symbol);
	}
}

fn duplicate<'a>(context: &Context, symbols: &mut Symbols<'a>,
				 source: &Source, path: &Path, node: Node<'a>,
				 symbol: SymbolKind, identifier: S<Identifier>) {
	let path = path.push(identifier.node);
	match symbols.table.get(&path) {
		Some(other) => context.emit(Diagnostic::error().message("duplicate symbol")
			.label(identifier.span.label()).label(other.span.label())),
		None => {
			symbols.items.push(Item::Item((path.clone(), source.clone(), node)));
			symbols.table.insert(path, S::new(symbol, identifier.span));
		}
	}
}
