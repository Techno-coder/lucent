use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::*;
use crate::span::S;

use super::{Scene, Terminal, TypeVariable};

pub fn path(context: &Context, scene: &mut Scene, value: &Value,
			index: &ValueIndex, path: &Path) -> crate::Result<TypeVariable> {
	let span = &value[*index].span;
	if context.statics.contains_key(path) {
		let node = super::type_variable(context, scene.parent
			.clone(), path.clone(), Some(span.clone()))?;
		return Ok(scene.ascribe(index, node));
	}

	context.pass(Diagnostic::error()
		.message("no symbol value at path")
		.label(span.label()))
}

pub fn function_call(context: &Context, scene: &mut Scene, place: Option<&S<TypeVariable>>,
					 value: &Value, index: &ValueIndex, path: &S<Path>, arguments: &[ValueIndex])
					 -> crate::Result<TypeVariable> {
	// TODO: intrinsic function calls

	let span = value[*index].span.clone();
	let arguments: Vec<_> = arguments.iter().map(|index|
		super::value(context, scene, place, value, index)
			.map(|value| (value, index))).collect::<Result<_, _>>()?;
	let candidates = context.functions.get(&path.node).unwrap();
	let candidates: Vec<_> = candidates.iter().enumerate().filter(|(_, candidate)|
		candidate.parameters.len() == arguments.len()).collect();

	if let [(kind, function)] = candidates.as_slice() {
		let iterator = Iterator::zip(arguments.into_iter(), function.parameters.iter());
		for ((argument, index), parameter) in iterator {
			if let Parameter::Variable(_, other) = &parameter.node {
				let other = scene.next_with(Terminal::Type(other.clone()));
				scene.unify(context, argument, other, &value[*index].span, &parameter.span);
			}
		}

		scene.functions.insert(*index, *kind);
		return_type(scene, index, &function.return_type)
	} else {
		let arguments: Vec<_> = arguments.into_iter().map(|(variable, index)|
			scene.resolve(variable).ok_or_else(|| context.error(Diagnostic::error()
				.label(value[*index].span.label()).message("unresolved type")
				.note("add a type annotation")))).collect::<Result<_, _>>()?;
		let mut candidates = candidates.into_iter().filter(|(_, candidate)|
			Iterator::zip(arguments.iter(), candidate.parameters.iter())
				.all(|(argument, parameter)| match &parameter.node {
					Parameter::Variable(_, variable) =>
						Type::equal(context, &argument.node, &variable.node),
					Parameter::Register(_) => true,
				}));

		let (kind, function) = candidates.next().ok_or_else(||
			context.error(Diagnostic::error().label(span.label())
				.message("no matching function")))?;

		if candidates.next().is_some() {
			// TODO: migrate error to declaration type ambiguity verification
			return context.pass(Diagnostic::error().label(span.label())
				.message("ambiguous function call"));
		}

		scene.functions.insert(*index, kind);
		return_type(scene, index, &function.return_type)
	}
}

fn return_type(scene: &mut Scene, index: &ValueIndex,
			   node: &S<ReturnType>) -> crate::Result<TypeVariable> {
	Ok(match &node.node {
		ReturnType::Register(_) => scene.next(),
		ReturnType::Type(node) => scene.ascribe(index, node.clone()),
	})
}
