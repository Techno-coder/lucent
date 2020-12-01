use std::collections::HashMap;
use std::sync::Arc;

use crate::FilePath;
use crate::node::{FIndex, Identifier, Path};
use crate::query::{E, ISpan, MScope, QScope, QueryError, Span};

use super::{Node, PSource, TreeNode};

/// Stores the names of items contained within a module.
/// The `PartialEq` implementation of this type is dependent
/// only on the semantic data of its nodes rather than their
/// source locations.
///
/// Tables must always be updated on a file change (due to
/// change in source locations) but invalidations only propagate
/// if semantic data is mutated.
#[derive(Debug, PartialEq)]
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
	pub span: TSpan,
}

impl SymbolTable {
	pub fn new(span: TSpan) -> Self {
		SymbolTable {
			symbols: vec![],
			modules: HashMap::new(),
			functions: HashMap::new(),
			structures: HashMap::new(),
			statics: HashMap::new(),
			libraries: HashMap::new(),
			span,
		}
	}
}

/// A span transparent to equality. This should not
/// be stored outside of a symbol table as its value
/// is volatile to source file changes. Hence, it
/// does not implement `Clone`.
#[derive(Debug)]
pub struct TSpan(Span);

impl TSpan {
	/// Converts a `Span` into a temporary `&TSpan`.
	pub fn scope<F, R>(span: Span, function: F) -> R where F: FnOnce(&TSpan) -> R {
		function(&TSpan(span))
	}

	pub fn offset(Self(span): &Self, relative: Span) -> ISpan {
		Span::offset(*span, relative)
	}

	pub fn lift(Self(span): &Self, relative: ISpan) -> Span {
		Span::lift(*span, relative)
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
	Static(Identifier),
	Library(Identifier),
}

#[derive(Debug, PartialEq)]
pub enum ModuleLocation {
	Inline(Arc<SymbolTable>),
	External(FilePath),
}

pub fn symbols(scope: QScope, path: &Path) -> crate::Result<Arc<SymbolTable>> {
	try_symbols(scope, path).ok_or_else(|| E::error()
		.message(format!("undefined module: {}", path))
		.label(scope.span.label()).to(scope))
}

pub fn try_symbols(parent: QScope, path: &Path) -> Option<Arc<SymbolTable>> {
	parent.ctx.symbols.inherit(parent, path.clone(), |scope| match path {
		Path::Root => file_symbols(scope, &scope.ctx.root),
		Path::Node(parent, name) => {
			let table = try_symbols(scope, parent);
			let modules = table.as_ref().and_then(|table| table.modules.get(&name));
			match modules.ok_or(QueryError::Failure)? {
				(_, ModuleLocation::External(path)) => file_symbols(scope, path),
				(_, ModuleLocation::Inline(table)) => Ok(table.clone()),
			}
		}
	}).ok()
}

fn file_symbols(parent: QScope, path: &FilePath) -> crate::Result<Arc<SymbolTable>> {
	let source = crate::source::source(parent, path)?;
	let tree = super::parser().parse(source.text.as_bytes(), None).unwrap();
	let (root, source) = (tree.root_node(), PSource::new(&source));

	let mut recurse = true;
	let mut cursor = root.walk();
	loop {
		let initial = recurse && cursor.goto_first_child();
		if initial || cursor.goto_next_sibling() {
			let node = TreeNode::new(cursor.node(), source);
			if cursor.node().is_error() {
				let error = E::error().message("syntax error");
				error.label(node.span().label()).emit(parent);
			} else if cursor.node().is_missing() {
				let kind = cursor.node().kind();
				let message = format!("missing '{}'", kind);
				let error = E::error().message(message);
				error.label(node.span().label()).emit(parent);
			} else { recurse = true; }
		} else if cursor.goto_parent() {
			recurse = false;
		} else { break; }
	}

	let root = TreeNode::new(root, source);
	inline_table(parent, path, root)
}

fn inline_table<'a>(scope: MScope, path: &FilePath, node: impl Node<'a>)
					-> crate::Result<Arc<SymbolTable>> {
	let span = TSpan(node.span());
	let mut table = SymbolTable::new(span);
	node.children().for_each(|node|
		drop(item(scope, path, &mut table, node)));
	Ok(Arc::new(table))
}

