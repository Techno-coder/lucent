use tree_sitter::Node;

use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::{Function, Identifier, IntegralSize, Parameter, ReturnType, Type};
use crate::parse::Scene;
use crate::span::S;

use super::{Source, SymbolKind, Symbols};

pub fn function(context: &Context, symbols: &mut Symbols,
				source: &Source, node: Node) -> crate::Result<Function> {
	let identifier = super::field_identifier(source, node);
	let is_root = node.child_by_field_name("root").is_some();
	let mut scene = Scene::new(context, symbols, source);

	let parameters = parameters(&mut scene, node)?;
	let return_type = return_type(&mut scene, node, &identifier)?;
	let convention = node.child_by_field_name("convention")
		.map(|node| super::identifier(source, node));

	let value = node.child_by_field_name("block").unwrap();
	scene.value.root = super::unit(&mut scene, value)?;
	let value = scene.value;

	let annotations = super::annotations(context, symbols, source, node)?;
	Ok(Function { is_root, convention, annotations, parameters, return_type, value })
}

fn parameters(scene: &mut Scene, node: Node) -> crate::Result<Vec<S<Parameter>>> {
	let cursor = &mut node.walk();
	node.children_by_field_name("parameter", cursor)
		.map(|node| Ok(S::create(match node.kind() {
			"register" => Parameter::Register(super::identifier(scene.source, node)),
			"parameter" => {
				let identifier = super::field_identifier(scene.source, node);
				match scene.generations.contains_key(&identifier.node) {
					true => scene.context.pass(Diagnostic::error()
						.message("duplicate parameter").label(identifier.span.label()))?,
					false => {
						let node_type = node_type(scene.context, scene.symbols,
							scene.source, node.child_by_field_name("type").unwrap())?;
						Parameter::Variable(scene.binding(identifier), node_type)
					}
				}
			}
			other => panic!("invalid parameter type: {}", other),
		}, node.byte_range(), scene.source.file))).collect()
}

fn return_type(scene: &mut Scene, node: Node, identifier: &S<Identifier>)
			   -> crate::Result<S<ReturnType>> {
	Ok(node.child_by_field_name("return")
		.map(|node| Ok(S::create(match node.kind() {
			"register" => ReturnType::Register(super::identifier(scene.source, node)),
			_ => ReturnType::Type(node_type(scene.context,
				scene.symbols, scene.source, node)?),
		}, node.byte_range(), scene.source.file))).transpose()?
		.unwrap_or_else(|| {
			let void = S::new(Type::Void, identifier.span.clone());
			S::new(ReturnType::Type(void), identifier.span.clone())
		}))
}

pub fn node_type(context: &Context, symbols: &Symbols,
				 source: &Source, node: Node) -> crate::Result<S<Type>> {
	Ok(S::create(match node.kind() {
		"pointer" => {
			let node = node_type(context, symbols,
				source, node.named_child(0).unwrap())?;
			Type::Pointer(Box::new(node))
		}
		"array_type" => {
			let element = node.child_by_field_name("type").unwrap();
			let size = node.child_by_field_name("size").unwrap();
			let size = super::value(context, symbols, source, size)?;
			let element = node_type(context, symbols, source, element)?;
			Type::Array(Box::new(element), size)
		}
		"slice_type" => {
			let element = node.child_by_field_name("type").unwrap();
			let element = node_type(context, symbols, source, element)?;
			Type::Slice(Box::new(element))
		}
		"path" => return path_type(context, source, symbols, node),
		other => panic!("invalid type kind: {}", other),
	}, node.byte_range(), source.file))
}

fn path_type(context: &Context, source: &Source, symbols: &Symbols,
			 node: Node) -> crate::Result<S<Type>> {
	let path = super::path(source, node);
	Ok(S::create(match path.to_string().as_str() {
		"rune" => Type::Rune,
		"truth" => Type::Truth,
		"never" => Type::Never,
		"i8" => Type::Signed(IntegralSize::Byte),
		"i16" => Type::Signed(IntegralSize::Word),
		"i32" => Type::Signed(IntegralSize::Double),
		"i64" => Type::Signed(IntegralSize::Quad),
		"u8" => Type::Unsigned(IntegralSize::Byte),
		"u16" => Type::Unsigned(IntegralSize::Word),
		"u32" => Type::Unsigned(IntegralSize::Double),
		"u64" => Type::Unsigned(IntegralSize::Quad),
		_ => match symbols.resolve(context, &path.node, &path.span) {
			Some((path, SymbolKind::Structure)) => Type::Structure(path),
			Some(_) => return context.pass(Diagnostic::error().label(path.span.label())
				.message(format!("symbol at path: {}, is not a data structure", path))),
			None => return context.pass(Diagnostic::error().label(path.span.label())
				.message(format!("no data structure at path: {}", path))),
		},
	}, node.byte_range(), source.file))
}
