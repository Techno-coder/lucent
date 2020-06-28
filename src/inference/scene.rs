use std::collections::HashMap;
use std::fmt;

use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::{FunctionKind, Size, Type, Value, ValueIndex, ValueNode, Variable};
use crate::span::{S, Span};

pub type TypeVariable = usize;

#[derive(Debug, Clone)]
pub enum Terminal {
	Type(S<Type>),
	Slice(TypeVariable),
	Sequence(TypeVariable),
	Array(TypeVariable, usize),
	Integral(S<Size>),
	Pointer(TypeVariable),
}

impl Terminal {
	fn equal(context: &Context, left: &Self, right: &Self) -> bool {
		use crate::node::Type as NodeType;
		use Terminal::*;
		match (left, right) {
			(Type(left), Type(right)) =>
				NodeType::equal(context, &left.node, &right.node),
			(Slice(left), Slice(right)) => left == right,
			(Sequence(left), Sequence(right)) => left == right,
			(Array(left, left_size), Array(right, right_size)) =>
				left == right && left_size == right_size,
			(Integral(left), Integral(right)) => left.node == right.node,
			(Pointer(left), Pointer(right)) => left == right,
			_ => false,
		}
	}
}

impl fmt::Display for Terminal {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Terminal::Type(node) => write!(f, "{}", node),
			Terminal::Integral(_) => write!(f, "integral"),
			Terminal::Sequence(_) => write!(f, "[_; ?]"),
			Terminal::Array(_, size) => write!(f, "[_; {}]", size),
			Terminal::Slice(_) => write!(f, "[_;]"),
			Terminal::Pointer(_) => write!(f, "*type"),
		}
	}
}

#[derive(Debug, Default)]
pub struct Scene {
	pub parent: Option<crate::query::Key>,
	pub terminals: HashMap<TypeVariable, Terminal>,
	pub parents: HashMap<TypeVariable, TypeVariable>,
	pub values: HashMap<ValueIndex, TypeVariable>,
	pub variables: HashMap<Variable, (TypeVariable, Span)>,
	pub functions: HashMap<ValueIndex, FunctionKind>,
	pub next_type: TypeVariable,
	pub failure: bool,
}

impl Scene {
	pub fn next(&mut self) -> TypeVariable {
		self.next_type += 1;
		self.next_type - 1
	}

	pub fn next_with(&mut self, terminal: Terminal) -> TypeVariable {
		let variable = self.next();
		self.terminals.insert(variable, terminal);
		variable
	}

