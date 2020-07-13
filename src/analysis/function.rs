use crate::context::Context;
use crate::node::{Function, FunctionPath};
use crate::query::Key;
use crate::span::Span;

pub fn function(context: &Context, parent: Option<Key>, path: &FunctionPath,
				function: &Function, span: Option<Span>) -> crate::Result<()> {
	let key = Key::Analyze(path.clone());
	context.unit.scope(parent, key, span, ||
		super::loops(context, function)).map(std::mem::drop)
}
