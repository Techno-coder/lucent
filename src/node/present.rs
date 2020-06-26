use std::collections::HashSet;

use crate::context::Context;
use crate::query::{Key, QueryError};
use crate::span::Span;

use super::{FunctionPath, ValueNode};

pub fn present_all(context: &Context) -> crate::Result<()> {
	let mut present = HashSet::new();
	context.unit.ephemeral(None, Key::TraverseRoots, None, || context.functions.iter()
		.try_for_each(|path| path.value().iter().filter(|function| function.is_root)
			.enumerate().map(|(kind, _)| FunctionPath(path.key().clone(), kind))
			.try_for_each(|path| Ok(if !present.contains(&path) {
				present.insert(path.clone());
				self::function(context, &mut present, &path)?
			})))).map(|_| *context.present.write() = present)
}

pub fn present(context: &Context, parent: Option<Key>, path: &FunctionPath,
			   span: Option<Span>) -> crate::Result<bool> {
	let mut present = false;
	context.unit.ephemeral(parent, Key::TraverseRoots, span,
		|| Ok(present = context.present.read().contains(path)))?;
	Ok(present)
}

fn function(context: &Context, present: &mut HashSet<FunctionPath>,
			path: &FunctionPath) -> crate::Result<()> {
	let FunctionPath(function, kind) = path;
	let functions = context.functions.get(function);
	let function = functions.as_ref().and_then(|table|
		table.get(*kind)).ok_or(QueryError::Failure)?;

	let types = crate::inference::type_function(context, None, path, None)?;
	types.functions.iter().try_for_each(|(index, kind)| {
		let path = match &function.value[*index].node {
			ValueNode::Call(path, _) => path.clone(),
			_ => panic!("value: {}, is not a function call", index),
		};

		let function = FunctionPath(path.node, *kind);
		Ok(if !present.contains(&function) {
			present.insert(function.clone());
			self::function(context, present, &function)?;
		})
	})
}
