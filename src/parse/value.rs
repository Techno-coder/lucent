use std::collections::HashMap;
use std::iter::FromIterator;

use crate::node::*;
use crate::query::{E, MScope, QueryError, S};

use super::{Inclusions, Node, TSpan};

struct Scene<'a> {
	frames: Vec<HashMap<Identifier, usize>>,
	value: &'a mut HValue,
}

impl<'a> Scene<'a> {
	pub fn scope<F, R>(&mut self, function: F) -> R
		where F: FnOnce(&mut Self) -> R {
		self.frames.push(HashMap::new());
		let value = function(self);
		self.frames.pop();
		value
	}

	pub fn generation(&self, name: &Identifier) -> Option<usize> {
		self.frames.iter().rev()
			.find_map(|frame| frame.get(name))
			.map(|index| *index)
	}
}

/// Parses an `HValue`. This function does not return
/// `Result` as any errors are replaced with `HNode::Error`.
pub fn value<'a>(scope: MScope, inclusions: &Inclusions,
				 span: &TSpan, node: impl Node<'a>) -> HValue {
	value_frame(scope, inclusions, span, &HVariables::new(), node)
}

pub fn value_frame<'a>(scope: MScope, inclusions: &Inclusions, span: &TSpan,
					   parameters: &HVariables, node: impl Node<'a>) -> HValue {
	HValue::new(|value| {
		let frames = vec![HashMap::from_iter(parameters
			.keys().map(|key| (key.clone(), 0)))];
		let scene = &mut Scene { frames, value };
		valued(scene, scope, inclusions, span, &node)
	})
}

