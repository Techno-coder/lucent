use std::fmt;

use crate::node::{HType, RType, Sign, Value};
use crate::query::{E, IScope, ISpan, S};

use super::Types;

pub const TRUTH: IType = truth();
pub const INDEX: IType = index();

#[derive(Debug, Clone)]
pub enum IType {
	Type(S<RType>),
	Sequence(S<RType>),
	IntegralSize,
}

impl IType {
	pub fn span(&self) -> ISpan {
		match self {
			IType::Type(node) => node.span,
			IType::Sequence(node) => node.span,
			IType::IntegralSize => ISpan::internal(),
		}
	}
}

impl fmt::Display for IType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			IType::Type(kind) => write!(f, "{}", kind.node),
			IType::Sequence(kind) => write!(f, "[{}; ?]", kind.node),
			IType::IntegralSize => write!(f, "<size>"),
		}
	}
}

#[derive(Debug)]
pub struct Scene<'a, 'b, 'c> {
	pub scope: IScope<'c, 'b, 'a>,
	pub return_type: Option<S<RType>>,
	pub value: &'a Value,
	pub types: Types,
}

impl Scene<'_, '_, '_> {
	pub fn unknown<T>(&mut self, span: ISpan) -> Option<T> {
		E::error().message("type annotations needed")
			.label(span.label()).result(self.scope).ok()
	}

	pub fn lift(&mut self, kind: &S<HType>) -> Option<S<RType>> {
		super::lift(self.scope, kind).ok()
	}
}

const fn truth() -> IType {
	IType::Type(S::new(RType::Truth, ISpan::internal()))
}

const fn index() -> IType {
	let kind = RType::IntegralSize(Sign::Unsigned);
	IType::Type(S::new(kind, ISpan::internal()))
}

pub fn raise(kind: S<RType>) -> IType {
	IType::Type(kind)
}
