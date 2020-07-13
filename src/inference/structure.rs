use std::collections::HashMap;

use crate::context::Context;
use crate::error::Diagnostic;
use crate::inference::Terminal;
use crate::node::{Identifier, Path, Type, Value, ValueIndex};
use crate::query::QueryError;
use crate::span::{S, Span};

use super::{Scene, TypeVariable};

pub fn field(context: &Context, scene: &mut Scene, place: Option<&S<TypeVariable>>,
			 value: &Value, index: &ValueIndex, node: &ValueIndex,
			 field: &S<Identifier>) -> crate::Result<TypeVariable> {
	let span = &value[*index].span;
	let node = super::value(context, scene, place, value, node)?;
	let node = scene.resolve(node).ok_or_else(||
		context.error(Diagnostic::error().label(span.label())
			.message("unresolved type").note("add a type annotation")))?;

	match &node.node {
		Type::Structure(path) => {
			let data = context.structures.get(path);
			let data = data.as_ref().ok_or(QueryError::Failure)?;
			match data.fields.get(&field.node) {
				None => context.pass(Diagnostic::error()
					.message(format!("structure has no field: {}", field))
					.label(field.span.label())),
				Some(field) => Ok(scene.ascribe(index, field.clone())),
			}
		}
		other => context.pass(Diagnostic::error()
			.label(span.label().with_message(other.to_string()))
			.message("type is not a structure")),
	}
}

pub fn create(context: &Context, scene: &mut Scene, place: Option<&S<TypeVariable>>,
			  value: &Value, index: &ValueIndex, fields: &HashMap<Identifier, (ValueIndex, Span)>,
			  path: &S<Path>) -> crate::Result<TypeVariable> {
	let data = context.structures.get(&path.node);
	let data = data.as_ref().ok_or(QueryError::Failure)?;
	fields.iter().map(|(field, (_, span))| (field, span))
		.filter(|(field, _)| !data.fields.contains_key(*field))
		.try_for_each(|(_, span)| context.pass(Diagnostic::error()
			.message("field not in structure").label(span.label())))?;

	for (field, node) in &data.fields {
		if let Some((other, span)) = fields.get(field) {
			let other = super::value(context, scene, place, value, other)?;
			let node_type = scene.next_with(Terminal::Type(node.clone()));
			scene.unify(context, other, node_type, span, &node.span);
		}
	}

	let node = Type::Structure(path.node.clone());
	Ok(scene.ascribe(index, S::new(node, path.span.clone())))
}
