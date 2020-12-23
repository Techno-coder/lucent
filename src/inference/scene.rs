use std::fmt;

use crate::generate::Target;
use crate::node::{HType, RType, Sign, Value};
use crate::query::{E, IScope, ISpan, S};

use super::Types;

pub const TRUTH: IType = truth();

#[derive(Debug, Clone)]
pub enum IType {
	Type(S<RType>),
	Sequence(Option<Target>, S<RType>),
	IntegralSize(Option<Target>),
}

impl IType {
	pub fn span(&self) -> ISpan {
		match self {
			IType::Type(node) => node.span,
			IType::Sequence(_, node) => node.span,
			IType::IntegralSize(_) => ISpan::internal(),
		}
	}
}

impl fmt::Display for IType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			IType::Type(kind) => write!(f, "{}", kind.node),
			IType::IntegralSize(target) => {
				crate::node::prefix(f, target)?;
				write!(f, "<size>")
			}
			IType::Sequence(target, kind) => {
				crate::node::prefix(f, target)?;
				write!(f, "[{}; ?]", kind.node)
			}
		}
	}
}

#[derive(Debug)]
pub struct Scene<'a, 'b, 'c> {
	pub scope: IScope<'c, 'b, 'a>,
	pub return_type: Option<S<RType>>,
	pub target: Option<Target>,
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

pub fn index(target: Option<Target>) -> IType {
	let kind = RType::IntegralSize(target, Sign::Unsigned);
	raise(S::new(kind, ISpan::internal()))
}

pub fn raise(kind: S<RType>) -> IType {
	IType::Type(kind)
}
