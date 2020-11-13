use std::convert::TryFrom;
use std::sync::Arc;

use tree_sitter::{Language, Parser};

use crate::FilePath;
use crate::node::*;
use crate::query::{E, ISpan, MScope, QScope, S};

use super::*;

extern { fn tree_sitter_lucent() -> Language; }

pub fn parser() -> Parser {
	let mut parser = Parser::new();
	let language = unsafe { tree_sitter_lucent() };
	parser.set_language(language).unwrap();
	parser
}

pub fn file_table(scope: QScope, symbols: &SymbolTable, mut inclusions: Inclusions,
				  path: &FilePath) -> crate::Result<Arc<ItemTable>> {
	let source = super::source(scope, path)?;
	let tree = parser().parse(source.text.as_bytes(), None).unwrap();
	let root = TreeNode::new(tree.root_node(), source.reference());
	parse_table(scope, symbols, &mut inclusions, root, None)
}

pub fn parse_table<'a>(scope: MScope, symbols: &SymbolTable, inclusions: &mut Inclusions,
					   node: impl Node<'a>, mut module: Option<(&TSpan, &mut HModule)>)
					   -> crate::Result<Arc<ItemTable>> {
	let mut table = ItemTable::default();
	node.children().map(|node| Ok(match node.kind() {
		"module" => {
			let name = node.identifier(scope)?;
			let (span, location) = &symbols.modules[&name];
			if let ModuleLocation::Inline(symbols) = location {
				let mut module = HModule::default();
				let scope_module = Some((span, &mut module));
				let parsed = inclusions.scope(name.clone(), |inclusions|
					parse_table(scope, symbols, inclusions, node, scope_module))?;
				table.modules.insert(name, (Arc::new(module), parsed));
			}
		}
		"use" => if node.attribute("path").is_none() {
			let mut children = node.children();
			let mut target = Path::Root;
			let span = node.span();
			for child in &mut children {
				match child.kind() {
					"identifier" => {
						let name = Identifier(child.text().into());
						target = Path::Node(Arc::new(target), name);
					}
					"wildcard" => return match children.next().is_some() {
						true => E::error().message("wildcard must appear last")
							.label(span.label()).result(scope),
						false => Ok(inclusions.wildcard(target)),
					},
					_ => child.invalid(scope)?,
				}
			}

			let name = node.attribute("name");
			let name = name.map(|name| Identifier(name.text().into()));
			inclusions.specific(scope, name, span, target)?;
		},
		"load" => {
			let identifier = node.identifier(scope)?;
			let module = node.field(scope, "module")?;
			match module.kind() {
				"string" => {
					// TODO: implement C header loading
					let span = &symbols.libraries[&identifier];
					let annotations = annotations(scope, inclusions, span, &node)?;
					let library = HLibrary { annotations, path: module.text().into() };
					table.libraries.insert(identifier, Arc::new(library));
				}
				"path" => {
					let target = node.field(scope, "target")?;
					let reference = match target.kind() {
						"integral" => {
							let integral = super::integral(scope, &target)?;
							LoadReference::Address(usize::try_from(integral)
								.map_err(|_| E::error().label(target.span().label())
									.message("invalid library symbol address").to(scope))?)
						}
						"identifier" => {
							let identifier = Identifier(target.text().into());
							LoadReference::Name(identifier)
						}
						_ => return target.invalid(scope),
					};

					let symbol = node.field(scope, "as")?;
					match symbol.kind() {
						"static" => {
							let span = &symbols.statics[&identifier];
							let library = path(scope, span, &module)?;
							let annotations = annotations(scope, inclusions, span, &node)?;
							let (name, kind) = super::variable(scope, inclusions, span, node)?;
							let load = HLoadStatic { library, reference, annotations, name, kind };
							table.statics.insert(identifier, Arc::new(PStatic::Load(load)));
						}
						"signature" => {
							let entry = table.functions.entry(identifier.clone()).or_default();
							let span = &symbols.functions[&identifier][entry.len()];
							let annotations = annotations(scope, inclusions, span, &node)?;
							let signature = signature(scope, inclusions, span, &node)?;
							let name = symbol.identifier_span(scope, span)?;
							let library = path(scope, span, &module)?;
							let function = HLoadFunction {
								library,
								reference,
								annotations,
								name,
								signature,
							};

							let entry = Arc::get_mut(entry).unwrap();
							entry.push(Arc::new(PFunction::Load(function)));
						}
						_ => symbol.invalid(scope)?,
					}
				}
				_ => module.invalid(scope)?,
			}
		}
		"function" => {
			let identifier = node.identifier(scope)?;
			let entry = table.functions.entry(identifier.clone()).or_default();
			let span = &symbols.functions[&identifier][entry.len()];
			if node.attribute("root").is_some() {
				table.roots.push((identifier.clone(), entry.len()));
			}

			let annotations = annotations(scope, inclusions, span, &node)?;
			let signature = signature(scope, inclusions, span, &node)?;
			let name = node.identifier_span(scope, span)?;
			let value = node.field(scope, "value")?;
			let value = super::value_frame(scope, inclusions,
				span, &signature.parameters, value);

			let entry = Arc::get_mut(entry).unwrap();
			let function = HFunction { annotations, name, signature, value };
			entry.push(Arc::new(PFunction::Local(function)));
		}
		"data" => {
			let identifier = node.identifier(scope)?;
			let span = &symbols.structures[&identifier];
			let fields = super::variables(scope, inclusions, span, &node)?;
			let name = node.identifier_span(scope, span)?;
			let data = Arc::new(HData { name, fields });
			table.structures.insert(identifier, data);
		}
		"static" => {
			let identifier = node.identifier(scope)?;
			let span = &symbols.statics[&identifier];
			let kind = node.attribute("type").map(|kind|
				super::kind(scope, inclusions, span, kind)).transpose()?;
			let value = node.attribute("value").map(|value|
				super::value(scope, inclusions, span, value));

			if !kind.is_some() && !value.is_some() {
				return E::error().message("static variable has no type")
					.note("add a type annotation or assign it a value")
					.label(node.span().label()).result(scope);
			}

			let name = node.identifier_span(scope, span)?;
			let annotations = annotations(scope, inclusions, span, &node)?;
			let statics = PStatic::Local(HStatic { annotations, name, kind, value });
			table.statics.insert(identifier, Arc::new(statics));
		}
		"annotation" => {
			let (span, module) = module.as_mut().ok_or_else(|| E::error()
				.message("annotation has no associated item").label(node.span().label())
				.note("global annotations must use: global_annotation").to(scope))?;
			annotation(scope, inclusions, &mut module.annotations, span, node)?;
		}
		"global_annotation" => {
			let identifier = node.field(scope, "name")?;
			let name = Identifier(node.text().into());
			let span = identifier.span();

			let value = node.field(scope, "value")?;
			let value = TSpan::scope(span, |span|
				super::value(scope, inclusions, span, value));
			table.global_annotations.push((name, span, value));
		}
		_ => (),
	})).last();
	Ok(Arc::new(table))
}

