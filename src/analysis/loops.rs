use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::{Function, Value, ValueIndex, ValueNode};

/// Verifies break and continue statements
/// are enclosed within a loop.
pub fn loops(context: &Context, function: &Function) -> crate::Result<()> {
	value(context, &function.value, &function.value.root, false)
}

fn value(context: &Context, value: &Value, index: &ValueIndex,
		 state: bool) -> crate::Result<()> {
	let span = &value[*index].span;
	Ok(match &value[*index].node {
		ValueNode::Block(values) => values.iter()
			.map(|index| self::value(context, value, index, state))
			.filter(Result::is_err).last().unwrap_or(Ok(()))?,
		ValueNode::Let(_, _, Some(index)) =>
			self::value(context, value, index, state)?,
		ValueNode::Set(target, index) => {
			self::value(context, value, target, state)?;
			self::value(context, value, index, state)?;
		}
		ValueNode::While(condition, index) => {
			self::value(context, value, condition, state)?;
			self::value(context, value, index, true)?;
		}
		ValueNode::When(branches) =>
			branches.iter().map(|(condition, index)| {
				self::value(context, value, index, state)?;
				self::value(context, value, condition, state)
			}).filter(Result::is_err).last().unwrap_or(Ok(()))?,
		ValueNode::Cast(index, _) |
		ValueNode::Return(Some(index)) |
		ValueNode::Compile(index) |
		ValueNode::Inline(index) =>
			self::value(context, value, index, true)?,
		ValueNode::Call(_, arguments) => arguments.iter()
			.map(|index| self::value(context, value, index, state))
			.filter(Result::is_err).last().unwrap_or(Ok(()))?,
		ValueNode::Field(index, _) =>
			self::value(context, value, index, true)?,
		ValueNode::Create(_, fields) => fields.values()
			.map(|(index, _)| self::value(context, value, index, state))
			.filter(Result::is_err).last().unwrap_or(Ok(()))?,
		ValueNode::Slice(index, start, end) => {
			self::value(context, value, index, state)?;
			start.iter().try_for_each(|start|
				self::value(context, value, start, state))?;
			end.iter().try_for_each(|end|
				self::value(context, value, end, state))?
		}
		ValueNode::Index(node, index) => {
			self::value(context, value, node, state)?;
			self::value(context, value, index, state)?;
		}
		ValueNode::Compound(_, target, index) => {
			self::value(context, value, target, state)?;
			self::value(context, value, index, state)?;
		}
		ValueNode::Binary(_, left, right) => {
			self::value(context, value, left, state)?;
			self::value(context, value, right, state)?;
		}
		ValueNode::Unary(_, index) =>
			self::value(context, value, index, state)?,
		ValueNode::Continue if !state => return context
			.pass(Diagnostic::error().label(span.label())
				.message("continue not enclosed in loop")),
		ValueNode::Break if !state => return context
			.pass(Diagnostic::error().label(span.label())
				.message("break not enclosed in loop")),
		_ => (),
	})
}
