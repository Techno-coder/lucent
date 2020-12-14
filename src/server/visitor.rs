use std::sync::Arc;

use crate::inference::Types;
use crate::node::*;
use crate::parse::*;
use crate::query::{ISpan, QScope, S};

pub trait Visitor<'a: 'b, 'b: 'c, 'c> {
	fn scope<'d>(&'d mut self) -> QScope<'a, 'b, 'd>;
	fn table(&mut self, _table: &ItemTable, _symbols: &SymbolTable) {}
	/// Invoked on each local function present in the table.
	fn function(&mut self, base: &TSpan, path: &FLocal, function: &HFunction);
	/// Invoked on each value present in the table with the
	/// exception of function root values. Values belonging to a
	/// particular item are guaranteed to be traversed root first.
	fn value(&mut self, base: &TSpan, path: VPath, value: &Value);
}

pub trait ReferenceVisitor<'a: 'b, 'b: 'c, 'c> {
	fn scope<'d>(&'d mut self) -> QScope<'a, 'b, 'd>;
	fn kind(&mut self, _base: &TSpan, _kind: &S<HType>) {}
	fn variable(&mut self, base: &TSpan, value: &Value,
				parameters: Option<&HVariables>,
				variable: &Variable, span: &ISpan);
	fn field(&mut self, base: &TSpan, structure: &Arc<Path>,
			 name: &Identifier, span: &ISpan);

	/// Invoked on a path to a function.
	fn function(&mut self, base: &TSpan, path: &HPath, index: FIndex);
	/// Invoked on a path to a data structure.
	fn structure(&mut self, base: &TSpan, path: &HPath);
	/// Invoked on a path to a static variable.
	fn statics(&mut self, base: &TSpan, path: &HPath);
	/// Invoked on a path to a library.
	fn library(&mut self, base: &TSpan, path: &HPath);
	/// Invoked on a path to a module.
	fn module(&mut self, base: &TSpan, path: &HPath);
	/// Invoked on a path that is not specialized.
	fn path(&mut self, base: &TSpan, path: &HPath);
}

impl<'a: 'b, 'b: 'c, 'c, T: ReferenceVisitor<'a, 'b, 'c>> Visitor<'a, 'b, 'c> for T {
	fn scope<'d>(&'d mut self) -> QScope<'a, 'b, 'd> { self.scope() }

	fn table(&mut self, table: &ItemTable, symbols: &SymbolTable) {
		for (name, structure) in &table.structures {
			let base = &symbols.structures[name];
			variables(self, base, &structure.fields);
		}

		for (name, statics) in &table.statics {
			let base = &symbols.statics[name];
			match statics {
				PStatic::Local(local) => local.kind.iter()
					.for_each(|kind| self::kind(self, base, kind)),
				PStatic::Load(load) => {
					self::kind(self, base, &load.kind);
					self.library(base, &load.library);
				}
			}
		}

		for (name, functions) in &table.functions {
			for (index, function) in functions.iter().enumerate() {
				let base = &symbols.functions[name][index];
				signature(self, base, function.signature());
				if let PFunction::Load(load) = function {
					self.library(base, &load.library);
				}
			}
		}

		let base = &symbols.span;
		table.inclusions.wildcard.iter()
			.for_each(|path| self.module(base, path));
		table.inclusions.specific.values()
			.for_each(|path| self.path(base, path));
	}

	fn function(&mut self, base: &TSpan, path: &FLocal, function: &HFunction) {
		let parameters = Some(&function.signature.parameters);
		let types = crate::inference::function(self.scope(), path);
		self::value(self, base, types.ok(), &function.value, parameters);
	}

	fn value(&mut self, base: &TSpan, path: VPath, value: &Value) {
		let types = crate::inference::types(self.scope(), &path);
		self::value(self, base, types.ok(), value, None);
	}
}

