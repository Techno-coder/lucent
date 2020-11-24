use std::convert::TryFrom;
use std::sync::Arc;

use tree_sitter::{Language, Parser, Query};

use crate::FilePath;
use crate::node::*;
use crate::query::{E, ISpan, MScope, QScope, S};
use crate::source::{File, Source};

use super::*;

pub fn language() -> Language {
	extern { fn tree_sitter_lucent() -> Language; }
	unsafe { tree_sitter_lucent() }
}

pub fn parser() -> Parser {
	let mut parser = Parser::new();
	parser.set_language(language()).unwrap();
	parser
}

pub fn errors() -> Query {
	let query = "(ERROR) @error";
	Query::new(language(), query).unwrap()
}

/// Contains references to `Source` instances.
/// Designed to be copied without overhead during parsing.
#[derive(Debug, Copy, Clone)]
pub struct PSource<'a> {
	pub text: &'a str,
	pub file: File,
}

impl<'a> PSource<'a> {
	pub fn new(source: &'a Source) -> Self {
		Self {
			text: &source.text,
			file: source.file,
		}
	}
}

pub fn file_table(scope: QScope, symbols: &SymbolTable, inclusions: Inclusions,
				  path: &FilePath) -> crate::Result<Arc<ItemTable>> {
	let source = crate::source::source(scope, path)?;
	let tree = parser().parse(source.text.as_bytes(), None).unwrap();
	let root = TreeNode::new(tree.root_node(), PSource::new(&source));
	Ok(parse_table(scope, symbols, inclusions, root))
}

pub fn parse_table<'a>(scope: MScope, symbols: &SymbolTable, inclusions: Inclusions,
					   node: impl Node<'a>) -> Arc<ItemTable> {
	let span = TSpan::offset(&symbols.span, node.span());
	let module = HModule { span, annotations: HAnnotations::new() };
	let mut table = ItemTable::new(module, inclusions);
	let item = |node| item(scope, symbols, &mut table, node);
	node.children().map(item).last();
	Arc::new(table)
}

fn item<'a>(scope: MScope, symbols: &SymbolTable, table: &mut ItemTable,
			node: impl Node<'a>) -> crate::Result<()> {
	let base = &symbols.span;
	let inclusions = &mut table.inclusions;
	Ok(match node.kind() {
		"module" => {
			let name = node.identifier(scope)?;
			let (_, location) = &symbols.modules[&name];
			if let ModuleLocation::Inline(symbols) = location {
				let inclusions = inclusions.scope(name.clone());
				let parsed = parse_table(scope, symbols, inclusions, node);
				table.modules.insert(name, parsed);
			}
		}
		"use" => if node.attribute("path").is_none() {
			let scope = &mut scope.span(node.span());
			let mut children = node.children();
			let mut target = HPath::root();
			for child in &mut children {
				match child.kind() {
					"identifier" => {
						let name = Identifier(child.text().into());
						let name = S::new(name, TSpan::offset(base, child.span()));
						target = HPath::Node(Box::new(target), name);
					}
					"wildcard" => return match children.next().is_some() {
						true => E::error().message("wildcard must appear last")
							.label(node.span().label()).result(scope),
						false => Ok(inclusions.wildcard(scope, target)?),
					},
					_ => child.invalid(scope)?,
				}
			}
			inclusions.specific(scope, base, target)?;
		},
		"load" => {
			let node = node.field(scope, "load")?;
			let module = node.field(scope, "module")?;
			match module.kind() {
				"string" => {
					// TODO: implement C header loading
					let identifier = node.identifier(scope)?;
					let span = &symbols.libraries[&identifier];
					let name = node.identifier_span(scope, span)?;
					let annotations = annotations(scope, inclusions, span, &node)?;
					let library = HLibrary { annotations, name, path: module.text().into() };
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
					let identifier = symbol.identifier(scope)?;
					let library = &mut |scope: MScope, base| {
						let path = path(scope, base, &module)?;
						let scope = &mut scope.span(module.span());
						inclusions.library(scope, base, &path)?
							.ok_or_else(|| E::error().message("undefined library")
								.label(scope.span.label()).to(scope))
					};

					match symbol.kind() {
						"static" => {
							let span = &symbols.statics[&identifier];
							let library = library(scope, base)?;
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
							let library = library(scope, span)?;
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
			let name = node.identifier_span(scope, span)?;
			let fields = super::variables(scope, inclusions, span, &node)?;
			let annotations = annotations(scope, inclusions, span, &node)?;
			let data = Arc::new(HData { annotations, name, fields });
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
			let module = Arc::get_mut(&mut table.module).unwrap();
			annotation(scope, inclusions, &mut module.annotations, base, node)?;
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
	})
}

pub fn path<'a>(scope: MScope, span: &TSpan,
				node: &impl Node<'a>) -> crate::Result<HPath> {
	let path = node.children().fold(HPath::root(), |path, name| {
		let span = TSpan::offset(span, name.span());
		let name = Identifier(name.text().into());
		HPath::Node(Box::new(path), S::new(name, span))
	});

	match path {
		HPath::Node(_, _) => Ok(path),
		HPath::Root(_) => E::error()
			.message("path cannot be empty")
			.label(node.span().label())
			.result(scope),
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
