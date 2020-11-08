use std::collections::HashMap;
use std::sync::Arc;

use tree_sitter::{Query, QueryCursor};

use crate::FilePath;
use crate::node::{FIndex, Identifier, Path};
use crate::query::{E, ISpan, key, MScope, QScope, Span};

use super::Node;

/// Stores the names of items contained within a module.
/// The `PartialEq` implementation of this type is dependent
/// only on the semantic data of its nodes rather than their
/// source locations.
///
/// Tables must always be updated on a file change (due to
/// change in source locations) but invalidations only propagate
/// if semantic data is mutated.
#[derive(Debug, Default, PartialEq)]
pub struct SymbolTable {
	/// Contains an ordered list of location dependent symbols
	/// in this module. Used for order dependent operations
	/// such as code generation.
	pub symbols: Vec<SymbolKey>,
	pub modules: HashMap<Identifier, (TSpan, ModuleLocation)>,
	/// Contains a list of locations for
	/// functions with the same name.
	pub functions: HashMap<Identifier, Vec<TSpan>>,
	pub structures: HashMap<Identifier, TSpan>,
	pub statics: HashMap<Identifier, TSpan>,
	pub libraries: HashMap<Identifier, TSpan>,
}

/// A span transparent to equality. This should not
/// be stored outside of a symbol table as its range
/// is dependent on source files. Hence, it does not
/// implement `Clone`.
#[derive(Debug)]
pub struct TSpan(Span);

impl TSpan {
	pub fn offset(Self(span): Self, relative: Span) -> ISpan {
		Span::offset(span, relative)
	}

	pub fn lift(Self(span): Self, relative: ISpan) -> Span {
		Span::lift(span, relative)
	}
}

impl PartialEq for TSpan {
	fn eq(&self, _: &Self) -> bool { true }
}

