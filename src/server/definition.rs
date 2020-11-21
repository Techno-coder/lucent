use lsp_types::*;

use crate::node::*;
use crate::parse::*;
use crate::query::{ESpan, MScope, QScope, S, Span};

use super::MScene;

pub fn definition(scene: MScene, request: GotoDefinitionParams)
				  -> crate::Result<Option<GotoDefinitionResponse>> {
	let position = request.text_document_position_params;
	let file = position.text_document.uri.to_file_path().unwrap();
	let position = &position.position;
	scope!(scope, scene);

	let module = &super::file_module(scope, &file)?;
	let table = &crate::parse::item_table(scope, module)?;
	let symbols = &crate::parse::symbols(scope, module)?;

	let mut locations = Vec::new();
	item_table(scope, position, table, symbols, &mut locations);
	Ok(Some(GotoDefinitionResponse::Array(locations)))
}

fn item_table(scope: QScope, position: &Position, table: &ItemTable,
			  symbols: &SymbolTable, locations: &mut Vec<Location>) {
	for (name, table) in &table.modules {
		let (_, symbols) = &symbols.modules[name];
		if let ModuleLocation::Inline(symbols) = symbols {
			item_table(scope, position, table, symbols, locations);
		}
	}

	for (_, span, value) in &table.global_annotations {
		locations.extend(TSpan::scope(*span, |base|
			self::value(scope, position, base, value, None)));
	}

	for (name, structure) in &table.structures {
		let base = &symbols.structures[name];
		locations.extend(annotations(scope, position, base, &structure.annotations)
			.or_else(|| variables(scope, position, base, &structure.fields)));
	}

	for (name, statics) in &table.statics {
		let base = &symbols.statics[name];
		locations.extend(match statics.as_ref() {
			PStatic::Local(local) =>
				annotations(scope, position, base, &local.annotations)
					.or_else(|| local.kind.as_ref().and_then(|kind|
						item_kind(scope, position, base, kind)))
					.or_else(|| local.value.as_ref().and_then(|value|
						self::value(scope, position, base, value, None))),
			PStatic::Load(load) =>
				annotations(scope, position, base, &load.annotations)
					.or_else(|| item_kind(scope, position, base, &load.kind))
					.or_else(|| library(scope, position, base, &load.library)),
		})
	}

	for (name, functions) in &table.functions {
		for (index, function) in functions.iter().enumerate() {
			let base = &symbols.functions[name][index];
			locations.extend(match function.as_ref() {
				PFunction::Local(local) =>
					annotations(scope, position, base, &local.annotations)
						.or_else(|| signature(scope, position, base, &local.signature))
						.or_else(|| value(scope, position, base, &local.value,
							Some(&local.signature.parameters))),
				PFunction::Load(load) =>
					annotations(scope, position, base, &load.annotations)
						.or_else(|| signature(scope, position, base, &load.signature))
						.or_else(|| library(scope, position, base, &load.library))
			});
		}
	}

	let base = &symbols.span;
	table.inclusions.wildcard.iter().for_each(|path|
		locations.extend(module_path(scope, position, base, path)));
	for path in table.inclusions.specific.values() {
		if let HPath::Node(module, name) = path {
			locations.extend(module_path(scope, position, base, module));
			if !contains(scope, position, TSpan::lift(base, name.span)) { continue; }
			let symbols = crate::parse::try_symbols(scope, &module.path());
			if let Some(symbols) = symbols {
				locations.extend(symbols.statics.contains_key(&name.node)
					.then(|| statics(scope, position, base, path)).flatten());
				locations.extend(symbols.structures.contains_key(&name.node)
					.then(|| structure(scope, position, base, path)).flatten());
				locations.extend(symbols.libraries.contains_key(&name.node)
					.then(|| library(scope, position, base, path)).flatten());

				if symbols.functions.contains_key(&name.node) {
					let path = (*path.path()).clone();
					let targets = crate::parse::functions(scope, &path).unwrap();
					for (index, target) in targets.iter().enumerate() {
						let target = match target.as_ref() {
							Universal::Local(local) => local.name.span,
							Universal::Load(load) => load.name.span,
						};

						let symbol = Symbol::Function(FPath(path.clone(), index));
						let target = ESpan::Item(symbol, target).lift(scope);
						locations.extend(location(scope, target));
					}
				}
			}
		}
	}
}

fn annotations(scope: MScope, position: &Position, base: &TSpan,
			   annotations: &HAnnotations) -> Option<Location> {
	annotations.iter().find_map(|(_, (_, value))|
		self::value(scope, position, base, value, None))
}

