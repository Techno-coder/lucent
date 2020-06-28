use std::collections::HashMap;
use std::fmt;
use std::ops::Index;

use crate::context::Context;
use crate::span::{S, Span};

use super::{Identifier, Path};

pub type ValueIndex = usize;

#[derive(Debug, Default, Clone)]
pub struct Value {
	pub root: ValueIndex,
	values: Vec<S<ValueNode>>,
}

impl Value {
	pub fn insert(&mut self, node: S<ValueNode>) -> ValueIndex {
		let index = self.values.len();
		self.values.push(node);
		index
	}
}

impl Index<ValueIndex> for Value {
	type Output = S<ValueNode>;

	fn index(&self, index: usize) -> &Self::Output {
		self.values.get(index).unwrap_or_else(||
			panic!("value index: {}, is invalid", index))
	}
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Variable(pub Identifier, pub u16);

#[derive(Debug, Clone)]
pub enum ValueNode {
	Block(Vec<ValueIndex>),
	Let(S<Variable>, Option<S<Type>>, Option<ValueIndex>),
	Set(ValueIndex, ValueIndex),
	While(ValueIndex, ValueIndex),
	When(Vec<(ValueIndex, ValueIndex)>),
	Cast(ValueIndex, S<Type>),
	Return(Option<ValueIndex>),
	// TODO: move compile to separate value
	Compile(ValueIndex),
	Inline(ValueIndex),
	Call(S<Path>, Vec<ValueIndex>),
	Field(ValueIndex, S<Identifier>),
	Create(S<Path>, HashMap<Identifier, (ValueIndex, Span)>),
	Slice(ValueIndex, Option<ValueIndex>, Option<ValueIndex>),
	Index(ValueIndex, ValueIndex),
	Compound(Dual, ValueIndex, ValueIndex),
	Binary(Binary, ValueIndex, ValueIndex),
	Unary(Unary, ValueIndex),
	Variable(Variable),
	Path(Path),
	String(String),
	Register(Identifier),
	Array(Vec<ValueIndex>),
	Integral(i128),
	Truth(bool),
	Rune(char),
	Break,
}

#[derive(Debug, Clone)]
pub enum Type {
	Void,
	Rune,
	Truth,
	Never,
	Structure(Path),
	Signed(Size),
	Unsigned(Size),
	Pointer(Box<S<Type>>),
	Array(Box<S<Type>>, Value),
	Slice(Box<S<Type>>),
}

impl Type {
	pub fn equal(context: &Context, left: &Self, right: &Self) -> bool {
		use Type::*;
		match (left, right) {
			(Void, Void) => true,
			(Rune, Rune) => true,
			(Truth, Truth) => true,
			(Never, Never) => true,
			(Structure(left), Structure(right)) => left == right,
			(Signed(left), Signed(right)) => left == right,
			(Unsigned(left), Unsigned(right)) => left == right,
			(Pointer(left), Pointer(right)) => Type::equal(context, &left.node, &right.node),
			// TODO: perform array size evaluations
			(Array(left, _), Array(right, _)) => Type::equal(context, &left.node, &right.node),
			(Slice(left), Slice(right)) => Type::equal(context, &left.node, &right.node),
			_ => false,
		}
	}
}

impl fmt::Display for Type {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Type::Void => write!(f, "void"),
			Type::Rune => write!(f, "rune"),
			Type::Truth => write!(f, "truth"),
			Type::Never => write!(f, "never"),
			Type::Structure(path) => write!(f, "{}", path),
			Type::Signed(size) => write!(f, "i{}", size),
			Type::Unsigned(size) => write!(f, "u{}", size),
			Type::Pointer(node) => write!(f, "*{}", node),
			Type::Array(node, value) => match value[value.root].node {
				ValueNode::Integral(size) => write!(f, "[{}; {}]", node, size),
				_ => write!(f, "[{}; _]", node),
			}
			Type::Slice(node) => write!(f, "[{};]", node),
		}
	}
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Size {
	Byte = 8,
	Word = 16,
	Double = 32,
	Quad = 64,
}

impl Size {
	pub fn bytes(self) -> usize {
		(self as usize) / 8
	}
}

impl fmt::Display for Size {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", *self as usize)
	}
}

#[derive(Debug, Copy, Clone)]
pub enum Binary {
	Dual(Dual),
	Compare(Compare),
	And,
	Or,
}

impl Binary {
	pub fn parse(string: &str) -> Option<Binary> {
		Some(match string {
			"||" => Binary::Or,
			"&&" => Binary::And,
			"==" => Binary::Compare(Compare::Equal),
			"!=" => Binary::Compare(Compare::NotEqual),
			"<" => Binary::Compare(Compare::Less),
			">" => Binary::Compare(Compare::Greater),
			"<=" => Binary::Compare(Compare::LessEqual),
			">=" => Binary::Compare(Compare::GreaterEqual),
			_ => Binary::Dual(Dual::parse(string)?),
		})
	}
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Dual {
	Add,
	Minus,
	Multiply,
	Divide,
	Modulo,
	BinaryOr,
	BinaryAnd,
	ExclusiveOr,
	ShiftLeft,
	ShiftRight,
}

impl Dual {
	pub fn parse(string: &str) -> Option<Dual> {
		Some(match string {
			"+" => Dual::Add,
			"-" => Dual::Minus,
			"*" => Dual::Multiply,
			"/" => Dual::Divide,
			"%" => Dual::Modulo,
			"&" => Dual::BinaryAnd,
			"^" => Dual::ExclusiveOr,
			"|" => Dual::BinaryOr,
			"<<" => Dual::ShiftLeft,
			">>" => Dual::ShiftRight,
			_ => return None,
		})
	}
}

#[derive(Debug, Copy, Clone)]
pub enum Compare {
	Less,
	Greater,
	LessEqual,
	GreaterEqual,
	NotEqual,
	Equal,
}

#[derive(Debug, Copy, Clone)]
pub enum Unary {
	Not,
	Negate,
	Reference,
	Dereference,
}
