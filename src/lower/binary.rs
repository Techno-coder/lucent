use crate::node::*;
use crate::query::{E, ISpan, QueryError, S};

use super::{pointer, Scene};

pub fn binary(scene: &mut Scene, binary: &HBinary,
			  left_node: &HIndex, right_node: &HIndex,
			  looped: bool, span: ISpan) -> crate::Result<S<LNode>> {
	let duals = |dual, sign| match dual {
		HDual::Add => LBinary::Add,
		HDual::Minus => LBinary::Minus,
		HDual::Multiply => LBinary::Multiply,
		HDual::Divide => LBinary::Divide(sign),
		HDual::Modulo => LBinary::Modulo(sign),
		HDual::BinaryOr => LBinary::BinaryOr,
		HDual::BinaryAnd => LBinary::BinaryAnd,
		HDual::ExclusiveOr => LBinary::ExclusiveOr,
		HDual::ShiftLeft => LBinary::ShiftLeft,
		HDual::ShiftRight => LBinary::ShiftRight,
	};

	let create = |scene: &mut Scene, binary, width| {
		let left = Box::new(super::lower(scene, left_node, looped)?);
		let right = Box::new(super::lower(scene, right_node, looped)?);
		Ok(S::new(LNode::Binary(binary, width, left, right), span))
	};

	macro_rules! integral {
	    ($left:expr, $operator:pat, $sign:ident => $low:expr) => {
			if let ($operator, RType::Integral($sign, width)) = (binary, &$left.node) {
				return create(scene, $low, *width);
			}

			if let ($operator, RType::IntegralSize(target, $sign)) = (binary, &$left.node) {
				let width = pointer(scene, target, span)?;
				return create(scene, $low, width);
			}
	    }
	}

	macro_rules! equal {
	    ($left:expr, $kind:pat => $width:expr) => {
	    	if let (HBinary::Equal, $kind) = (binary, &$left.node) {
	    		let width = $width;
	    		return create(scene, LBinary::Equal, width);
	    	}

	    	if let (HBinary::NotEqual, $kind) = (binary, &$left.node) {
	    		let width = $width;
	    		return create(scene, LBinary::NotEqual, width);
	    	}
	    };
	}

	let left = bail!(scene.types.nodes.get(left_node))?;
	integral!(left, HBinary::Less, sign => LBinary::Less(*sign));
	integral!(left, HBinary::Greater, sign => LBinary::Greater(*sign));
	integral!(left, HBinary::LessEqual, sign => LBinary::LessEqual(*sign));
	integral!(left, HBinary::GreaterEqual, sign => LBinary::GreaterEqual(*sign));
	integral!(left, HBinary::Dual(dual), sign => duals(*dual, *sign));

	equal!(left, RType::Rune => Width::D);
	equal!(left, RType::Truth => Width::B);
	equal!(left, RType::Integral(_, width) => *width);
	equal!(left, RType::IntegralSize(target, _) => pointer(scene, target, span)?);
	equal!(left, RType::Function(function) => pointer(scene, &function.target, span)?);
	equal!(left, RType::Pointer(target, _) => pointer(scene, target, span)?);

	let (binary, width) = match (binary, &left.node) {
		(HBinary::Or, RType::Truth) => (LBinary::Or, Width::B),
		(HBinary::And, RType::Truth) => (LBinary::And, Width::B),
		(HBinary::Dual(HDual::Add), RType::Pointer(target, _)) =>
			(LBinary::Add, pointer(scene, target, span)?),
		(HBinary::Dual(HDual::Minus), RType::Pointer(target, _)) =>
			(LBinary::Minus, pointer(scene, target, span)?),
		(_, _) => invalid_binary(scene, &left.node, span)?,
	};

	create(scene, binary, width)
}

pub fn invalid_binary<T>(scene: &mut Scene, left: &RType,
						 span: ISpan) -> crate::Result<T> {
	let error = E::error().message("invalid binary operation");
	error.label(span.other().message(left)).result(scene.scope)
}