fn value(scope: MScope, position: &Position, base: &TSpan, value: &HValue,
		 parameters: Option<&HVariables>) -> Option<Location> {
	for node in value {
		let span = TSpan::lift(base, node.span);
		if !contains(scope, position, span) { continue; }
		let scope = &mut scope.span(span);
		return match &node.node {
			HNode::New(path, fields) => {
				structure(scope, position, base, path).or_else(|| {
					let path = path.path();
					let data = crate::parse::structure(scope, &path);
					let data = &data.unwrap().fields;

					for (name, (span, _)) in fields {
						let span = TSpan::lift(base, *span);
						if !contains(scope, position, span) { continue; }
						return data.get(name).and_then(|(span, _)| {
							let symbol = Symbol::Structure((*path).clone());
							let target = ESpan::Item(symbol, *span).lift(scope);
							location(scope, target)
						});
					}
					None
				})
			}
			HNode::Variable(variable) => {
				if let Some(parameters) = parameters {
					for (name, (span, _)) in parameters {
						if &Variable(name.clone(), 0) == variable {
							let span = TSpan::lift(base, *span);
							return location(scope, span);
						}
					}
				}

				for other in value {
					if let HNode::Let(other, _, _) = &other.node {
						if &other.node == variable {
							let span = TSpan::lift(base, other.span);
							return location(scope, span);
						}
					}
				}
				None
			}
			HNode::Function(path) | HNode::Call(path, _) =>
				item_path(scope, position, base, path, |scope, path| {
					// TODO: type information for function index
					let path = FPath(path.clone(), 0);
					let target = crate::parse::function(scope, &path);
					let target = match target.unwrap().as_ref() {
						Universal::Local(local) => local.name.span,
						Universal::Load(load) => load.name.span,
					};

					let symbol = Symbol::Function(path);
					let target = ESpan::Item(symbol, target).lift(scope);
					location(scope, target)
				}),
			HNode::Static(path) => statics(scope, position, base, path),
			HNode::Inline(value) | HNode::Compile(value) =>
				self::value(scope, position, base, value, None),
			HNode::Let(_, Some(kind), _) | HNode::Cast(_, Some(kind)) |
			HNode::SliceNew(kind, _) => item_kind(scope, position, base, kind),
			// TODO: type information for field structure
			HNode::Field(_, _field) => unimplemented!(),
			_ => None,
		};
	}
	None
}

fn statics(scope: QScope, position: &Position,
		   base: &TSpan, path: &HPath) -> Option<Location> {
	item_path(scope, position, base, path, |scope, path| {
		let target = crate::parse::statics(scope, path);
		let target = match target.unwrap().as_ref() {
			Universal::Local(local) => local.name.span,
			Universal::Load(load) => load.name.span,
		};

		let symbol = Symbol::Static(path.clone());
		let target = ESpan::Item(symbol, target).lift(scope);
		location(scope, target)
	})
}

fn library(scope: QScope, position: &Position,
		   base: &TSpan, path: &HPath) -> Option<Location> {
	item_path(scope, position, base, path, |scope, path| {
		let target = crate::parse::library(scope, path);
		let target = target.unwrap().name.span;
		let symbol = Symbol::Library(path.clone());
		let target = ESpan::Item(symbol, target).lift(scope);
		location(scope, target)
	})
}

fn structure(scope: QScope, position: &Position,
			 base: &TSpan, path: &HPath) -> Option<Location> {
	item_path(scope, position, base, path, |scope, path| {
		let target = crate::parse::structure(scope, path);
		let target = target.unwrap().name.span;
		let symbol = Symbol::Structure(path.clone());
		let target = ESpan::Item(symbol, target).lift(scope);
		location(scope, target)
	})
}

fn variables(scope: QScope, position: &Position,
			 base: &TSpan, variables: &HVariables) -> Option<Location> {
	for (span, kind) in variables.values() {
		let span = TSpan::lift(base, *span);
		if !contains(scope, position, span) { continue; }
		return item_kind(scope, position, base, kind);
	}
	None
}

fn signature(scope: QScope, position: &Position, base: &TSpan,
			 signature: &HSignature) -> Option<Location> {
	item_kind(scope, position, base, &signature.return_type)
		.or_else(|| variables(scope, position, base, &signature.parameters))
}

fn item_kind(scope: QScope, position: &Position,
			 base: &TSpan, kind: &S<HType>) -> Option<Location> {
	match &kind.node {
		HType::Structure(path) => structure(scope, position, base, path),
		HType::Function(kind) => signature(scope, position, base, kind),
		HType::Array(kind, value) => {
			let location = self::value(scope, position, base, value, None);
			location.or_else(|| item_kind(scope, position, base, kind))
		}
		HType::Pointer(kind) | HType::Slice(kind) =>
			item_kind(scope, position, base, &kind),
		_ => None,
	}
}

fn item_path<F>(scope: QScope, position: &Position, base: &TSpan,
				path: &HPath, complete: F) -> Option<Location>
	where F: FnOnce(QScope, &Path) -> Option<Location> {
	match path {
		HPath::Root(_) => None,
		HPath::Node(module, name) => {
			let span = TSpan::lift(base, name.span);
			match contains(scope, position, span) {
				false => module_path(scope, position, base, module),
				true => complete(scope, &path.path()),
			}
		}
	}
}

fn module_path(scope: QScope, position: &Position, base: &TSpan,
			   path: &HPath) -> Option<Location> {
	item_path(scope, position, base, path, |scope, path| match path {
		Path::Root => None,
		Path::Node(module, name) => {
			let symbols = crate::parse::symbols(scope, &module);
			let (span, _) = symbols.unwrap().modules[name];
			location(scope, span)
		}
	})
}

fn location(scope: MScope, Span(span): Span) -> Option<Location> {
	span.map(|(file, (start, end))| super::file_location(scope.ctx, file, start..end))
}

fn contains(scope: MScope, position: &Position, Span(span): Span) -> bool {
	use codespan_lsp::position_to_byte_index;
	span.map(|(file, (start, end))| (start..end)
		.contains(&position_to_byte_index(&scope.ctx.files,
			file, position).unwrap())).unwrap_or(false)
}
