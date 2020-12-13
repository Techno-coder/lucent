use std::fmt::Display;

use crate::node::{FPath, HBinary, HDual, HIndex, HNode, RType, Sign, Unary, VPath};
use crate::query::{E, ISpan, S};

use super::{IType, Scene};

macro_rules! kind {
    ($pattern:pat) => {
		IType::Type(S { node: $pattern, .. })
	};

    ($span:ident, $pattern:pat) => {
		IType::Type(S { node: $pattern, span: $span })
    };
}

pub fn checked(scene: &mut Scene, index: &HIndex, kind: Option<S<RType>>) {
	kind.map(|kind| check(scene, index, super::raise(kind)));
}

/// Asserts that a node has a given type. The scene
/// must not be updated if the expected type does not match.
pub fn check(scene: &mut Scene, index: &HIndex, kind: IType) {
	let span = scene.value[*index].span;
	let insert = |scene: &mut Scene, kind: S<RType>| scene
		.types.nodes.insert(*index, kind).unwrap_none();
	let conflict = |kind: IType, found: &dyn Display| {
		let expected = format!("expected: {}", kind);
		let note = format!("{}, found: {}", expected, found);
		E::error().message("mismatched types")
			.label(span.label().message(note))
			.label(kind.span().other())
	};

	let node = &scene.value[*index].node;
	(|| Some(match (node, kind) {
		(_, kind!(RType::Void)) => drop(super::synthesize(scene, index)),
		(HNode::Block(nodes), IType::Type(kind)) => {
			let (last, nodes) = nodes.split_last().unwrap();
			nodes.iter().for_each(|node| drop(super::synthesize(scene, node)));
			check(scene, last, super::raise(kind.clone()));
			insert(scene, kind);
		}
		(HNode::When(branches), IType::Type(kind)) => {
			let mut complete = false;
			for (condition, node) in branches {
				let other = &scene.value[*condition].node;
				complete |= matches!(other, HNode::Truth(true));
				super::check(scene, node, super::raise(kind.clone()));
				super::check(scene, condition, super::TRUTH);
			}

			let void = S::new(RType::Void, ISpan::internal());
			match complete || unifies(&kind, &void) {
				true => insert(scene, kind.clone()),
				false => conflict(IType::Type(kind), &void.node)
					.note("expression is missing default branch")
					.note("add a branch with condition: true")
					.emit(scene.scope),
			}
		}
		(HNode::Slice(node, left, right), kind!(RType::Slice(target, box kind))) |
		(HNode::Slice(node, left, right), IType::Sequence(target, kind)) => {
			left.as_ref().map(|node| check(scene, node, super::TRUTH));
			right.as_ref().map(|node| check(scene, node, super::TRUTH));
			check(scene, node, IType::Sequence(target.clone(), kind.clone()));
			let kind = RType::Slice(target, Box::new(kind));
			insert(scene, S::new(kind, span));
		}
		(HNode::Array(nodes), IType::Sequence(_, kind)) => {
			nodes.iter().for_each(|node| check(scene,
				node, IType::Type(kind.clone())));
			let kind = RType::Array(Box::new(kind), nodes.len());
			insert(scene, S::new(kind, span));
		}
		(HNode::Array(nodes), kind!(other, RType::Array(box kind, size))) => {
			nodes.iter().for_each(|node| check(scene,
				node, IType::Type(kind.clone())));
			if nodes.len() == size {
				let kind = RType::Array(Box::new(kind), nodes.len());
				return Some(insert(scene, S::new(kind, other)));
			}

			let expected = format!("expected array size: {}", size);
			let found = format!("found size: {}", nodes.len());
			let note = format!("{}, {}", expected, found);
			E::error().message("mismatched types")
				.label(span.label().message(note))
				.emit(scene.scope);
		}
		(HNode::Index(node, index), IType::Type(kind)) => {
			let target = scene.target.clone();
			check(scene, index, super::INDEX);
			check(scene, node, IType::Sequence(target, kind.clone()));
			insert(scene, kind);
		}
		(HNode::Function(path), kind!(other, RType::Function(function))) => {
			let function = S::new(RType::Function(function), other);
			let (path, scope) = (path.path(), &mut scene.scope.span(span));
			let candidates = super::signatures(scope, &path).ok()?;
			let candidates: Vec<_> = candidates.into_iter().map(|candidate|
				S::new(RType::Function(Box::new(candidate)), ISpan::internal()))
				.enumerate().filter(|(_, kind)| unifies(&function, &kind)).collect();

			if candidates.len() == 1 {
				let candidate = candidates.into_iter().next();
				let (target, signature) = candidate.unwrap();
				scene.types.functions.insert(*index, target);
				insert(scene, signature);
			} else if candidates.is_empty() {
				let note = format!("expected: {}", function.node);
				E::error().message("no matching function")
					.label(span.label().message(note))
					.label(other.other()).result(scene.scope).ok()?
			} else {
				let error = E::error().message("ambiguous function");
				let error = candidates.iter().fold(error, |error, (index, _)|
					error.note(FPath(path.clone(), *index).to_string()));
				error.label(span.label()).result(scene.scope).ok()?
			}
		}
		(HNode::Binary(HBinary::Dual(HDual::Add | HDual::Minus), left, right),
			IType::Type(kind @ S { node: RType::Pointer(_, _), .. })) => {
			check(scene, left, IType::Type(kind.clone()));
			check(scene, right, IType::IntegralSize);
			insert(scene, kind);
		}
		(HNode::Binary(HBinary::Dual(_), left, right), IType::Type(kind)) => {
			check(scene, left, IType::Type(kind.clone()));
			check(scene, right, IType::Type(kind.clone()));
			insert(scene, kind);
		}
		(HNode::Unary(Unary::Dereference, node), IType::Type(kind)) => {
			let target = scene.target.clone();
			let pointer = RType::Pointer(target, Box::new(kind.clone()));
			check(scene, node, IType::Type(S::new(pointer, span)));
			insert(scene, kind);
		}
		(HNode::Unary(Unary::Reference, node), kind!(RType::Pointer(_, box kind))) |
		(HNode::Unary(Unary::Not | Unary::Negate, node), IType::Type(kind)) => {
			check(scene, node, IType::Type(kind.clone()));
			insert(scene, kind);
		}
		(HNode::Cast(node, None), IType::Type(kind)) => {
			super::synthesize(scene, node);
			insert(scene, kind);
		}
		(HNode::Integral(_), IType::IntegralSize) =>
			insert(scene, S::new(RType::IntegralSize(Sign::Unsigned), span)),
		(HNode::Integral(_), kind!(span, RType::IntegralSize(sign))) =>
			insert(scene, S::new(RType::IntegralSize(sign), span)),
		(HNode::Integral(_), kind!(span, RType::Integral(sign, width))) =>
			insert(scene, S::new(RType::Integral(sign, width), span)),
		(HNode::Integral(_), kind) =>
			conflict(kind, &"<integral>").emit(scene.scope),
		(HNode::Register(_), IType::Type(kind)) => insert(scene, kind),
		(HNode::Inline(_), IType::Type(kind)) => insert(scene, kind),
		(HNode::Compile(index), kind) => {
			let path = VPath(scene.scope.symbol.clone(), *index);
			let scope = &mut scene.scope.span(span);
			let value = crate::parse::value(scope, &path).ok()?;
			let types = super::hint(scope, &path, Some(kind)).ok()?;
			insert(scene, types.nodes.get(&value.root).cloned()?);
		}
		(_, kind) => {
			let target = super::synthesize(scene, index)?;
			if !unify(&target, &kind) {
				let error = conflict(kind, &target.node);
				match target.span != scene.value[*index].span {
					true => error.label(target.span.other()
						.message("type originates here")),
					false => error,
				}.emit(scene.scope);
			}
		}
	}))();
}

