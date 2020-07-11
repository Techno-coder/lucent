use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::*;
use crate::span::{S, Span};

use super::{Scene, Terminal, TypeVariable};

pub fn value(context: &Context, scene: &mut Scene, place: Option<&S<TypeVariable>>,
			 value: &Value, index: &ValueIndex) -> crate::Result<TypeVariable> {
	let span = &value[*index].span;
	Ok(match &value[*index].node {
		ValueNode::Block(values) => {
			let last = values.iter().map(|index| self::value(context,
				scene, place, value, index)).last().unwrap()?;
			*scene.values.entry(*index).insert(last).get()
		}
		ValueNode::Let(variable, ascription, other) => {
			let (node, type_variable) = (variable.clone(), scene.next());
			scene.variables.insert(node.node, (type_variable, node.span));
			ascription.iter().cloned().for_each(|ascription| scene.terminals
				.insert(type_variable, Terminal::Type(ascription)).unwrap_none());
			other.as_ref().map(|other| self::value(context, scene, place, value, other))
				.transpose()?.into_iter().for_each(|other_node| scene.unify(context,
				type_variable, other_node, &variable.span, &value[other.unwrap()].span));
			scene.ascribe(index, S::new(Type::Void, span.clone()))
		}
		ValueNode::Set(target, node) => {
			let node = self::value(context, scene, place, value, node)?;
			let target = self::value(context, scene, place, value, target)?;
			scene.unify(context, target, node, &value[target].span, &value[node].span);
			scene.ascribe(index, S::new(Type::Void, span.clone()))
		}
		ValueNode::While(condition, node) => {
			let _ = self::value(context, scene, place, value, node);
			let _ = truth_type(context, scene, place, value, condition);
			scene.ascribe(index, S::new(Type::Void, span.clone()))
		}
		ValueNode::When(branches) => {
			branches.iter().map(|(condition, _)| truth_type(context, scene,
				place, value, condition)).for_each(|variable| std::mem::drop(variable));
			let default = |condition| matches!(condition, &ValueNode::Truth(true));
			if !branches.iter().any(|(condition, _)| default(&value[*condition].node)) {
				branches.iter().map(|(_, index)| self::value(context, scene,
					place, value, index)).for_each(|variable| std::mem::drop(variable));
				return Ok(scene.ascribe(index, S::new(Type::Void, span.clone())));
			}

			let ((_, first), slice) = branches.split_first().unwrap();
			let variable = self::value(context, scene, place, value, first)?;
			let first_span = &value[*first].span;
			for (_, node) in slice {
				let other = self::value(context, scene, place, value, node)?;
				scene.unify(context, variable, other, first_span, &value[*node].span);
			}

			let entry = scene.values.entry(*index);
			*entry.insert(variable).get()
		}
		ValueNode::Cast(node, target) => {
			let _ = self::value(context, scene, place, value, node);
			scene.ascribe(index, target.clone())
		}
		ValueNode::Return(node) => {
			let place_node = place.ok_or_else(|| context.error(Diagnostic::error()
				.message("cannot return within enclosing item").label(span.label())))?;
			let node = node.as_ref().map(|index| self::value(context,
				scene, place, value, index)).transpose()?.unwrap_or_else(||
				scene.next_with(Terminal::Type(S::new(Type::Void, span.clone()))));
			scene.unify(context, node, place_node.node, span, &place_node.span);
			scene.ascribe(index, S::new(Type::Never, span.clone()))
		}
		ValueNode::Compile(node) => {
			let node = self::value(context, scene, place, value, node)?;
			*scene.values.entry(*index).insert(node).get()
		}
		ValueNode::Inline(node) => {
			let _ = self::value(context, scene, place, value, node);
			scene.ascribe(index, S::new(Type::Void, span.clone()))
		}
		ValueNode::Call(path, arguments) => super::function_call(context,
			scene, place, value, index, path, arguments)?,
		ValueNode::Field(node, field) => super::field(context,
			scene, place, value, index, node, field)?,
		ValueNode::Create(path, fields) => super::create(context,
			scene, place, value, index, fields, path)?,
		ValueNode::Slice(base, left, right) => {
			let element = scene.next();
			let base_span = &value[*base].span;
			let base = self::value(context, scene, place, value, base)?;
			let base_type = scene.next_with(Terminal::Sequence(element));
			scene.unify(context, base, base_type, base_span, base_span);
			right.as_ref().map(|right| integral_type(context,
				scene, place, value, right)).transpose()?;
			left.as_ref().map(|left| integral_type(context,
				scene, place, value, left)).transpose()?;
			scene.terminal(index, Terminal::Slice(element))
		}
		ValueNode::Index(base, node) => {
			let element = scene.next();
			let base_span = &value[*base].span;
			let base = self::value(context, scene, place, value, base)?;
			let base_type = scene.next_with(Terminal::Sequence(element));
			scene.unify(context, base, base_type, base_span, base_span);
			integral_type(context, scene, place, value, node)?;
			*scene.values.entry(*index).insert(element).get()
		}
		ValueNode::Compound(dual, target, other) => {
			let _ = self::dual(context, scene, place, value, dual, target, other);
			scene.ascribe(index, S::new(Type::Void, span.clone()))
		}
		ValueNode::Binary(binary, left, right) => match binary {
			Binary::Compare(Compare::Less) | Binary::Compare(Compare::LessEqual) |
			Binary::Compare(Compare::Greater) | Binary::Compare(Compare::GreaterEqual) => {
				integral_type(context, scene, place, value, left)?;
				let left_node = *scene.values.get(left).unwrap();
				let right_node = self::value(context, scene, place, value, right)?;
				let (left_span, right_span) = (&value[*left].span, &value[*right].span);
				scene.unify(context, left_node, right_node, left_span, right_span);
				scene.ascribe(index, S::new(Type::Truth, span.clone()))
			}
			Binary::Compare(Compare::Equal) | Binary::Compare(Compare::NotEqual) => {
				let left_node = self::value(context, scene, place, value, left)?;
				let right_node = self::value(context, scene, place, value, right)?;
				let (left_span, right_span) = (&value[*left].span, &value[*right].span);
				scene.unify(context, left_node, right_node, left_span, right_span);
				scene.ascribe(index, S::new(Type::Truth, span.clone()))
			}
			Binary::And | Binary::Or => {
				let _ = truth_type(context, scene, place, value, left);
				let _ = truth_type(context, scene, place, value, right);
				scene.ascribe(index, S::new(Type::Truth, span.clone()))
			}
			Binary::Dual(dual) => {
				self::dual(context, scene, place, value, dual, left, right)?;
				let variable = scene.values.get(left).unwrap().clone();
				*scene.values.entry(*index).insert(variable).get()
			}
		},
		ValueNode::Unary(unary, node) => match unary {
			Unary::Dereference => {
				let variable = scene.next();
				let target = scene.next_with(Terminal::Pointer(variable));
				let base = self::value(context, scene, place, value, node)?;
				scene.unify(context, base, target, &value[base].span, span);
				variable
			}
			Unary::Reference => {
				let node = self::value(context, scene, place, value, node)?;
				scene.terminal(index, Terminal::Pointer(node))
			}
			Unary::Negate => {
				integral_type(context, scene, place, value, node)?;
				let variable = scene.values.get(node).unwrap().clone();
				*scene.values.entry(*index).insert(variable).get()
			}
			Unary::Not => {
				let variable = self::value(context, scene, place, value, node)?;
				let terminal = &scene.find(variable);
				match scene.terminals.get(terminal) {
					Some(Terminal::Integral(_)) |
					Some(Terminal::Type(S { node: Type::Signed(_), .. })) |
					Some(Terminal::Type(S { node: Type::Unsigned(_), .. })) =>
						*scene.values.entry(*index).insert(variable).get(),
					Some(Terminal::Type(S { node: Type::Truth, .. })) =>
						scene.ascribe(index, S::new(Type::Truth, span.clone())),
					Some(node) => return context.pass(Diagnostic::error()
						.label(span.label().with_message(node.to_string()))
						.message("operation undefined for type")),
					None => return context.pass(Diagnostic::error().label(span.label())
						.message("unresolved type").note("add a type annotation")),
				}
			}
		},
		ValueNode::Variable(variable) => *scene.values
			.entry(*index).insert(*scene.variables.get(variable)
			.map(|(variable, _)| variable).unwrap()).get(),
		ValueNode::Path(path) => super::path(context,
			scene, value, index, path)?,
		ValueNode::String(string) => {
			let mut value = Value::default();
			let size = ValueNode::Integral(string.len() as i128);
			value.root = value.insert(S::new(size, span.clone()));
			let element = S::new(Type::Unsigned(Size::Byte), span.clone());
			let node = Type::Array(Box::new(element), value);
			scene.ascribe(index, S::new(node, span.clone()))
		}
		ValueNode::Register(_) => {
			let variable = scene.next();
			*scene.values.entry(*index).insert(variable).get()
		}
		ValueNode::Array(elements) => {
			let variable = scene.next();
			for element in elements {
				let other = self::value(context, scene, place, value, element)?;
				scene.unify(context, variable, other, span, &value[other].span);
			}

			// TODO: perform evaluation of array size
			unimplemented!()
		}
		ValueNode::Integral(node) => integral(context, scene, index, node, span.clone())?,
		ValueNode::Truth(_) => scene.ascribe(index, S::new(Type::Truth, span.clone())),
		ValueNode::Rune(_) => scene.ascribe(index, S::new(Type::Rune, span.clone())),
		ValueNode::Break => scene.ascribe(index, S::new(Type::Void, span.clone())),
	})
}