/// Represents a child in a `SymbolTable` tree.
#[derive(Debug, PartialEq)]
pub enum SymbolKey {
	Module(Identifier),
	Function(Identifier, FIndex),
	Structure(Identifier),
	Static(Identifier),
	Library(Identifier),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum SymbolPath {
	Module(Path),
	Function(Path, FIndex),
	Structure(Path),
	Static(Path),
}

#[derive(Debug, PartialEq)]
pub enum ModuleLocation {
	Inline(Arc<SymbolTable>),
	External(FilePath),
}

pub fn symbols(parent: QScope, path: Path) -> crate::Result<Arc<SymbolTable>> {
	let key = key::Symbols(path.clone());
	let span = parent.span.clone();

	parent.ctx.symbols.scope(parent, key, |scope| {
		let scope = &mut scope.span(span.clone());
		let Path(mut path) = path;
		if path.is_empty() {
			let path = &scope.ctx.root;
			return file_symbols(scope, path);
		}

		let name = path.pop().unwrap();
		let table = symbols(scope, Path(path))?;
		match table.modules.get(&name) {
			Some((_, ModuleLocation::Inline(table))) => Ok(table.clone()),
			Some((_, ModuleLocation::External(path))) => file_symbols(scope, path),
			None => scope.result(E::error().label(span.label())
				.message(format!("undefined module: {}", name)))
		}
	})
}

fn file_symbols(parent: QScope, path: &FilePath) -> crate::Result<Arc<SymbolTable>> {
	let source = super::source(parent, path)?;
	let tree = super::parser().parse(source.text.as_bytes(), None).unwrap();
	inline_table(parent, path, Node::new(tree.root_node(), source.reference()))
}

fn inline_table(scope: MScope, path: &FilePath, node: Node) -> crate::Result<Arc<SymbolTable>> {
	let mut table = SymbolTable::default();
	let errors = &super::errors_query();
	for node in node.children() {
		let span = node.span();
		match node.kind() {
			"ERROR" => syntax_error(scope, span)?,
			"module" => {
				let name = node.identifier()?;
				if let Some((_, (TSpan(other), _))) = table.modules.get_key_value(&name) {
					scope.emit(E::error().message("duplicate module")
						.label(span.label()).label(other.other()));
				} else {
					let module = ModuleLocation::Inline(inline_table(scope, path, node)?);
					table.symbols.push(SymbolKey::Module(name.clone()));
					table.modules.insert(name, (TSpan(span), module));
				}
			}
			_ if !valid(scope, &node, errors) => (),
			"function" => {
				let name = node.identifier()?;
				let index = table.functions.get(&name).map(Vec::len).unwrap_or(0);
				table.symbols.push(SymbolKey::Function(name.clone(), index));
				table.functions.entry(name).or_default().push(TSpan(span));
			}
			"data" => {
				let name = node.identifier()?;
				if let Some(TSpan(other)) = table.structures.get(&name) {
					scope.emit(E::error().message("duplicate structure")
						.label(span.label()).label(other.other()));
				} else {
					table.symbols.push(SymbolKey::Structure(name.clone()));
					table.structures.insert(name, TSpan(span));
				}
			}
			"static" => {
				let name = node.identifier()?;
				if let Some(TSpan(other)) = table.statics.get(&name) {
					scope.emit(E::error().message("duplicate static variable")
						.label(span.label()).label(other.other()));
				} else {
					table.symbols.push(SymbolKey::Static(name.clone()));
					table.statics.insert(name, TSpan(span));
				}
			}
			"load" => load(scope, &mut table, node)?,
			"use" => import(scope, &mut table, path, node)?,
			"global_annotation" | "annotation" | "identifier" => (),
			other => panic!("invalid node kind: {}", other),
		}
	}

	Ok(Arc::new(table))
}

pub fn valid(scope: MScope, node: &Node, errors: &Query) -> bool {
	let mut cursor = QueryCursor::new();
	let captures = node.captures(&mut cursor, errors);
	captures.map(|node| syntax_error(scope, node.span())).last().is_none()
}

fn import(scope: MScope, table: &mut SymbolTable, path: &FilePath,
		  node: Node) -> crate::Result<()> {
	let target = node.field("use")?;
	let span = node.span();

	Ok(if let Some(other) = target.attribute("path") {
		let path = path.join(other.text());
		let name = target.attribute("as").as_ref().map(Node::text);
		let stem = path.file_stem().and_then(|name| name.to_str());
		let name = name.or(stem).ok_or_else(|| scope.emit(E::error()
			.message("invalid module name").label(other.span().label())
			.note("specify a name manually with: as <name>")));

		if let Ok(name) = name {
			let name = Identifier(name.to_owned());
			if let Some((_, (TSpan(other), _))) = table.modules.get_key_value(&name) {
				scope.emit(E::error().message("duplicate module")
					.label(span.label()).label(other.other()));
			} else {
				let module = ModuleLocation::External(path);
				table.symbols.push(SymbolKey::Module(name.clone()));
				table.modules.insert(name, (TSpan(span), module));
			}
		}
	})
}

/// Adds symbols for the `load` construct.
/// Symbols other than the library itself are not added to the
/// symbol sequence as their binary position is not affected by
/// their source location.
fn load(scope: MScope, table: &mut SymbolTable, node: Node) -> crate::Result<()> {
	let module = node.field("module")?;
	let span = node.span();
	Ok(match module.kind() {
		"string" => {
			// TODO: implement C header loading
			let name = node.field("name")?.text();
			let name = Identifier(name.to_owned());
			if let Some(TSpan(other)) = table.libraries.get(&name) {
				scope.emit(E::error().message("duplicate library")
					.label(span.label()).label(other.other()));
			} else {
				table.symbols.push(SymbolKey::Library(name.clone()));
				table.libraries.insert(name, TSpan(span));
			}
		}
		"identifier" => {
			let node = node.field("as")?;
			let name = node.identifier()?;
			match node.kind() {
				"static" => {
					if let Some(TSpan(other)) = table.statics.get(&name) {
						scope.emit(E::error().message("duplicate static variable")
							.label(span.label()).label(other.other()));
					} else {
						table.statics.insert(name, TSpan(span));
					}
				}
				"signature" => table.functions.entry(name)
					.or_default().push(TSpan(span)),
				other => panic!("invalid node kind: {}", other),
			}
		}
		other => panic!("invalid node kind: {}", other)
	})
}

fn syntax_error(scope: MScope, span: Span) -> crate::Result<()> {
	let error = E::error().message("syntax error");
	scope.result(error.label(span.label()))
}