fn item<'a>(scope: MScope, path: &FilePath, table: &mut SymbolTable,
			node: impl Node<'a>) -> crate::Result<()> {
	let span = node.span();
	Ok(match node.kind() {
		"module" => {
			let name = node.identifier(scope)?;
			if let Some((TSpan(other), _)) = table.modules.get(&name) {
				E::error().label(span.label()).label(other.other())
					.message("duplicate module").emit(scope);
			} else {
				let module = inline_table(scope, path, node)?;
				let module = ModuleLocation::Inline(module);
				table.symbols.push(SymbolKey::Module(name.clone()));
				table.modules.insert(name, (TSpan(span), module));
			}
		}
		"use" => import(scope, table, path, node)?,
		"load" => load(scope, table, node)?,
		"function" => {
			let name = node.identifier(scope)?;
			let index = table.functions.get(&name).map(Vec::len).unwrap_or(0);
			table.symbols.push(SymbolKey::Function(name.clone(), index));
			table.functions.entry(name).or_default().push(TSpan(span));
		}
		"data" => {
			let name = node.identifier(scope)?;
			if let Some(TSpan(other)) = table.structures.get(&name) {
				E::error().label(span.label()).label(other.other())
					.message("duplicate structure").emit(scope);
			} else {
				table.structures.insert(name, TSpan(span));
			}
		}
		"static" => {
			let name = node.identifier(scope)?;
			if let Some(TSpan(other)) = table.statics.get(&name) {
				E::error().label(span.label()).label(other.other())
					.message("duplicate static variable").emit(scope);
			} else {
				table.symbols.push(SymbolKey::Static(name.clone()));
				table.statics.insert(name, TSpan(span));
			}
		}
		"global_annotation" | "annotation" => (),
		"identifier" => (), // Ignore module identifier.
		_ => node.invalid(scope)?,
	})
}

fn import<'a>(scope: MScope, table: &mut SymbolTable, path: &FilePath,
			  node: impl Node<'a>) -> crate::Result<()> {
	Ok(if let Some(other) = node.attribute("path") {
		let range = 1..other.text().len() - 1;
		let path = path.parent().unwrap().join(&other.text()[range]);
		let name = node.attribute("name").as_ref().map(Node::text);
		let stem = path.file_stem().and_then(|name| name.to_str());
		let name = name.or(stem).ok_or_else(|| E::error()
			.message("invalid module name").label(other.span().label())
			.note("specify the name with: as <name>").emit(scope));

		if let Ok(name) = name {
			let span = node.span();
			let name = Identifier(name.into());
			if let Some((TSpan(other), _)) = table.modules.get(&name) {
				E::error().label(span.label()).label(other.other())
					.message("duplicate module").emit(scope);
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
fn load<'a>(scope: MScope, table: &mut SymbolTable,
			node: impl Node<'a>) -> crate::Result<()> {
	let node = node.field(scope, "load")?;
	let module = node.field(scope, "module")?;
	let span = node.span();
	Ok(match module.kind() {
		"string" => {
			// TODO: implement C header loading
			let name = node.identifier(scope)?;
			if let Some(TSpan(other)) = table.libraries.get(&name) {
				E::error().label(span.label()).label(other.other())
					.message("duplicate library").emit(scope);
			} else {
				table.symbols.push(SymbolKey::Library(name.clone()));
				table.libraries.insert(name, TSpan(span));
			}
		}
		"path" => {
			let node = node.field(scope, "as")?;
			let name = node.identifier(scope)?;
			match node.kind() {
				"static" => {
					if let Some(TSpan(other)) = table.statics.get(&name) {
						E::error().label(span.label()).label(other.other())
							.message("duplicate static variable").emit(scope);
					} else {
						table.statics.insert(name, TSpan(span));
					}
				}
				"signature" => table.functions.entry(name)
					.or_default().push(TSpan(span)),
				_ => node.invalid(scope)?,
			}
		}
		_ => module.invalid(scope)?,
	})
}