	pub fn unify(&mut self, context: &Context, left: TypeVariable,
				 right: TypeVariable, left_span: &Span, right_span: &Span) {
		use Terminal::{Integral, Array, Slice, Sequence, Pointer};
		let (left, right) = (self.find(left), self.find(right));
		if left == right { return; }

		match (self.terminals.get(&left), self.terminals.get(&right)) {
			(Some(Integral(left_size)), Some(Integral(right_size))) => {
				let maximum = std::cmp::max(left_size.node, right_size.node);
				let left_node = S::new(maximum, left_size.span.clone());
				self.terminals.insert(left, Terminal::Integral(left_node));
				self.parents.insert(right, left);
				self.terminals.remove(&right);
			}
			(Some(left), Some(right)) if Terminal::equal(context, left, right) => (),
			(Some(Terminal::Type(S { node: Type::Never, .. })), Some(_)) |
			(Some(_), Some(Terminal::Type(S { node: Type::Never, .. }))) => (),
			(Some(Terminal::Type(S { node: Type::Array(left, _left_size), .. })),
				Some(Array(right, _right_size))) => {
				// TODO: perform size equality check
				let left_span = left.span.clone();
				let (left_node, right) = (left.clone(), right.clone());
				let left_node = self.next_with(Terminal::Type(*left_node));
				self.unify(context, left_node, right, &left_span, right_span);
			}
			(Some(Terminal::Type(S { node: Type::Slice(left), .. })), Some(Slice(right))) |
			(Some(Terminal::Type(S { node: Type::Pointer(left), .. })), Some(Pointer(right))) => {
				let left_span = left.span.clone();
				let (left_node, right) = (left.clone(), right.clone());
				let left_node = self.next_with(Terminal::Type(*left_node));
				self.unify(context, left_node, right, &left_span, right_span);
			}
			(Some(Terminal::Type(S { node: Type::Signed(_), .. })), Some(Integral(_))) |
			(Some(Terminal::Type(S { node: Type::Unsigned(_), .. })), Some(Integral(_))) => {
				let terminal = self.terminals.get(&left).cloned();
				self.terminals.insert(right, terminal.unwrap());
			}
			(Some(Slice(left)), Some(Slice(right))) |
			(Some(Sequence(left)), Some(Slice(right))) |
			(Some(Sequence(left)), Some(Array(right, _))) |
			(Some(Sequence(left)), Some(Sequence(right))) |
			(Some(Pointer(left)), Some(Pointer(right))) => {
				let (left, right) = (left.clone(), right.clone());
				self.unify(context, left, right, left_span, right_span)
			}
			(Some(Array(left, left_size)), Some(Array(right, right_size)))
			if left_size == right_size => {
				let (left, right) = (left.clone(), right.clone());
				self.unify(context, left, right, left_span, right_span)
			}
			(Some(Pointer(_)), Some(Terminal::Type(_))) |
			(Some(Integral(_)), Some(Terminal::Type(_))) |
			(Some(Array(_, _)), _) | (Some(Slice(_)), _) =>
				self.unify(context, right, left, right_span, left_span),
			(Some(left), Some(right)) => {
				self.failure = true;
				context.emit(Diagnostic::error()
					.label(left_span.label().with_message(left.to_string()))
					.label(right_span.label().with_message(right.to_string()))
					.message("conflicting types"))
			}
			(Some(_), None) => { self.parents.insert(right, left); }
			(None, Some(_)) => { self.parents.insert(left, right); }
			(None, None) => { self.parents.insert(left, right); }
		}
	}

	pub fn find(&mut self, variable: TypeVariable) -> TypeVariable {
		match self.parents.get(&variable).cloned() {
			None => return variable,
			Some(parent) => {
				let root = self.find(parent);
				*self.parents.entry(variable).insert(root).get()
			}
		}
	}

	pub fn ascribe(&mut self, value: &ValueIndex, node: S<Type>) -> TypeVariable {
		self.terminal(value, Terminal::Type(node))
	}

	pub fn terminal(&mut self, value: &ValueIndex, terminal: Terminal) -> TypeVariable {
		assert!(!self.values.contains_key(&value));
		let variable = self.next_with(terminal);
		self.values.insert(*value, variable);
		variable
	}

	pub fn resolve(&mut self, variable: TypeVariable) -> Option<S<Type>> {
		let root = &self.find(variable);
		Some(match self.terminals.get(root)?.clone() {
			Terminal::Type(node) => node.clone(),
			Terminal::Pointer(node) => {
				let node = self.resolve(node)?;
				let span = node.span.clone();
				S::new(Type::Pointer(Box::new(node)), span)
			}
			Terminal::Slice(node) => {
				let node = self.resolve(node)?;
				let span = node.span.clone();
				S::new(Type::Slice(Box::new(node)), span)
			}
			Terminal::Array(node, size) => {
				let node = self.resolve(node)?;
				let span = node.span.clone();
				let mut value = Value::default();
				let size = ValueNode::Integral(size as i128);
				value.root = value.insert(S::new(size, span.clone()));
				S::new(Type::Array(Box::new(node), value), span)
			}
			Terminal::Integral(size) =>
				S::new(Type::Signed(size.node), size.span),
			Terminal::Sequence(_) => return None,
		})
	}
}

