use std::sync::Arc;

use crate::context::Context;
use crate::node::{Function, FunctionPath};
use crate::query::{Key, QueryError};
use crate::span::Span;

pub fn function(context: &Context, parent: Option<Key>, path: &FunctionPath,
				span: Option<Span>) -> crate::Result<Arc<Function>> {
	let FunctionPath(function, kind) = path;
	let functions = context.functions.get(&function);
	let function = functions.as_ref().and_then(|table|
		table.get(*kind).cloned()).ok_or(QueryError::Failure)?;
	crate::analysis::function(context, parent, path, &function, span)?;
	Ok(function)
}