fn dual(context: &Context, scene: &mut Scene, place: Option<&S<TypeVariable>>, value: &Value,
		dual: &Dual, left: &ValueIndex, right: &ValueIndex) -> crate::Result<()> {
	let (left_span, right_span) = (&value[*left].span, &value[*right].span);
	let node = self::value(context, scene, place, value, left)?;
	let node = scene.find(node);
	Ok(match dual {
		Dual::Multiply | Dual::Divide | Dual::Modulo |
		Dual::ShiftLeft | Dual::ShiftRight => {
			integral_type(context, scene, place, value, right)?;
			let right_node = *scene.values.get(right).unwrap();
			scene.unify(context, node, right_node, left_span, right_span);
		}
		_ => match scene.terminals.get(&node) {
			Some(Terminal::Type(S { node: Type::Pointer(_), .. }))
			if dual == &Dual::Add || dual == &Dual::Minus =>
				integral_type(context, scene, place, value, right)?,
			Some(Terminal::Integral(_)) |
			Some(Terminal::Type(S { node: Type::Signed(_), .. })) |
			Some(Terminal::Type(S { node: Type::Unsigned(_), .. })) => {
				let right = self::value(context, scene, place, value, right)?;
				scene.unify(context, node, right, left_span, right_span);
			}
			Some(Terminal::Type(S { node: Type::Truth, .. }))
			if dual != &Dual::Add && dual != &Dual::Minus => {
				let right = self::value(context, scene, place, value, right)?;
				scene.unify(context, node, right, left_span, right_span);
			}
			Some(node) => return context.pass(Diagnostic::error()
				.label(left_span.label().with_message(node.to_string()))
				.message("operation undefined for type")),
			None => return context.pass(Diagnostic::error().label(left_span.label())
				.message("unresolved type").note("add a type annotation")),
		},
	})
}