pub fn unifies(left: &S<RType>, right: &S<RType>) -> bool {
	use crate::node::Type::*;
	match (&left.node, &right.node) {
		(Truth, Truth) | (Rune, Rune) => true,
		(Void, Void) | (_, Never) | (Never, _) => true,
		(Structure(_), _) => left.node == right.node,
		(Integral(_, _), _) => left.node == right.node,
		(IntegralSize(_), _) => left.node == right.node,
		(Array(left_kind, left_size), Array(right_kind, right_size)) =>
			left_size == right_size && unifies(&left_kind, &right_kind),
		(Slice(left_target, left), Slice(right_target, right)) |
		(Pointer(left_target, left), Pointer(right_target, right)) =>
			left_target == right_target && unifies(left, right),
		(Function(left), Function(right)) => {
			let mut equal = left.convention.as_ref().map(|node| &node.node)
				== right.convention.as_ref().map(|node| &node.node);
			equal &= unifies(&left.return_type, &right.return_type);
			equal &= left.parameters.len() == right.parameters.len();
			Iterator::zip(left.parameters.iter(), &right.parameters)
				.all(|(left, right)| unifies(left, right)) && equal
		}
		(_, _) => false,
	}
}

fn unify(target: &S<RType>, kind: &IType) -> bool {
	match (&target.node, kind) {
		(RType::Array(kind, _), IType::Sequence(_, other)) => unifies(kind, other),
		(RType::Slice(target, kind), IType::Sequence(targets, other)) =>
			target == targets && unifies(kind, other),
		(RType::IntegralSize(_), IType::IntegralSize) => true,
		(_, IType::Type(kind)) => unifies(target, kind),
		(_, _) => false,
	}
}
