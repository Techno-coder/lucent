use std::sync::Arc;

use crate::node::*;
use crate::parse::*;
use crate::query::{ISpan, QScope, S};

pub trait Visitor<'a> {
	fn scope<'b>(&'b mut self) -> QScope<'a, 'a, 'b>;
	fn table(&mut self, table: &ItemTable, symbols: &SymbolTable);
	fn value(&mut self, base: &TSpan, path: VPath, value: &Value,
			 parameters: Option<&HVariables>);
}

pub trait ReferenceVisitor<'a> {
	fn scope<'b>(&'b mut self) -> QScope<'a, 'a, 'b>;
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

impl<'a, T: ReferenceVisitor<'a>> Visitor<'a> for T {
	fn scope<'b>(&'b mut self) -> QScope<'a, 'a, 'b> { self.scope() }

	fn table(&mut self, table: &ItemTable, symbols: &SymbolTable) {
		for (name, structure) in &table.structures {
			let base = &symbols.structures[name];
			variables(self, base, &structure.fields);
		}

		for (name, statics) in &table.statics {
			let base = &symbols.statics[name];
			match statics.as_ref() {
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
				match function.as_ref() {
					PFunction::Local(local) =>
						signature(self, base, &local.signature),
					PFunction::Load(load) => {
						signature(self, base, &load.signature);
						self.library(base, &load.library);
					}
				}
			}
		}

		let base = &symbols.span;
		table.inclusions.wildcard.iter()
			.for_each(|path| self.module(base, path));
		table.inclusions.specific.values()
			.for_each(|path| self.path(base, path));
	}

	fn value(&mut self, base: &TSpan, _path: VPath,
			 value: &Value, parameters: Option<&HVariables>) {
		value.into_iter().for_each(|(_index, node)| match &node.node {
			HNode::Let(_, Some(kind), _) | HNode::Cast(_, Some(kind)) |
			HNode::SliceNew(kind, _) => self::kind(self, base, kind),
			HNode::Variable(variable) => self.variable(base,
				value, parameters, variable, &node.span),
			HNode::Static(path) => self.statics(base, path),
			HNode::Function(path) | HNode::Call(path, _) => {
				// TODO: type information for function index
				let index = 0;
				self.function(base, path, index);
			}
			HNode::New(path, fields) => {
				self.structure(base, path);
				let path = &path.path();
				for (name, (span, _)) in fields {
					self.field(base, path, name, span);
				}
			}
			HNode::Field(_value, _field) => {
				// TODO: type information for field structure
				let _scope = self.scope();
			}
			_ => (),
		})
	}
}

pub fn traverse<'a>(visitor: &mut impl Visitor<'a>,
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
		valued(visitor, base, symbol, match statics.as_ref() {
			PStatic::Local(local) => &local.values,
			PStatic::Load(load) => &load.values,
		})
	}

	for (name, functions) in &table.functions {
		let path = module.push(name.clone());
		for (index, function) in functions.iter().enumerate() {
			let base = &symbols.functions[name][index];
			let symbol = Symbol::Function(FPath(path.clone(), index));
			valued(visitor, base, symbol, match function.as_ref() {
				PFunction::Local(local) => &local.values,
				PFunction::Load(load) => &load.values,
			});
		}
	}
}

fn valued<'a>(visitor: &mut impl Visitor<'a>, base: &TSpan,
			  symbol: Symbol, values: &VStore) {
	for (index, value) in values {
		let path = VPath(symbol.clone(), index.clone());
		visitor.value(base, path, value, None);
	}
}

fn kind<'a>(visitor: &mut impl ReferenceVisitor<'a>,
			base: &TSpan, kind: &S<HType>) {
	visitor.kind(base, kind);
	match &kind.node {
		HType::Structure(path) => visitor.structure(base, path),
		HType::Function(kind) => signature(visitor, base, kind),
		HType::Pointer(kind) => self::kind(visitor, base, kind),
		HType::Slice(kind) => self::kind(visitor, base, kind),
		HType::Array(kind, _) => self::kind(visitor, base, kind),
		_ => (),
	}
}

fn signature<'a>(visitor: &mut impl ReferenceVisitor<'a>,
				 base: &TSpan, signature: &HSignature) {
	variables(visitor, base, &signature.parameters);
	kind(visitor, base, &signature.return_type);
}

fn variables<'a>(visitor: &mut impl ReferenceVisitor<'a>,
				 base: &TSpan, variables: &HVariables) {
	variables.values().for_each(|(_, kind)|
		self::kind(visitor, base, kind));
}
