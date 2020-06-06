use std::collections::HashMap;

use tree_sitter::Node;

use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::*;
use crate::span::{S, Span};

use super::{Source, SymbolKind, Symbols};

pub struct Scene<'a, 'b> {
	pub context: &'a Context,
	pub symbols: &'a Symbols<'b>,
	pub generations: HashMap<Identifier, u16>,
	pub stack: Vec<HashMap<Identifier, u16>>,
	pub source: &'a Source,
	pub value: Value,
}

impl<'a, 'b> Scene<'a, 'b> {
	pub fn new(context: &'a Context, symbols: &'a Symbols<'b>, source: &'a Source) -> Self {
		let (generations, stack) = (HashMap::new(), vec![HashMap::new()]);
		Scene { context, symbols, generations, stack, source, value: Value::default() }
	}

	pub fn binding(&mut self, identifier: S<Identifier>) -> S<Variable> {
		let generation = self.generations.entry(identifier.node.clone()).or_default();
		self.stack.last_mut().unwrap().insert(identifier.node.clone(), *generation);
		let variable = S::new(Variable(identifier.node, *generation), identifier.span);
		*generation += 1;
		variable
	}

	fn value(&mut self, value: ValueNode, node: Node) -> ValueIndex {
		self.value.insert(S::create(value, node.byte_range(), self.source.file))
	}

	fn generation(&self, identifier: &Identifier) -> Option<u16> {
		self.stack.iter().rev().find_map(|frame| frame.get(&identifier)).cloned()
	}

	fn variable(&mut self, identifier: S<Identifier>) -> crate::Result<ValueIndex> {
		match self.generation(&identifier.node) {
			Some(generation) => {
				let variable = ValueNode::Variable(Variable(identifier.node, generation));
				Ok(self.value.insert(S::new(variable, identifier.span)))
			}
			None => self.context.pass(Diagnostic::error()
				.label(identifier.span.label()).message("undefined variable"))
		}
	}
}

pub fn value(context: &Context, symbols: &Symbols,
			 source: &Source, node: Node) -> crate::Result<Value> {
	let mut scene = Scene::new(context, symbols, source);
	scene.value.root = unit(&mut scene, node)?;
	Ok(scene.value)
}