pub fn path<'a>(scope: MScope, span: &TSpan, node: &impl Node<'a>) -> crate::Result<S<Path>> {
	let path = node.children().fold(Path::Root, |path, identifier|
		Path::Node(Arc::new(path), Identifier(identifier.text().into())));
	match path {
		Path::Node(_, _) => Ok(S::new(path, TSpan::offset(span, node.span()))),
		Path::Root => E::error().message("path cannot be empty")
			.label(node.span().label()).result(scope),
	}
}

pub fn signature<'a>(scope: MScope, inclusions: &Inclusions, span: &TSpan,
					 node: &impl Node<'a>) -> crate::Result<HSignature> {
	Ok(HSignature {
		convention: node.attribute("convention").map(|node|
			node.identifier_span(scope, span)).transpose()?,
		parameters: super::variables(scope, inclusions, span, node)?,
		return_type: node.attribute("return").map(|node|
			super::kind(scope, inclusions, span, node)).transpose()?
			.unwrap_or_else(|| S::new(HType::Void, ISpan::internal())),
	})
}

fn annotations<'a>(scope: MScope, inclusions: &Inclusions, span: &TSpan,
				   node: &impl Node<'a>) -> crate::Result<HAnnotations> {
	let mut annotations = HAnnotations::new();
	node.children().filter(|node| node.kind() == "annotation").try_for_each(|node|
		annotation(scope, inclusions, &mut annotations, span, node))?;
	Ok(annotations)
}

fn annotation<'a>(scope: MScope, inclusions: &Inclusions, annotations: &mut HAnnotations,
				  span: &TSpan, node: impl Node<'a>) -> crate::Result<()> {
	let value = node.field(scope, "value")?;
	let value = super::value(scope, inclusions, span, value);
	let name = node.identifier_span(scope, span)?;
	let (name, offset) = (name.node, name.span);

	Ok(match annotations.get(&name) {
		None => annotations.insert(name, (offset, value)).unwrap_none(),
		Some((other, _)) => E::error().message("duplicate annotation")
			.label(TSpan::lift(span, *other).label())
			.label(TSpan::lift(span, offset).other())
			.emit(scope),
	})
}
