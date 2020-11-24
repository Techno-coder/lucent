use crate::node::*;
use crate::parse::*;
use crate::query::{ISpan, QScope, S};

pub trait Visitor {
	fn table(&mut self, table: &ItemTable, symbols: &SymbolTable);
	fn value(&mut self, base: &TSpan, value: &HValue, parameters: Option<&HVariables>);
	fn kind(&mut self, base: &TSpan, kind: &S<HType>);
}

pub trait ReferenceVisitor<'a> {
	fn scope<'b>(&'b mut self) -> QScope<'a, 'a, 'b>;
	fn kind(&mut self, _base: &TSpan, _kind: &S<HType>) {}

	fn variable(&mut self, base: &TSpan, value: &HValue,
				parameters: Option<&HVariables>,
				variable: &Variable, span: &ISpan);
	fn field(&mut self, base: &TSpan, structure: &Path,
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

impl<'a, T: ReferenceVisitor<'a>> Visitor for T {
	fn table(&mut self, table: &ItemTable, symbols: &SymbolTable) {
		for (name, statics) in &table.statics {
			let base = &symbols.statics[name];
			if let PStatic::Load(load) = statics.as_ref() {
				self.library(base, &load.library);
			}
		}

		for (name, functions) in &table.functions {
			for (index, function) in functions.iter().enumerate() {
				let base = &symbols.functions[name][index];
				if let PFunction::Load(load) = function.as_ref() {
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

	fn value(&mut self, base: &TSpan, value: &HValue, parameters: Option<&HVariables>) {
		value.into_iter().for_each(|node| match &node.node {
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

	fn kind(&mut self, base: &TSpan, kind: &S<HType>) {
		self.kind(base, kind);
		if let HType::Structure(path) = &kind.node {
			self.structure(base, path);
		}
	}
}

pub fn traverse(visitor: &mut impl Visitor,
				table: &ItemTable, symbols: &SymbolTable) {
	visitor.table(table, symbols);
	for (name, table) in &table.modules {
		let (_, symbols) = &symbols.modules[name];
		if let ModuleLocation::Inline(symbols) = symbols {
			traverse(visitor, table, symbols);
		}
	}

	annotations(visitor, &symbols.span, &table.module.annotations);
	table.global_annotations.iter().for_each(|(_, span, value)|
		TSpan::scope(*span, |base| visitor.value(base, value, None)));

	for (name, structure) in &table.structures {
		let base = &symbols.structures[name];
		annotations(visitor, base, &structure.annotations);
		variables(visitor, base, &structure.fields);
	}

	for (name, statics) in &table.statics {
		let base = &symbols.statics[name];
		match statics.as_ref() {
			PStatic::Local(local) => {
				annotations(visitor, base, &local.annotations);
				local.kind.iter().for_each(|kind|
					self::kind(visitor, base, kind));
				local.value.iter().for_each(|value|
					valued(visitor, base, value, None));
			}
			PStatic::Load(load) => {
				annotations(visitor, base, &load.annotations);
				kind(visitor, base, &load.kind);
			}
		}
	}

	for (name, functions) in &table.functions {
		for (index, function) in functions.iter().enumerate() {
			let base = &symbols.functions[name][index];
			match function.as_ref() {
				PFunction::Local(local) => {
					annotations(visitor, base, &local.annotations);
					signature(visitor, base, &local.signature);
					valued(visitor, base, &local.value,
						Some(&local.signature.parameters));
				}
				PFunction::Load(load) => {
					annotations(visitor, base, &load.annotations);
					signature(visitor, base, &load.signature);
				}
			}
		}
	}
}

fn valued(visitor: &mut impl Visitor, base: &TSpan,
		  value: &HValue, parameters: Option<&HVariables>) {
	visitor.value(base, value, parameters);
	for node in value {
		match &node.node {
			HNode::Inline(value) => valued(visitor, base, value, None),
			HNode::Compile(value) => valued(visitor, base, value, None),
			HNode::Let(_, Some(kind), _) | HNode::Cast(_, Some(kind)) |
			HNode::SliceNew(kind, _) => self::kind(visitor, base, kind),
			_ => (),
		}
	}
}

fn kind(visitor: &mut impl Visitor, base: &TSpan, kind: &S<HType>) {
	visitor.kind(base, kind);
	match &kind.node {
		HType::Function(kind) => signature(visitor, base, kind),
		HType::Pointer(kind) => self::kind(visitor, base, kind),
		HType::Slice(kind) => self::kind(visitor, base, kind),
		HType::Array(kind, value) => {
			valued(visitor, base, value, None);
			self::kind(visitor, base, kind);
		}
		_ => (),
	}
}

fn annotations(visitor: &mut impl Visitor, base: &TSpan, annotations: &HAnnotations) {
	annotations.iter().for_each(|(_, (_, value))| visitor.value(base, value, None));
}

fn signature(visitor: &mut impl Visitor, base: &TSpan, signature: &HSignature) {
	variables(visitor, base, &signature.parameters);
	visitor.kind(base, &signature.return_type);
}

fn variables(visitor: &mut impl Visitor, base: &TSpan, variables: &HVariables) {
	variables.values().for_each(|(_, kind)| visitor.kind(base, kind));
}
