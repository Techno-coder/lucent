use std::ops::Index;

use crate::span::S;

use super::{Identifier, Path};

pub type ExpressionIndex = usize;

#[derive(Debug)]
pub struct Function {
	pub is_root: S<bool>,
	pub convention: S<Identifier>,
	pub annotations: Vec<super::Annotation>,
	pub parameters: Vec<Parameter>,
	pub return_type: S<Type>,
	pub expression: Expression,
}

#[derive(Debug)]
pub enum Parameter {
	Register(S<Identifier>),
	Variable(S<Identifier>, S<Type>),
}

#[derive(Debug)]
pub struct Expression {
	pub root: ExpressionIndex,
	expressions: Vec<S<ExpressionNode>>,
}

impl Index<ExpressionIndex> for Expression {
	type Output = S<ExpressionNode>;

	fn index(&self, index: usize) -> &Self::Output {
		self.expressions.get(index).unwrap_or_else(||
			panic!("Expression index: {}, is invalid", index))
	}
}

#[derive(Debug)]
pub enum ExpressionNode {
	Let(S<Identifier>, Option<S<Type>>, ExpressionIndex),
	Set(ExpressionIndex, ExpressionIndex),
	While(ExpressionIndex, ExpressionIndex),
	When(Vec<(ExpressionIndex, ExpressionIndex)>),
	Cast(ExpressionIndex, S<Type>),
	Return(Option<ExpressionIndex>),
	Compile(ExpressionIndex),
	Inline(ExpressionIndex),
	Call(S<Path>, Vec<ExpressionIndex>),
	Binary(Binary, ExpressionIndex, ExpressionIndex),
	Unary(Unary, ExpressionIndex),
	Path(Path),
	String(String),
	Register(Identifier),
	Integral(i128),
	Truth(bool),
	Rune(char),
	Break,
}

#[derive(Debug)]
pub enum Type {
	Void,
	Rune,
	String,
	Truth,
	Never,
	Signed(IntegralSize),
	Unsigned(IntegralSize),
	Pointer(Box<S<Type>>),
	Array(Box<S<Type>>, ExpressionIndex),
}

#[derive(Debug)]
pub enum IntegralSize {
	Byte,
	Word,
	Double,
	Quad,
}

#[derive(Debug)]
pub enum Binary {
	Add,
	Minus,
	Multiply,
	Divide,
	Modulo,
	BinaryOr,
	BinaryAnd,
	ExclusiveOr,
	Less,
	Greater,
	LessEqual,
	GreaterEqual,
	NotEqual,
	Equal,
	And,
	Or,
	ShiftLeft,
	ShiftRight,
}

#[derive(Debug)]
pub enum Unary {
	Not,
	Negate,
	Reference,
	Dereference,
}