pub fn unit(scene: &mut Scene, node: Node) -> crate::Result<ValueIndex> {
	let value = match node.kind() {
		"break" => ValueNode::Break,
		"path" => return path(scene, node),
		"group" => return unit(scene, node.named_child(0).unwrap()),
		"string" => ValueNode::String(super::string(scene.source, node).node),
		"register" => ValueNode::Register(super::identifier(scene.source, node).node),
		"truth" => ValueNode::Truth(&scene.source.text[node.byte_range()] == "true"),
		"rune" => super::string(scene.source, node).node
			.chars().next().map(ValueNode::Rune).unwrap(),
		"block" => {
			let cursor = &mut node.walk();
			scene.stack.push(HashMap::new());
			let block = node.named_children(cursor).map(|node|
				unit(scene, node)).collect::<Result<_, _>>()?;

			scene.stack.pop();
			ValueNode::Block(block)
		}
		"integral" => {
			let string = &scene.source.text[node.byte_range()];
			let string = &string.replace('\'', "");
			ValueNode::Integral(match string.get(..2) {
				Some("0x") => i128::from_str_radix(&string[2..], 16),
				Some("0o") => i128::from_str_radix(&string[2..], 8),
				Some("0b") => i128::from_str_radix(&string[2..], 2),
				_ => i128::from_str_radix(string, 10),
			}.unwrap())
		}
		"let" => {
			let identifier = super::field_identifier(scene.source, node);
			let node_type = node.child_by_field_name("type")
				.map(|node| super::node_type(scene.context,
					scene.symbols, scene.source, node)).transpose()?;
			let value = node.child_by_field_name("value")
				.map(|node| unit(scene, node)).transpose()?;
			ValueNode::Let(scene.binding(identifier), node_type, value)
		}
		"set" => {
			let target = unit(scene, node.child_by_field_name("target").unwrap())?;
			let value = unit(scene, node.child_by_field_name("value").unwrap())?;
			ValueNode::Set(target, value)
		}
		"compound" => {
			let target = unit(scene, node.child_by_field_name("target").unwrap())?;
			let value = unit(scene, node.child_by_field_name("value").unwrap())?;
			let operator = node.child_by_field_name("operator").unwrap();
			let operator = &scene.source.text[operator.byte_range()];
			let operator = Dual::parse(operator).unwrap_or_else(||
				panic!("invalid compound operator: {}", operator));
			ValueNode::Compound(operator, target, value)
		}
		"return" => {
			let value = node.child_by_field_name("value").map(|node|
				unit(scene, node)).transpose()?;
			ValueNode::Return(value)
		}
		"when" => {
			let cursor = &mut node.walk();
			ValueNode::When(node.named_children(cursor).map(|node|
				Ok((unit(scene, node.child_by_field_name("condition").unwrap())?,
					unit(scene, node.child_by_field_name("branch").unwrap())?)))
				.collect::<Result<_, _>>()?)
		}
		"while" => {
			let condition = unit(scene, node.child_by_field_name("condition").unwrap())?;
			let value = unit(scene, node.child_by_field_name("block").unwrap())?;
			ValueNode::While(condition, value)
		}
		"access" => {
			let value = unit(scene, node.child_by_field_name("value").unwrap())?;
			let field = node.child_by_field_name("field").unwrap();
			let field = super::identifier(scene.source, field);
			ValueNode::Field(value, field)
		}
		"create" => {
			let cursor = &mut node.walk();
			let fields = node.children_by_field_name("field", cursor);
			let path = super::path(scene.source, node.child_by_field_name("path").unwrap());
			ValueNode::Create(path, fields.map(|field| Ok(match field.kind() {
				"identifier" => {
					let identifier = super::identifier(scene.source, field);
					let variable = scene.variable(identifier.clone())?;
					(identifier.node, (variable, identifier.span))
				}
				"field" => {
					let name = field.child_by_field_name("name").unwrap();
					let identifier = super::identifier(scene.source, name);
					let value = field.child_by_field_name("value").unwrap();
					(identifier.node, (unit(scene, value)?, identifier.span))
				}
				other => panic!("invalid field kind: {}", other)
			})).collect::<Result<_, _>>()?)
		}
		"slice" => {
			let value = node.child_by_field_name("value");
			let value = unit(scene, value.unwrap())?;
			let left = node.child_by_field_name("left")
				.map(|node| unit(scene, node)).transpose()?;
			let right = node.child_by_field_name("right")
				.map(|node| unit(scene, node)).transpose()?;
			ValueNode::Slice(value, left, right)
		}
		"index" => {
			let value = unit(scene, node.child_by_field_name("value").unwrap())?;
			let index = unit(scene, node.child_by_field_name("index").unwrap())?;
			ValueNode::Index(value, index)
		}
		"array" => {
			let cursor = &mut node.walk();
			let values = node.named_children(cursor).map(|node|
				unit(scene, node)).collect::<Result<_, _>>()?;
			ValueNode::Array(values)
		}
		"call" => {
			let path = node.child_by_field_name("function");
			let path = super::path(scene.source, path.unwrap());
			let span = path.span;

			match scene.symbols.resolve(scene.context, &path.node, &span) {
				Some((path, SymbolKind::Function)) |
				Some((path, SymbolKind::Intrinsic)) => {
					let cursor = &mut node.walk();
					let arguments = node.children_by_field_name("argument", cursor)
						.map(|node| unit(scene, node)).collect::<Result<_, _>>()?;
					ValueNode::Call(S::new(path, span), arguments)
				}
				Some(_) => return scene.context.pass(Diagnostic::error().label(span.label())
					.message(format!("symbol for path: {}, is not a function", path.node))),
				None => return scene.context.pass(Diagnostic::error().label(span.label())
					.message(format!("no function for path: {}", path.node)))
			}
		}
		"cast" => {
			let value = unit(scene, node.child_by_field_name("value").unwrap())?;
			let node_type = node.child_by_field_name("type").unwrap();
			let node_type = super::node_type(scene.context,
				scene.symbols, scene.source, node_type)?;
			ValueNode::Cast(value, node_type)
		}
		"unary" => {
			let value = unit(scene, node.child_by_field_name("value").unwrap())?;
			let operator = node.child_by_field_name("operator").unwrap();
			match &scene.source.text[operator.byte_range()] {
				"#" => ValueNode::Compile(value),
				"inline" => ValueNode::Inline(value),
				"!" => ValueNode::Unary(Unary::Not, value),
				"-" => ValueNode::Unary(Unary::Negate, value),
				"&" => ValueNode::Unary(Unary::Reference, value),
				"*" => ValueNode::Unary(Unary::Dereference, value),
				other => panic!("invalid unary operator: {}", other),
			}
		}
		"binary" => {
			let left = unit(scene, node.child_by_field_name("left").unwrap())?;
			let right = unit(scene, node.child_by_field_name("right").unwrap())?;
			let operator = node.child_by_field_name("operator").unwrap();
			let operator = &scene.source.text[operator.byte_range()];
			let operator = Binary::parse(operator).unwrap_or_else(||
				panic!("invalid binary operator: {}", operator));
			ValueNode::Binary(operator, left, right)
		}
		other => panic!("invalid value kind: {}", other),
	};

	Ok(scene.value(value, node))
}

fn path(scene: &mut Scene, node: Node) -> crate::Result<ValueIndex> {
	let cursor = &mut node.walk();
	let mut elements: Vec<_> = node.named_children(cursor).map(|node: Node|
		(node, super::identifier(scene.source, node))).collect();
	let (first, first_element) = elements.first().unwrap();
	if let Some(generation) = scene.generation(&first_element.node) {
		let (first, first_element) = elements.remove(0);
		let variable = Variable(first_element.node, generation);
		let variable = scene.value(ValueNode::Variable(variable), first);
		return Ok(path_fields(scene, variable, elements));
	}

	let first = *first;
	let mut fields = Vec::new();
	while !elements.is_empty() {
		let path = Path(elements.iter().map(|(_, element)|
			element.node.clone()).collect());
		let (last, _) = elements.last().unwrap();
		let range = first.start_byte()..last.end_byte();
		let span = Span::new(range, scene.source.file);

		match scene.symbols.resolve(scene.context, &path, &span) {
			Some((path, SymbolKind::Variable)) |
			Some((path, SymbolKind::Module)) => {
				fields.reverse();
				let path = S::new(ValueNode::Path(path), span);
				let path = scene.value.insert(path);
				return Ok(path_fields(scene, path, fields));
			}
			Some(_) => return scene.context.pass(Diagnostic::error().label(span.label())
				.message(format!("symbol for path: {}, is not a variable", path))),
			None => fields.push(elements.pop().unwrap()),
		};
	}

	let path = super::path(scene.source, node);
	scene.context.pass(Diagnostic::error().label(path.span.label())
		.message(format!("no variable for path: {}", path)))
}

fn path_fields(scene: &mut Scene, node: ValueIndex,
			   fields: Vec<(Node, S<Identifier>)>) -> ValueIndex {
	fields.into_iter().rev().fold(node, |field, (node, identifier)|
		scene.value(ValueNode::Field(field, identifier), node))
}
