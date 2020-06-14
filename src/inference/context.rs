use std::collections::HashMap;
use std::ops::Index;
use std::sync::Arc;

use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::*;
use crate::query::{Key, QueryError};
use crate::span::{S, Span};

use super::{Scene, Terminal};

#[derive(Debug, Default)]
pub struct Types {
	pub types: HashMap<ValueIndex, Type>,
	pub variables: HashMap<Variable, Type>,
	pub functions: HashMap<ValueIndex, FunctionKind>,
}

impl Index<&ValueIndex> for Types {
	type Output = Type;

	fn index(&self, index: &ValueIndex) -> &Self::Output {
		self.types.get(index).unwrap_or_else(||
			panic!("type for index: {}, is absent", index))
	}
}

pub fn type_function(context: &Context, parent: Option<Key>, path: &Path,
					 kind: FunctionKind, span: Option<Span>) -> crate::Result<Arc<Types>> {
	let key = Key::TypeFunction(path.clone(), kind);
	context.type_contexts.scope(parent, key.clone(), span, || {
		let functions = context.functions.get(path);
		let function = functions.as_ref().and_then(|table|
			table.get(kind)).ok_or(QueryError::Failure)?;
		let mut scene = Scene::default();
		scene.parent = Some(key);

		for parameter in &function.parameters {
			if let Parameter::Variable(variable, node) = parameter.node.clone() {
				let type_variable = scene.next();
				scene.variables.insert(variable.node, (type_variable, variable.span));
				let terminal = Terminal::Type(S::new(node.node, node.span));
				scene.terminals.insert(type_variable, terminal);
			}
		}

		let place = S::new(scene.next(), function.return_type.span.clone());
		if let ReturnType::Type(node) = function.return_type.node.clone() {
			let terminal = S::new(node.node, node.span.clone());
			let variable = scene.next_with(Terminal::Type(terminal));
			scene.unify(context, variable, place.node, &node.span, &place.span);
		}

		let root = super::value(context, &mut scene,
			Some(&place), &function.value, &function.value.root)?;
		match function.return_type.node.clone() {
			ReturnType::Type(S { node: Type::Void, .. }) |
			ReturnType::Type(S { node: Type::Never, .. }) => (),
			ReturnType::Type(node) => {
				let terminal = S::new(node.node, node.span.clone());
				let variable = scene.next_with(Terminal::Type(terminal));
				let span = &function.value[function.value.root].span;
				scene.unify(context, root, variable, span, &node.span);
			}
			_ => (),
		}

		match scene.failure {
			true => Err(QueryError::Failure),
			false => types(context, &function.value, scene),
		}
	})
}

pub fn type_variable(context: &Context, parent: Option<Key>, path: Path,
					 span: Option<Span>) -> crate::Result<S<Type>> {
	let variable = context.statics.get(&path);
	let variable = variable.as_ref().ok_or(QueryError::Failure)?;
	if let Some(node) = &variable.node_type { return Ok(node.clone()); }

	let key = Key::TypeVariable(path.clone());
	context.type_contexts.scope(parent, key.clone(), span, || {
		let mut scene = Scene::default();
		scene.parent = Some(key);

		let value = variable.value.as_ref().unwrap();
		let root = super::value(context, &mut scene, None, value, &value.root)?;
		if let Some(node) = &variable.node_type {
			let other = scene.next_with(Terminal::Type(node.clone()));
			scene.unify(context, root, other, &value[value.root].span, &node.span);
		}

		types(context, value, scene)
	}).map(|types| {
		let node = variable.value.as_ref().unwrap();
		let node = types.types.get(&node.root).unwrap();
		S::new(node.clone(), variable.identifier.span.clone())
	})
}

fn types(context: &Context, value: &Value, mut scene: Scene) -> crate::Result<Types> {
	let mut variables = HashMap::new();
	std::mem::take(&mut scene.variables).into_iter()
		.try_for_each(|(variable, (node, span))| match scene.resolve(node) {
			Some(node) => Ok(variables.insert(variable, node.node).unwrap_none()),
			None => context.pass(Diagnostic::error().label(span.label())
				.message("unresolved type").note("add a type annotation")),
		})?;

	let mut types = HashMap::new();
	std::mem::take(&mut scene.values).into_iter()
		.try_for_each(|(index, node)| match scene.resolve(node) {
			Some(node) => Ok(types.insert(index, node.node).unwrap_none()),
			None => context.pass(Diagnostic::error().label(value[index].span.label())
				.message("unresolved type").note("add a type annotation")),
		})?;

	let functions = scene.functions;
	Ok(Types { types, variables, functions })
}
