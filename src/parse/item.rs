use std::collections::HashMap;
use std::sync::Arc;

use indexmap::IndexMap;

use crate::node::*;
use crate::query::{E, MScope, QScope, S};

use super::{Inclusions, ModuleLocation, Node, Scene, TSpan};

pub type PFunction = Universal<HFunction, HLoadFunction>;
pub type PStatic = Universal<HStatic, HLoadStatic>;

#[derive(Debug)]
pub enum Universal<V, L> {
	Local(Arc<V>),
	Load(Arc<L>),
}

impl<V, L> Clone for Universal<V, L> {
	fn clone(&self) -> Self {
		use Universal::*;
		match self {
			Local(local) => Local(local.clone()),
			Load(load) => Load(load.clone()),
		}
	}
}

impl PFunction {
	pub fn signature(&self) -> &HSignature {
		match self {
			PFunction::Local(local) => &local.signature,
			PFunction::Load(load) => &load.signature,
		}
	}
}

/// Stores items contained within a module.
/// Only child items within the same file are
/// reachable from a given table.
#[derive(Debug)]
pub struct ItemTable {
	pub module: Arc<HModule>,
	pub modules: HashMap<Identifier, Arc<ItemTable>>,
	pub functions: HashMap<Identifier, Arc<Vec<PFunction>>>,
	pub structures: HashMap<Identifier, Arc<HData>>,
	pub statics: HashMap<Identifier, PStatic>,
	pub libraries: HashMap<Identifier, Arc<HLibrary>>,
	pub roots: Vec<(Identifier, FIndex)>,
	pub inclusions: Arc<Inclusions>,
}

impl ItemTable {
	pub fn new(module: HModule,
			   inclusions: Inclusions) -> Self {
		Self {
			module: Arc::new(module),
			modules: HashMap::new(),
			functions: HashMap::new(),
			structures: HashMap::new(),
			statics: HashMap::new(),
			libraries: HashMap::new(),
			inclusions: Arc::new(inclusions),
			roots: vec![],
		}
	}
}

pub fn local(scope: QScope, FLocal(path): &FLocal)
			 -> crate::Result<Arc<HFunction>> {
	match self::function(scope, path)? {
		PFunction::Local(local) => Ok(local.clone()),
		_ => panic!("function: {}, is not local", path),
	}
}

pub fn function(scope: QScope, FPath(path, index): &FPath) -> crate::Result<PFunction> {
	Ok(functions(scope, path)?.get(*index).cloned().unwrap_or_else(||
		panic!("function: {}, index: {}, does not exist", path, index)))
}

pub fn functions(scope: QScope, path: &Path) -> crate::Result<Arc<Vec<PFunction>>> {
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
	scope.ctx.module.inherit(scope, path.clone(),
		|scope| Ok(item_table(scope, path)?.module.clone()))
}

pub fn statics(scope: QScope, path: &Path) -> crate::Result<Arc<PStatic>> {
	scope.ctx.statics.inherit(scope, path.clone(), |scope| match path {
		Path::Root => panic!("invalid static path: {}", path),
		Path::Node(module, identifier) => {
			let statics = &item_table(scope, module)?.statics;
			statics.get(identifier).cloned().ok_or_else(|| E::error()
				.message(format!("undefined static variable: {}", path))
				.label(scope.span.label()).to(scope)).map(Arc::new)
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

pub fn item_table(scope: QScope, path: &Path) -> crate::Result<Arc<ItemTable>> {
	let inclusions = Inclusions::root(Arc::new(path.clone()));
	scope.ctx.item_table.inherit(scope, path.clone(), |scope| match path {
		Path::Root => {
			let symbols = super::symbols(scope, path)?;
			super::file_table(scope, &symbols, inclusions, &scope.ctx.root)
		}
		Path::Node(parent, name) => {
			let symbols = super::symbols(scope, parent)?;
			let (_, location) = symbols.modules.get(name).ok_or_else(||
				E::error().message(format!("undefined module: {}", path))
					.label(scope.span.label()).to(scope))?;

			match location {
				ModuleLocation::External(file) => {
					let symbols = super::symbols(scope, path)?;
					super::file_table(scope, &symbols, inclusions, file)
				}
				ModuleLocation::Inline(_) => {
					let table = item_table(scope, parent)?;
					Ok(table.modules[name].clone())
				}
			}
		}
	})
}

pub fn value(scope: QScope, VPath(symbol, index): &VPath) -> crate::Result<Arc<Value>> {
	let store = |store: &VStore| store[*index].clone();
	Ok(match symbol {
		Symbol::Module(path) => store(&module(scope, path)?.values),
		Symbol::Library(path) => store(&library(scope, path)?.values),
		Symbol::Static(path) => match statics(scope, path)?.as_ref() {
			PStatic::Local(local) => store(&local.values),
			PStatic::Load(load) => store(&load.values),
		}
		Symbol::Function(path) => match function(scope, path)? {
			PFunction::Local(local) => store(&local.values),
			PFunction::Load(load) => store(&load.values),
		}
		Symbol::Structure(path) => store(&structure(scope, path)?.values),
		Symbol::Global(name) => {
			let globals = super::global_annotations(scope)?;
			let global = globals.get(name).ok_or_else(|| E::error()
				.message(format!("undefined global annotation: {}", name))
				.label(scope.span.label()).to(scope))?;
			store(&global.values)
		}
	})
}

pub fn variables<'a>(scope: MScope, scene: &mut Scene, span: &TSpan,
					 node: &impl Node<'a>) -> crate::Result<HVariables> {
	let mut variables = IndexMap::new();
	for node in node.children().filter(|node| node.kind() == "variable") {
		let (name, kind) = variable(scope, scene, span, node)?;
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

pub fn variable<'a>(scope: MScope, scene: &mut Scene, span: &TSpan,
					node: impl Node<'a>) -> crate::Result<(S<Identifier>, S<HType>)> {
	let kind = node.field(scope, "type")?;
	let kind = super::kind(scope, scene, span, kind)?;
	Ok((node.identifier_span(scope, span)?, kind))
}
