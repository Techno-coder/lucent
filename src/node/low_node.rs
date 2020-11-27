use crate::query::S;

use super::*;

pub type LReceiver = Receiver<LIndex>;

/// A lower variant of `HNode` that is
/// amenable to code generation. Used in lowering
/// to `SNode`s or stack based code generators
/// such as `wasm`.
///
/// Type information is not preserved.
/// Control flow statements such as `continue`
/// and `break` must be valid at their position.
// TODO: add Compile and Inline
#[derive(Debug)]
pub enum LNode {
	Block(Vec<LIndex>),
	Let(S<LTarget>, S<Size>),
	LetZero(S<LTarget>, S<Size>),
	Set(LIndex, LIndex),
	Loop(LIndex),
	When(Vec<(LIndex, LIndex)>),
	Cast(LIndex, S<Width>),
	Return(LIndex),
	Call(LReceiver, Vec<LIndex>),
	Offset(LIndex, S<usize>),
	Binary(LBinary, Width, LIndex, LIndex),
	Unary(Unary, Width, LIndex),
	Target(S<LTarget>),
	Path(Path),
	String(String),
	Register(Identifier),
	Integral(i128),
	Truth(bool),
	Continue,
	Break,
}

/// Identifies a stack variable.
#[derive(Debug)]
pub struct LTarget(pub usize);

#[derive(Debug)]
pub enum LBinary {
	Add,
	Minus,
	Multiply,
	Divide(Sign),
	Modulo(Sign),
	BinaryOr,
	BinaryAnd,
	ExclusiveOr,
	ShiftLeft,
	ShiftRight,
	Less(Sign),
	Greater(Sign),
	LessEqual(Sign),
	GreaterEqual(Sign),
	NotEqual,
	Equal,
	And,
	Or,
}
