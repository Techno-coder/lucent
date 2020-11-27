use std::sync::Arc;

use lsp_types::*;

use crate::node::*;
use crate::parse::*;
use crate::query::{ESpan, ISpan, QScope, Span};

use super::{MScene, ReferenceVisitor};

pub fn definition(scene: MScene, request: GotoDefinitionParams)
				  -> crate::Result<Option<GotoDefinitionResponse>> {
	let position = request.text_document_position_params;
	let file = position.text_document.uri.to_file_path().unwrap();
	let position = position.position;
	scope!(scope, scene);

	let module = &super::file_module(scope, &file)?;
	let symbols = &crate::parse::symbols(scope, module)?;
	let table = &crate::parse::item_table(scope, module)?;

	let mut visitor = Definitions { scope, locations: vec![], position };
	super::traverse(&mut visitor, table, symbols);
	Ok(Some(GotoDefinitionResponse::Array(visitor.locations)))
}

struct Definitions<'a> {
	scope: QScope<'a, 'a, 'a>,
	locations: Vec<Location>,
	position: Position,
}

impl<'a> Definitions<'a> {
	fn item(&mut self, base: &TSpan, path: &HPath) -> bool {
		match path {
			HPath::Root(_) => false,
			HPath::Node(module, name) => {
				let span = TSpan::lift(base, name.span);
				let contains = self.contains(span);
				if !contains { self.module(base, module); }
				contains
			}
		}
	}

	fn location(&mut self, Span(span): Span) {
		self.locations.extend(span.map(|(file, (start, end))|
			super::file_location(self.scope.ctx, file, start..end)));
	}

	fn contains(&self, Span(span): Span) -> bool {
		use codespan_lsp::position_to_byte_index;
		span.map(|(file, (start, end))| (start..end)
			.contains(&position_to_byte_index(&self.scope.ctx.files,
				file, &self.position).unwrap())).unwrap_or(false)
	}
}

impl<'a> ReferenceVisitor<'a> for Definitions<'a> {
	fn scope<'b>(&'b mut self) -> QScope<'a, 'a, 'b> { self.scope }

	fn variable(&mut self, base: &TSpan, value: &Value,
				parameters: Option<&HVariables>,
				variable: &Variable, span: &ISpan) {
		if !self.contains(TSpan::lift(base, *span)) { return; }
		if let Some(parameters) = parameters {
			for (name, (span, _)) in parameters {
				if &Variable(name.clone(), 0) == variable {
					self.location(TSpan::lift(base, *span));
				}
			}
		}

		for (_, other) in value {
			if let HNode::Let(other, _, _) = &other.node {
				if &other.node == variable {
					self.location(TSpan::lift(base, other.span));
				}
			}
		}
	}

	fn field(&mut self, base: &TSpan, structure: &Arc<Path>,
			 name: &Identifier, span: &ISpan) {
		if !self.contains(TSpan::lift(base, *span)) { return; }
		let data = crate::parse::structure(self.scope, structure);
		if let Some((span, _)) = data.unwrap().fields.get(name) {
			let symbol = Symbol::Structure(structure.clone());
			let target = ESpan::Item(symbol, *span).lift(self.scope);
			self.location(target);
		}
	}

	fn function(&mut self, base: &TSpan, path: &HPath, index: usize) {
		if !self.item(base, path) { return; }
		let path = FPath(path.path(), index);
		let target = crate::parse::function(self.scope, &path);
		let target = match target.unwrap().as_ref() {
			Universal::Local(local) => local.name.span,
			Universal::Load(load) => load.name.span,
		};

		let symbol = Symbol::Function(path);
		let target = ESpan::Item(symbol, target).lift(self.scope);
		self.location(target)
	}

	fn structure(&mut self, base: &TSpan, path: &HPath) {
		if !self.item(base, path) { return; }
		let path = path.path();
		let target = crate::parse::structure(self.scope, &path);

		let symbol = Symbol::Structure(path);
		let target = target.unwrap().name.span;
		let target = ESpan::Item(symbol, target).lift(self.scope);
		self.location(target)
	}

	fn statics(&mut self, base: &TSpan, path: &HPath) {
		if !self.item(base, path) { return; }
		let path = path.path();
		let target = crate::parse::statics(self.scope, &path);
		let target = match target.unwrap().as_ref() {
			Universal::Local(local) => local.name.span,
			Universal::Load(load) => load.name.span,
		};

		let symbol = Symbol::Static(path);
		let target = ESpan::Item(symbol, target).lift(self.scope);
		self.location(target)
	}

	fn library(&mut self, base: &TSpan, path: &HPath) {
		if !self.item(base, path) { return; }
		let path = path.path();
		let target = crate::parse::library(self.scope, &path);

		let symbol = Symbol::Library(path);
		let target = target.unwrap().name.span;
		let target = ESpan::Item(symbol, target).lift(self.scope);
		self.location(target)
	}

	fn module(&mut self, base: &TSpan, path: &HPath) {
		if self.item(base, path) {
			let path = &path.path();
			let symbols = crate::parse::symbols(self.scope, path).unwrap();
			let module = crate::parse::module(self.scope, path).unwrap();
			self.location(TSpan::lift(&symbols.span, module.span));
		}
	}

	fn path(&mut self, base: &TSpan, path: &HPath) {
		if !self.item(base, path) { return; }
		if let HPath::Node(module, name) = path {
			let symbols = crate::parse::symbols(self.scope, &module.path()).unwrap();
			symbols.statics.contains_key(&name.node).then(|| self.statics(base, path));
			symbols.structures.contains_key(&name.node).then(|| self.structure(base, path));
			symbols.libraries.contains_key(&name.node).then(|| self.library(base, path));

			if symbols.functions.contains_key(&name.node) {
				let path = path.path();
				let targets = crate::parse::functions(self.scope, &path);
				for (index, target) in targets.unwrap().iter().enumerate() {
					let target = match target.as_ref() {
						Universal::Local(local) => local.name.span,
						Universal::Load(load) => load.name.span,
					};

					let symbol = Symbol::Function(FPath(path.clone(), index));
					let target = ESpan::Item(symbol, target).lift(self.scope);
					self.location(target);
				}
			}
		}
	}
}