use std::collections::HashMap;
use std::sync::Arc;

use indexmap::IndexMap;

use crate::node::*;
use crate::query::{E, MScope, QScope, S, Span};

use super::{Inclusions, ModuleLocation, Node, TSpan};

pub type PFunction = Universal<HFunction, HLoadFunction>;
pub type PStatic = Universal<HStatic, HLoadStatic>;

#[derive(Debug)]
pub enum Universal<V, L> {
	Local(V),
	Load(L),
}

#[derive(Debug, Default)]
pub struct ItemTable {
	pub(super) global_annotations: Vec<(Identifier, Span, HValue)>,
	pub(super) modules: HashMap<Identifier, (Arc<HModule>, Arc<ItemTable>)>,
	pub(super) functions: HashMap<Identifier, Arc<Vec<Arc<PFunction>>>>,
	pub(super) structures: HashMap<Identifier, Arc<HData>>,
	pub(super) statics: HashMap<Identifier, Arc<PStatic>>,
	pub(super) libraries: HashMap<Identifier, Arc<HLibrary>>,
	pub(super) roots: Vec<(Identifier, FIndex)>,
}

pub fn function(scope: QScope, FPath(path, index): &FPath) -> crate::Result<Arc<PFunction>> {
	Ok(functions(scope, path)?.get(*index).cloned().unwrap_or_else(||
		panic!("function: {}, index: {}, does not exist", path, index)))
}

pub fn functions(scope: QScope, path: &Path) -> crate::Result<Arc<Vec<Arc<PFunction>>>> {
	scope.ctx.functions.inherit(scope, path.clone(), |scope| match path {
		Path::Root => panic!("invalid function path: {}", path),
		Path::Node(module, identifier) => {
			let functions = &item_table(scope, module)?.functions;
			functions.get(identifier).cloned().ok_or_else(|| E::error()
				.message(format!("undefined function: {}", path))
				.label(scope.span.label()).to(scope))
		}
	})
}

pub fn module(scope: QScope, path: &Path) -> crate::Result<Arc<HModule>> {
	scope.ctx.module.inherit(scope, path.clone(), |scope| match path {
		Path::Root => panic!("invalid module path: {}", path),
		Path::Node(module, identifier) => {
			let modules = &item_table(scope, module)?.modules;
			let module = modules.get(identifier).map(|(module, _)| module);
			module.cloned().ok_or_else(|| E::error()
				.message(format!("undefined module: {}", path))
				.label(scope.span.label()).to(scope))
		}
	})
}

pub fn statics(scope: QScope, path: &Path) -> crate::Result<Arc<PStatic>> {
	scope.ctx.statics.inherit(scope, path.clone(), |scope| match path {
		Path::Root => panic!("invalid static path: {}", path),
		Path::Node(module, identifier) => {
			let statics = &item_table(scope, module)?.statics;
			statics.get(identifier).cloned().ok_or_else(|| E::error()
				.message(format!("undefined static variable: {}", path))
				.label(scope.span.label()).to(scope))
		}
	})
}

pub fn library(scope: QScope, path: &Path) -> crate::Result<Arc<HLibrary>> {
	scope.ctx.library.inherit(scope, path.clone(), |scope| match path {
		Path::Root => panic!("invalid library path: {}", path),
		Path::Node(module, identifier) => {
			let libraries = &item_table(scope, module)?.libraries;
			libraries.get(identifier).cloned().ok_or_else(|| E::error()
				.message(format!("undefined library: {}", path))
				.label(scope.span.label()).to(scope))
		}
	})
}

pub fn structure(scope: QScope, path: &Path) -> crate::Result<Arc<HData>> {
	scope.ctx.structure.inherit(scope, path.clone(), |scope| match path {
		Path::Root => panic!("invalid structure path: {}", path),
		Path::Node(module, identifier) => {
			let structures = &item_table(scope, module)?.structures;
			structures.get(identifier).cloned().ok_or_else(|| E::error()
				.message(format!("undefined structure: {}", path))
				.label(scope.span.label()).to(scope))
		}
	})
}

fn item_table(scope: QScope, path: &Path) -> crate::Result<Arc<ItemTable>> {
	let inclusions = Inclusions::new(path.clone());
	scope.ctx.item_table.inherit(scope, path.clone(), |scope| match path {
		Path::Root => {
			let symbols = super::symbols(scope, path)?;
			super::file_table(scope, &symbols, inclusions, &scope.ctx.root)
		}
		Path::Node(parent, module) => {
			let symbols = super::symbols(scope, parent)?;
			let (_, location) = symbols.modules.get(module).ok_or_else(||
				E::error().message(format!("undefined module: {}", path))
					.label(scope.span.label()).to(scope))?;

			match location {
				ModuleLocation::External(path) =>
					super::file_table(scope, &symbols, inclusions, path),
				ModuleLocation::Inline(_) => {
					let table = item_table(scope, parent)?;
					let (_, table) = &table.modules[module];
					Ok(table.clone())
				}
			}
		}
	})
}

pub fn variables<'a>(scope: MScope, inclusions: &Inclusions, span: &TSpan,
					 node: &impl Node<'a>) -> crate::Result<HVariables> {
	let mut variables = IndexMap::new();
	for node in node.children().filter(|node| node.kind() == "variable") {
		let (name, kind) = variable(scope, inclusions, span, node)?;
		let (name, offset) = (name.node, name.span);
		match variables.get(&name) {
			None => variables.insert(name, (offset, kind)).unwrap_none(),
			Some((other, _)) => E::error().message("duplicate variable")
				.label(TSpan::lift(span, *other).label())
				.label(TSpan::lift(span, offset).other())
				.emit(scope),
		}
	}

	Ok(variables)
}

pub fn variable<'a>(scope: MScope, inclusions: &Inclusions, span: &TSpan,
					node: impl Node<'a>) -> crate::Result<(S<Identifier>, S<HType>)> {
	let kind = node.field(scope, "type")?;
	let kind = super::kind(scope, inclusions, span, kind)?;
	Ok((node.identifier_span(scope, span)?, kind))
}
