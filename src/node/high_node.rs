use std::collections::HashMap;

use indexmap::IndexMap;

use crate::FilePath;
use crate::query::{ISpan, S, Span};

use super::*;

pub type GlobalAnnotations = HashMap<Identifier, HGlobalAnnotation>;
pub type HAnnotations = HashMap<Identifier, (ISpan, VIndex)>;
pub type HVariables = IndexMap<Identifier, (ISpan, S<HType>)>;
pub type HFields = HashMap<Identifier, (ISpan, HIndex)>;

#[derive(Debug)]
pub struct HModule {
	pub values: VStore,
	pub annotations: HAnnotations,
	pub span: ISpan,
}

#[derive(Debug, PartialEq)]
pub struct HStatic {
	pub values: VStore,
	pub annotations: HAnnotations,
	pub name: S<Identifier>,
	pub kind: Option<S<HType>>,
	pub value: Option<VIndex>,
}

#[derive(Debug, PartialEq)]
pub struct HData {
	pub values: VStore,
	pub annotations: HAnnotations,
	pub name: S<Identifier>,
	pub fields: HVariables,
}

#[derive(Debug, PartialEq)]
pub struct HFunction {
	pub values: VStore,
	pub annotations: HAnnotations,
	pub name: S<Identifier>,
	pub signature: HSignature,
	pub value: VIndex,
}

#[derive(Debug, PartialEq)]
pub struct HSignature {
	pub convention: Option<S<Identifier>>,
	pub parameters: HVariables,
	pub return_type: S<HType>,
}

#[derive(Debug, PartialEq)]
pub struct HLibrary {
	pub values: VStore,
	pub annotations: HAnnotations,
	pub name: S<Identifier>,
	pub path: FilePath,
}

#[derive(Debug, PartialEq)]
pub struct HLoadFunction {
	pub values: VStore,
	pub library: HPath,
	pub reference: LoadReference,
	pub annotations: HAnnotations,
	pub name: S<Identifier>,
	pub signature: HSignature,
}

#[derive(Debug, PartialEq)]
pub struct HLoadStatic {
	pub values: VStore,
	pub library: HPath,
	pub reference: LoadReference,
	pub annotations: HAnnotations,
	pub name: S<Identifier>,
	pub kind: S<HType>,
}

#[derive(Debug, PartialEq)]
pub struct HGlobalAnnotation {
	pub values: VStore,
	pub value: VIndex,
	pub span: Span,
}

/// A high level abstract syntax tree node that
/// closely resembles source code. All paths and
/// variables are resolved and exist with the
/// exception of the `Unresolved` error variant.
#[derive(Debug, PartialEq)]
pub enum HNode {
	Block(Vec<HIndex>),
	Let(S<Variable>, Option<S<HType>>, Option<HIndex>),
	Set(HIndex, HIndex),
	While(HIndex, HIndex),
	When(Vec<(HIndex, HIndex)>),
	Cast(HIndex, Option<S<HType>>),
	Return(Option<HIndex>),
	Compile(VIndex),
	Inline(VIndex),
	Call(HPath, Vec<HIndex>),
	Method(HIndex, Vec<HIndex>),
	Field(HIndex, S<Identifier>),
	New(HPath, HFields),
	SliceNew(S<HType>, HFields),
	Slice(HIndex, Option<HIndex>, Option<HIndex>),
	Index(HIndex, HIndex),
	Compound(HDual, HIndex, HIndex),
	Binary(HBinary, HIndex, HIndex),
	Unary(Unary, HIndex),
	Variable(Variable),
	Function(HPath),
	Static(HPath),
	String(String),
	Register(Identifier),
	Array(Vec<HIndex>),
	Integral(i128),
	Truth(bool),
	Rune(char),
	Break,
	Continue,
	Unresolved(HPath),
	Error(Vec<HIndex>),
}

#[derive(Debug, PartialEq)]
pub enum HBinary {
	Dual(HDual),
	And,
	Or,
	NotEqual,
	Equal,
	Less,
	Greater,
	LessEqual,
	GreaterEqual,
}

impl HBinary {
	pub fn parse(string: &str) -> Option<Self> {
		Some(match string {
			"&&" => Self::And,
			"||" => Self::Or,
			"!=" => Self::NotEqual,
			"==" => Self::Equal,
			"<" => Self::Less,
			">" => Self::Greater,
			"<=" => Self::LessEqual,
			">=" => Self::GreaterEqual,
			_ => Self::Dual(HDual::parse(string)?),
		})
	}
}

#[derive(Debug, PartialEq)]
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

impl HDual {
	pub fn parse(string: &str) -> Option<Self> {
		Some(match string {
			"+" => Self::Add,
			"-" => Self::Minus,
			"*" => Self::Multiply,
			"/" => Self::Divide,
			"%" => Self::Modulo,
			"|" => Self::BinaryOr,
			"&" => Self::BinaryAnd,
			"^" => Self::ExclusiveOr,
			"<<" => Self::ShiftLeft,
			">>" => Self::ShiftRight,
			_ => return None,
		})
	}
}

#[derive(Debug, PartialEq)]
pub enum HType {
	Void,
	Rune,
	Truth,
	Never,
	Structure(HPath),
	Integral(Sign, Width),
	Pointer(Box<S<HType>>),
	Function(Box<HSignature>),
	Array(Box<S<HType>>, VIndex),
	Slice(Box<S<HType>>),
}