fn integral_type(context: &Context, scene: &mut Scene, place: Option<&S<TypeVariable>>,
				 value: &Value, index: &ValueIndex) -> crate::Result<()> {
	let index_span = &value[*index].span;
	let index = self::value(context, scene, place, value, index)?;
	let index_type = S::new(Size::Byte, index_span.clone());
	let index_type = scene.next_with(Terminal::Integral(index_type));
	Ok(scene.unify(context, index, index_type, index_span, index_span))
}

fn truth_type(context: &Context, scene: &mut Scene, place: Option<&S<TypeVariable>>,
			  value: &Value, index: &ValueIndex) -> crate::Result<()> {
	let index_span = &value[*index].span;
	let index = self::value(context, scene, place, value, index)?;
	let index_type = S::new(Type::Truth, index_span.clone());
	let index_type = scene.next_with(Terminal::Type(index_type));
	Ok(scene.unify(context, index, index_type, index_span, index_span))
}

fn integral(context: &Context, scene: &mut Scene, index: &ValueIndex,
			value: &i128, span: Span) -> crate::Result<TypeVariable> {
	Ok(scene.terminal(index, Terminal::Integral(S::new(match value {
		_ if (i8::MIN as i128..=i8::MAX as i128).contains(value) => Size::Byte,
		_ if (i16::MIN as i128..=i16::MAX as i128).contains(value) => Size::Word,
		_ if (i32::MIN as i128..=i32::MAX as i128).contains(value) => Size::Double,
		_ if (i64::MIN as i128..=i64::MAX as i128).contains(value) => Size::Quad,
		_ if (u64::MIN as i128..=u64::MAX as i128).contains(value) => Size::Quad,
		_ => return context.pass(Diagnostic::error().label(span.label())
			.message("literal does not fit in signed integral type")
			.note("add a type annotation"))
	}, span))))
}
