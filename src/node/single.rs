use crate::query::S;

use super::*;

pub type SReceiver = Receiver<SIndex>;

#[derive(Debug)]
pub struct SFunction {
	pub frame_size: usize,
	pub convention: Convention,
}

/// A linear variant of `LNode` in the form of
/// single static assignment (SSA) statements.
///
/// A control flow graph is not necessary as all
/// mutating assignments outside of an enclosing
/// scope are performed on functionally global
/// stack variables.
// TODO: associate STarget with register constraints
#[derive(Debug)]
pub enum SNode {
	Block(Vec<SIndex>),
	Load(STarget, SOffset),
	Store(SOffset, STarget),
	Integral(STarget, i128),
	Let(STarget, LBinary, STarget, STarget),
	Call(Option<STarget>, SReceiver, Vec<SPlace>),
	If(STarget, SIndex, Option<SIndex>),
	Return(Option<SPlace>),
	Loop(SIndex),
	Continue,
	Break,
	Never,
}

/// Represents a movable value.
#[derive(Debug)]
pub enum SPlace {
	Target(STarget),
	Stack(SOffset, Size),
}

/// Offset in bytes from base of stack frame.
#[derive(Debug)]
pub struct SOffset(pub usize);

/// A temporary register with an associated
/// identifier and target width.
#[derive(Debug, Hash, Eq, PartialEq)]
pub struct STarget(pub usize, pub Width);