pub fn traverse<'a: 'b, 'b: 'c, 'c>(visitor: &mut impl Visitor<'a, 'b, 'c>,
									table: &ItemTable, symbols: &SymbolTable) {
	visitor.table(table, symbols);
	let module = &table.inclusions.module;
	let symbol = Symbol::Module(module.clone());
	valued(visitor, &symbols.span, symbol, &table.module.values);

	for (name, table) in &table.modules {
		let (_, symbols) = &symbols.modules[name];
		if let ModuleLocation::Inline(symbols) = symbols {
			traverse(visitor, table, symbols);
		}
	}

	if module.as_ref() == &Path::Root {
		let table = crate::parse::global_annotations(visitor.scope());
		let annotations = table.iter().map(|table| table.iter());
		for (name, annotation) in annotations.flatten() {
			TSpan::scope(annotation.span, |base| valued(visitor,
				base, Symbol::Global(name.clone()), &annotation.values));
		}
	}

	for (name, structure) in &table.structures {
		let base = &symbols.structures[name];
		let symbol = Symbol::Structure(module.push(name.clone()));
		valued(visitor, base, symbol, &structure.values);
	}

	for (name, statics) in &table.statics {
		let base = &symbols.statics[name];
		let symbol = Symbol::Static(module.push(name.clone()));
		valued(visitor, base, symbol, match statics {
			PStatic::Local(local) => &local.values,
			PStatic::Load(load) => &load.values,
		})
	}

	for (name, functions) in &table.functions {
		let path = module.push(name.clone());
		for (index, function) in functions.iter().enumerate() {
			let path = FPath(path.clone(), index);
			let base = &symbols.functions[name][index];
			if let PFunction::Local(local) = function {
				let path = &FLocal(path.clone());
				visitor.function(base, path, &local);
			}

			let symbol = Symbol::Function(path);
			valued(visitor, base, symbol, match function {
				PFunction::Local(local) => &local.values,
				PFunction::Load(load) => &load.values,
			});
		}
	}
}

fn valued<'a: 'b, 'b: 'c, 'c>(visitor: &mut impl Visitor<'a, 'b, 'c>,
							  base: &TSpan, symbol: Symbol, values: &VStore) {
	for (index, value) in values {
		let path = VPath(symbol.clone(), index.clone());
		visitor.value(base, path, value);
	}
}

fn kind<'a: 'b, 'b: 'c, 'c>(visitor: &mut impl ReferenceVisitor<'a, 'b, 'c>,
							base: &TSpan, kind: &S<HType>) {
	visitor.kind(base, kind);
	match &kind.node {
		HType::Structure(path) => visitor.structure(base, path),
		HType::Function(kind) => signature(visitor, base, kind),
		HType::Pointer(_, kind) => self::kind(visitor, base, kind),
		HType::Slice(_, kind) => self::kind(visitor, base, kind),
		HType::Array(kind, _) => self::kind(visitor, base, kind),
		_ => (),
	}
}

fn signature<'a: 'b, 'b: 'c, 'c>(visitor: &mut impl ReferenceVisitor<'a, 'b, 'c>,
								 base: &TSpan, signature: &HSignature) {
	variables(visitor, base, &signature.parameters);
	kind(visitor, base, &signature.return_type);
}

fn variables<'a: 'b, 'b: 'c, 'c>(visitor: &mut impl ReferenceVisitor<'a, 'b, 'c>,
								 base: &TSpan, variables: &HVariables) {
	variables.values().for_each(|(_, kind)|
		self::kind(visitor, base, kind));
}

fn value<'a: 'b, 'b: 'c, 'c>(visitor: &mut impl ReferenceVisitor<'a, 'b, 'c>,
							 base: &TSpan, types: Option<Arc<Types>>, value: &Value,
							 parameters: Option<&HVariables>) {
	value.into_iter().for_each(|(index, node)| match &node.node {
		HNode::Let(_, Some(kind), _) | HNode::Cast(_, Some(kind)) |
		HNode::SliceNew(_, kind, _) => self::kind(visitor, base, kind),
		HNode::Variable(variable) => visitor.variable(base,
			value, parameters, variable, &node.span),
		HNode::Static(path) => visitor.statics(base, path),
		HNode::Function(paths) | HNode::Call(paths, _) =>
			drop(types.as_ref().map(|types| types
				.functions.get(&index).map(|index|
				visitor.function(base, paths, *index)))),
		HNode::New(path, fields) => {
			visitor.structure(base, path);
			let path = &path.path();
			for (name, (span, _)) in fields {
				visitor.field(base, path, name, span);
			}
		}
		HNode::Field(value, name) => drop(types.as_ref()
			.map(|types| types.nodes.get(value).map(|kind|
				if let RType::Structure(path) = &kind.node {
					visitor.field(base, path, &name.node, &name.span);
				}))),
		_ => (),
	})
}