fn valued<'a>(scene: &mut Scene, scope: MScope, inclusions: &Inclusions,
			  base: &TSpan, node: &impl Node<'a>) -> HIndex {
	let valued_node = |scene: &mut Scene, scope: MScope, node|
		valued(scene, scope, inclusions, base, &node);
	let valued = |scene: &mut Scene, scope: MScope, field| {
		let field = node.field(scope, field)?;
		Ok(valued_node(scene, scope, field))
	};

	let label = node.span().label();
	let span = TSpan::offset(base, node.span());
	let node = (|| Ok(match node.kind() {
		"break" => HNode::Break,
		"continue" => HNode::Continue,
		"string" => HNode::String(node.text().into()),
		"register" => HNode::Register(Identifier(node.text().into())),
		"truth" => HNode::Truth(node.text() == "true"),
		"rune" => match node.text().chars().next() {
			None => E::error().message("empty rune")
				.label(label.clone()).result(scope)?,
			Some(rune) => HNode::Rune(rune),
		}
		"integral" => HNode::Integral(integral(scope, node)?),
		"path" => paths(scene, scope, inclusions, base, node)?.node,
		"block" => scene.scope(|scene| HNode::Block(node.children()
			.map(|node| valued_node(scene, scope, node)).collect())),
		"group" => HNode::Block(vec![valued(scene, scope, "value")?]),
		// let variable: type = value
		"let" => {
			let next = |index| index + 1;
			let name = node.identifier_span(scope, base)?;
			let generation = scene.generation(&name.node).map(next).unwrap_or(0);
			let variable = S::new(Variable(name.node.clone(), generation), name.span);
			scene.frames.last_mut().unwrap().insert(name.node, generation);

			let kind = node.attribute("type").map(|node|
				super::kind(scope, inclusions, base, node)).transpose()?;
			let value = node.attribute("value").map(|node|
				valued_node(scene, scope, node));
			HNode::Let(variable, kind, value)
		}
		// variable = value
		"set" => {
			let target = valued(scene, scope, "target")?;
			let value = valued(scene, scope, "value")?;
			HNode::Set(target, value)
		}
		// variable += value
		"compound" => {
			let target = valued(scene, scope, "target")?;
			let value = valued(scene, scope, "value")?;
			let operator = node.field(scope, "operator")?;
			let dual = HDual::parse(operator.text())
				.ok_or_else(|| E::error().message("invalid compound operator")
					.label(operator.span().label()).to(scope))?;
			HNode::Compound(dual, target, value)
		}
		// return value
		"return" => HNode::Return(node.attribute("value")
			.map(|node| valued_node(scene, scope, node))),
		// if condition: statement
		"when" => HNode::When(node.children().map(|node| {
			let branch = node.field(scope, "branch")?;
			let branch = valued_node(scene, scope, branch);
			let condition = node.field(scope, "condition")?;
			let condition = valued_node(scene, scope, condition);
			Ok((condition, branch))
		}).collect::<Result<_, _>>()?),
		// while condition: statement
		"while" => {
			let condition = valued(scene, scope, "condition")?;
			let value = valued(scene, scope, "value")?;
			HNode::While(condition, value)
		}
		// value.field
		"access" => {
			let value = valued(scene, scope, "value")?;
			let field = node.identifier_span(scope, base)?;
			HNode::Field(value, field)
		}
		// new Structure field: value
		"new" => {
			let mut fields = HashMap::new();
			for field in node.children().filter(|node| node.kind() == "field") {
				let name = field.identifier_span(scope, base)?;
				let (name, span) = (name.node, name.span);
				let value = field.attribute("value")
					.map(|node| Ok(valued_node(scene, scope, node)));
				let value = value.unwrap_or_else(|| scene.generation(&name)
					.map(|index| HNode::Variable(Variable(name.clone(), index)))
					.map(|node| scene.value.insert(S::new(node, span)))
					.ok_or_else(|| E::error()
						.message(format!("undefined variable: {}", name))
						.label(field.span().label()).to(scope)))?;

				match fields.get(&name) {
					None => fields.insert(name, (span, value)).unwrap_none(),
					Some((other, _)) => E::error().message("duplicate field")
						.label(TSpan::lift(base, *other).label())
						.label(TSpan::lift(base, span).other())
						.emit(scope),
				}
			}

			let kind = node.field(scope, "type")?;
			match kind.kind() {
				"path" => {
					let scope = &mut scope.span(kind.span());
					let node = super::path(scope, base, &kind)?;
					let path = inclusions.structure(scope, base, &node)?
						.ok_or_else(|| E::error().message("unresolved structure")
							.label(kind.span().label()).to(scope))?;
					HNode::New(path, fields)
				}
				"slice_type" => {
					let kind = kind.field(scope, "type")?;
					let kind = super::kind(scope, inclusions, base, kind)?;
					HNode::SliceNew(kind, fields)
				}
				_ => kind.invalid(scope)?,
			}
		}
		// variable[left:right]
		"slice" => {
			let value = valued(scene, scope, "value")?;
			let left = node.attribute("left")
				.map(|node| valued_node(scene, scope, node));
			let right = node.attribute("right")
				.map(|node| valued_node(scene, scope, node));
			HNode::Slice(value, left, right)
		}
		// variable[index]
		"index" => {
			let value = valued(scene, scope, "value")?;
			let index = valued(scene, scope, "index")?;
			HNode::Index(value, index)
		}
		// [value]
		"array" => scene.scope(|scene| HNode::Array(node.children()
			.map(|node| valued_node(scene, scope, node)).collect())),
		// path(value)
		"call" => {
			let arguments = node.field(scope, "arguments")?.children()
				.map(|node| valued_node(scene, scope, node)).collect();
			let function = node.field(scope, "function")?;
			if function.kind() != "path" {
				let value = valued_node(scene, scope, function);
				return Ok(HNode::Method(value, arguments));
			}

			let node = paths(scene, scope, inclusions, base, &function)?;
			match node.node {
				HNode::Unresolved(_) | HNode::Error(_) => node.node,
				HNode::Function(path) => HNode::Call(path, arguments),
				_ => HNode::Method(scene.value.insert(node), arguments),
			}
		}
		// value as type
		"cast" => {
			let value = valued(scene, scope, "value")?;
			let kind = node.field(scope, "type")?;
			HNode::Cast(value, (kind.text() != "_").then(||
				super::kind(scope, inclusions, base, kind)).transpose()?)
		}
		// -value
		"unary" => {
			let values = node.field(scope, "value")?;
			match node.field(scope, "operator")?.text() {
				"#" => HNode::Compile(value(scope, inclusions, base, values)),
				"inline" => HNode::Inline(value(scope, inclusions, base, values)),
				"!" => HNode::Unary(Unary::Not, valued_node(scene, scope, values)),
				"-" => HNode::Unary(Unary::Negate, valued_node(scene, scope, values)),
				"&" => HNode::Unary(Unary::Reference, valued_node(scene, scope, values)),
				_ => E::error().message("invalid unary operator")
					.label(label.clone()).result(scope)?,
			}
		}
		// value!
		"dereference" => {
			let value = valued(scene, scope, "value")?;
			HNode::Unary(Unary::Dereference, value)
		}
		// left + right
		"binary" => {
			let left = valued(scene, scope, "left")?;
			let right = valued(scene, scope, "right")?;
			let operator = node.field(scope, "operator")?;
			let operator = HBinary::parse(operator.text())
				.ok_or_else(|| E::error().message("invalid binary operator")
					.label(operator.span().label()).to(scope))?;
			HNode::Binary(operator, left, right)
		}
		_ => {
			let _: crate::Result<()> = node.invalid(scope);
			let value = |node| valued_node(scene, scope, node);
			HNode::Error(node.children().map(value).collect())
		}
	}))().unwrap_or_else(|_: QueryError| HNode::Error(vec![]));
	scene.value.insert(S::new(node, span))
}

