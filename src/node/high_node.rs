use std::collections::{BTreeMap, HashMap};

use crate::FilePath;
use crate::query::S;

use super::*;

pub type HAnnotations = HashMap<S<Identifier>, HValue>;
pub type HIndex = VIndex<HNode>;
pub type HValue = Value<HNode>;

#[derive(Debug)]
pub struct HModule {
	pub annotations: HAnnotations,
}

#[derive(Debug)]
pub struct HStatic {
	pub annotations: HAnnotations,
	pub name: S<Identifier>,
	pub kind: Option<S<HType>>,
	pub value: Option<HValue>,
}

#[derive(Debug)]
pub struct HFunction {
	pub annotations: HAnnotations,
	pub convention: Option<Identifier>,
	pub name: S<Identifier>,
	pub parameters: Vec<HParameter>,
	pub return_type: S<HType>,
	pub value: HValue,
}

#[derive(Debug)]
pub struct HParameter {
	pub identifier: S<Identifier>,
	pub kind: S<HType>,
}

#[derive(Debug)]
pub struct HLibrary {
	pub annotations: HAnnotations,
	pub path: FilePath,
}

#[derive(Debug)]
pub struct HLoadFunction {
	pub library: Path,
	pub reference: LoadReference,
	pub annotations: HAnnotations,
	pub convention: Option<Identifier>,
	pub name: S<Identifier>,
	pub parameters: Vec<S<HType>>,
	pub return_type: S<HType>,
}

#[derive(Debug)]
pub struct HLoadStatic {
	pub library: Path,
	pub reference: LoadReference,
	pub annotations: HAnnotations,
	pub name: S<Identifier>,
	pub kind: S<HType>,
}

/// A high level abstract syntax tree node
/// that closely resembles source code.
#[derive(Debug, Hash, Eq, PartialEq)]
pub enum HNode {
	Block(Vec<HIndex>),
	Let(S<Variable>, Option<S<HType>>, Option<HIndex>),
	Set(HIndex, HIndex),
	While(HIndex, HIndex),
	When(Vec<(HIndex, HIndex)>),
	Cast(HIndex, S<HType>),
	Return(Option<HIndex>),
	Compile(HValue),
	Inline(HValue),
	Call(S<Path>, Vec<HIndex>),
	Field(HIndex, S<Identifier>),
	New(S<Path>, BTreeMap<S<Identifier>, HIndex>),
	SliceNew(S<Path>, BTreeMap<S<Identifier>, HIndex>),
	Slice(HIndex, Option<HIndex>, Option<HIndex>),
	Index(HIndex, HIndex),
	Compound(HDual, HIndex, HIndex),
	Binary(HBinary, HIndex, HIndex),
	Unary(Unary, HIndex),
	Variable(Variable),
	Path(Path),
	String(String),
	Register(Identifier),
	Array(Vec<HIndex>),
	Integral(i128),
	Truth(bool),
	Rune(char),
	Continue,
	Break,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum HBinary {
	Dual(HDual),
	Less,
	Greater,
	LessEqual,
	GreaterEqual,
	NotEqual,
	Equal,
	And,
	Or,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum HDual {
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

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum HType {
	Void,
	Rune,
	Truth,
	Never,
	Structure(Path),
	Integral(Sign, Width),
	Pointer(Box<S<HType>>),
	Array(Box<S<HType>>, HValue),
	Slice(Box<S<HType>>),
}
