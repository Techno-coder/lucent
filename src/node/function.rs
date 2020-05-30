use std::ops::Index;

use crate::span::S;

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
			panic!("Value index: {}, is invalid", index))
	}
}

#[derive(Debug, Clone)]
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
	Compile(ValueIndex),
	Inline(ValueIndex),
	Call(S<Path>, Vec<ValueIndex>),
	Field(ValueIndex, S<Identifier>),
	Create(S<Path>, Vec<(S<Identifier>, ValueIndex)>),
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
	Signed(IntegralSize),
	Unsigned(IntegralSize),
	Pointer(Box<S<Type>>),
	Array(Box<S<Type>>, Value),
	Slice(Box<S<Type>>),
}

#[derive(Debug, Copy, Clone)]
pub enum IntegralSize {
	Byte,
	Word,
	Double,
	Quad,
}

#[derive(Debug, Copy, Clone)]
pub enum Binary {
	Dual(Dual),
	Less,
	Greater,
	LessEqual,
	GreaterEqual,
	NotEqual,
	Equal,
	And,
	Or,
}

impl Binary {
	pub fn parse(string: &str) -> Option<Binary> {
		Some(match string {
			"&&" => Binary::And,
			"||" => Binary::Or,
			"==" => Binary::Equal,
			"!=" => Binary::NotEqual,
			"<" => Binary::Less,
			">" => Binary::Greater,
			"<=" => Binary::LessEqual,
			">=" => Binary::GreaterEqual,
			_ => Binary::Dual(Dual::parse(string)?),
		})
	}
}

#[derive(Debug, Copy, Clone)]
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
pub enum Unary {
	Not,
	Negate,
	Reference,
	Dereference,
}
