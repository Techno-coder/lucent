use std::collections::HashMap;
use std::sync::Arc;

use crate::query::S;

use super::*;

pub type LReceiver = Receiver<BNode>;
type BNode = Box<S<LNode>>;
type BUnit = Box<S<LUnit>>;

#[derive(Debug)]
pub struct LFunction {
	pub unit: S<LUnit>,
	pub parameters: HashMap<Identifier, LTarget>,
	pub locals: HashMap<LTarget, S<Size>>,
}

#[derive(Debug)]
pub struct LValue {
	pub value: LValued,
	pub locals: HashMap<LTarget, S<Size>>,
}

#[derive(Debug)]
pub enum LValued {
	Node(S<LNode>),
	Block(S<LUnit>),
}

/// A lower variant of `HNode` that is
/// amenable to code generation. Used in lowering
/// to `SNode`s or stack based code generators
/// such as `wasm`. Type information is not
/// preserved. Blocks may be empty.
#[derive(Debug, Clone)]
pub enum LNode {
	Block(Box<[S<LUnit>]>, BNode),
	If(BNode, BNode, Option<BNode>),
	Call(LReceiver, Vec<S<LNode>>),
	Cast(BNode, (Sign, Width), Width),
	Binary(LBinary, Width, BNode, BNode),
	Unary(LUnary, Width, BNode),
	Dereference(LPlace),
	Compile(VIndex),
	Target(LTarget),
	Function(FPath),
	Static(Arc<Path>),
	Register(Register),
	String(Arc<str>),
	Integral(i128),
	Never(BUnit),
}

/// The imperative complement to `LNode`s.
/// Control flow statements such as `continue`
/// and `break` must be valid at their position.
/// Blocks may be empty. No mutations are made
/// on zero sized values.
#[derive(Debug, Clone)]
pub enum LUnit {
	Block(Box<[S<LUnit>]>),
	If(S<LNode>, BUnit, Option<BUnit>),
	Call(LReceiver, Vec<S<LNode>>),
	Return(Option<S<LNode>>),
	Set(LPlace, S<LNode>),
	Zero(LTarget),
	Loop(BUnit),
	Compile(VIndex),
	Inline(VIndex),
	Node(S<LNode>),
	Continue,
	Break,
}

/// An assignable value.
/// The enclosed node must evaluate to
/// an architecture compatible pointer.
#[derive(Debug, Clone)]
pub struct LPlace(pub BNode);

/// Identifies a stack variable.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct LTarget(pub usize);

#[derive(Debug, Copy, Clone)]
pub enum LUnary {
	Not,
	Negate,
}

#[derive(Debug, Copy, Clone)]
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
