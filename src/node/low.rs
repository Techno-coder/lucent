use crate::query::S;

use super::*;

pub type LReceiver = Receiver<BNode>;
pub type LNode = S<LowNode>;
type BNode = Box<LNode>;

#[derive(Debug)]
pub struct LFunction {
	pub node: LNode,
}

/// A lower variant of `HNode` that is
/// amenable to code generation. Used in lowering
/// to `SNode`s or stack based code generators
/// such as `wasm`.
///
/// Type information is not preserved.
/// Control flow statements such as `continue`
/// and `break` must be valid at their position.
#[derive(Debug)]
pub enum LowNode {
	Block(Vec<LNode>),
	Let(S<LTarget>, S<Size>),
	LetZero(S<LTarget>, S<Size>),
	Set(BNode, BNode),
	Loop(BNode),
	If(BNode, BNode, Option<BNode>),
	Cast(BNode, S<Width>),
	Return(BNode),
	Call(LReceiver, Vec<LNode>),
	Offset(BNode, S<usize>),
	Binary(LBinary, Width, BNode, BNode),
	Unary(Unary, Width, BNode),
	Compile(VPath),
	Inline(VPath),
	Target(S<LTarget>),
	Function(FPath),
	Static(Path),
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