fn paths<'a>(scene: &mut Scene, scope: MScope, inclusions: &Inclusions,
			 base: &TSpan, node: &impl Node<'a>) -> crate::Result<S<HNode>> {
	let map_names = || node.children().map(|node|
		(Identifier(node.text().into()), node.span()));
	let field = |scene: &mut Scene, node, (name, span)| {
		let field = S::new(name, TSpan::offset(base, span));
		let field = HNode::Field(scene.value.insert(node), field);
		S::new(field, TSpan::offset(base, span))
	};

	// Find variable in scope with name.
	let names = &mut map_names();
	let (name, span) = names.next().unwrap();
	if let Some(generation) = scene.generation(&name) {
		let variable = HNode::Variable(Variable(name, generation));
		let node = S::new(variable, TSpan::offset(base, span));
		let fields = |node, name| field(scene, node, name);
		return Ok(names.fold(node, fields));
	}

	// Find symbol at possible paths.
	let mut names = map_names();
	let mut path = HPath::root();
	for (name, span) in &mut names {
		let scope = &mut scope.span(span);
		let name = S::new(name, TSpan::offset(base, span));
		path = HPath::Node(Box::new(path), name);

		let node = inclusions.function(scope, base, &path)
			.map(|path| path.map(|path| HNode::Function(path))).transpose();
		let node = node.or_else(|| inclusions.statics(scope, base, &path)
			.map(|path| path.map(|path| HNode::Static(path))).transpose());
		if let Some(node) = node.transpose()? {
			let node = S::new(node, TSpan::offset(base, span));
			let fields = |node, name| field(scene, node, name);
			return Ok(names.fold(node, fields));
		}
	}

	let error = E::error().message("unresolved path");
	error.label(node.span().label()).emit(scope);
	let span = TSpan::offset(base, node.span());
	Ok(S::new(HNode::Unresolved(path), span))
}

pub fn integral<'a>(scope: MScope, node: &impl Node<'a>) -> crate::Result<i128> {
	let string = node.text().replace('\'', "");
	match string.get(..2) {
		Some("0x") => i128::from_str_radix(&string[2..], 16),
		Some("0o") => i128::from_str_radix(&string[2..], 8),
		Some("0b") => i128::from_str_radix(&string[2..], 2),
		_ => i128::from_str_radix(&string, 10),
	}.map_err(|_| E::error().message("invalid integer")
		.label(node.span().label()).to(scope))
}
