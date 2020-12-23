use crate::generate::Target;
use crate::node::*;
use crate::query::{E, S};

use super::{IType, raise, Scene, TRUTH};

pub fn synthesize(scene: &mut Scene, index: &HIndex) -> Option<S<RType>> {
	let kind = synthesized(scene, index)?;
	scene.types.nodes.insert(*index, kind.clone()).unwrap_none();
	Some(kind)
}

fn synthesized(scene: &mut Scene, index: &HIndex) -> Option<S<RType>> {
	let span = scene.value[*index].span;
	let node = &scene.value[*index].node;
	Some(S::new(match node {
		HNode::Block(nodes) => {
			let synthesize = |node| synthesize(scene, node);
			match nodes.iter().map(synthesize).last() {
				Some(kind) => return kind,
				None => RType::Void,
			}
		}
		HNode::Let(variable, kind, node) => {
			let kind = match (kind, node) {
				(None, None) => scene.unknown(span)?,
				(Some(kind), None) => scene.lift(kind)?,
				(None, Some(node)) => synthesize(scene, &node)?,
				(Some(kind), Some(node)) => {
					let kind = scene.lift(kind)?;
					let raised = raise(kind.clone());
					super::check(scene, node, raised);
					kind
				}
			};

			let variable = variable.node.clone();
			scene.types.variables.insert(variable, kind);
			RType::Void
		}
		HNode::Set(target, node) => {
			let kind = synthesize(scene, target)?;
			super::check(scene, node, raise(kind));
			RType::Void
		}
		HNode::While(condition, node) => {
			super::check(scene, condition, TRUTH);
			synthesize(scene, node);
			RType::Void
		}
		HNode::When(branches) => {
			let mut first: Option<S<RType>> = None;
			let (mut equal, mut complete) = (true, false);
			for (condition, node) in branches {
				super::check(scene, condition, TRUTH);
				let other = &scene.value[*condition].node;
				complete |= matches!(other, HNode::Truth(true));
				synthesize(scene, node).map(|kind| match first.as_ref() {
					Some(first) => equal &= super::unifies(&first, &kind),
					None => first = Some(kind),
				});
			}

			match equal && complete {
				false => RType::Void,
				true => first?.node,
			}
		}
		HNode::Cast(node, kind) => {
			let kind = kind.as_ref()
				.or_else(|| scene.unknown(span))
				.and_then(|kind| scene.lift(kind))?;
			let value = &scene.value[*node].node;
			if let HNode::Integral(_) = value {
				let kind = raise(kind.clone());
				super::check(scene, node, kind);
			} else { synthesize(scene, node); }
			kind.node
		}
		HNode::Return(node) => {
			if let Some(kind) = &scene.return_type {
				let void = S::new(RType::Void, span);
				if let Some(node) = node {
					let kind = raise(kind.clone());
					super::check(scene, &node, kind);
				} else if !super::unifies(kind, &void) {
					E::error().message("missing return value")
						.note(format!("function returns: {}", &kind.node))
						.label(span.label()).emit(scene.scope);
				}
			}
			RType::Never
		}
		HNode::Compile(index) => {
			let path = VPath(scene.scope.symbol.clone(), *index);
			let scope = &mut scene.scope.span(span);
			let value = crate::parse::value(scope, &path).ok()?;
			let types = super::types(scope, &path).ok()?;
			return types.nodes.get(&value.root).cloned();
		}
		HNode::Inline(_) => RType::Void,
		HNode::Call(path, arguments) => {
			let path = path.path();
			let scope = &mut scene.scope.span(span);
			let candidates: Vec<_> = super::signatures(scope, &path)
				.ok()?.into_iter().enumerate().filter(|(_, signature)|
				signature.parameters.len() == arguments.len()).collect();

			if candidates.len() == 1 {
				let candidate = candidates.into_iter().next();
				let (target, signature) = candidate.unwrap();
				Iterator::zip(signature.parameters.into_iter()
					.map(raise), arguments).for_each(|(parameter, node)|
					super::check(scene, node, parameter));
				scene.types.functions.insert(*index, target);
				return Some(signature.return_type);
			}

			let arguments: Vec<_> = arguments.iter().map(|node|
				synthesize(scene, node)).collect::<Option<_>>()?;
			let candidates = candidates.into_iter().filter(|(_, candidate)|
				Iterator::zip(candidate.parameters.iter(), &arguments)
					.all(|(left, right)| super::unifies(left, right)))
				.collect::<Vec<_>>();

			if candidates.len() == 1 {
				let candidate = candidates.into_iter().next();
				let (target, signature) = candidate.unwrap();
				scene.types.functions.insert(*index, target);
				signature.return_type.node
			} else if candidates.is_empty() {
				let error = E::error().message("no matching function call");
				error.label(span.label()).result(scene.scope).ok()?
			} else {
				let error = E::error().message("ambiguous function call");
				let error = candidates.iter().fold(error, |error, (index, _)|
					error.note(FPath(path.clone(), *index).to_string()));
				error.label(span.label()).result(scene.scope).ok()?
			}
		}
		HNode::Method(node, arguments) => {
			let span = scene.value[*node].span;
			let kind = synthesize(scene, node)?;
			if let RType::Function(signature) = kind.node {
				Iterator::zip(signature.parameters.into_iter()
					.map(raise), arguments).for_each(|(parameter, node)|
					super::check(scene, node, parameter));
				signature.return_type.node
			} else {
				let message = format!("expected function, found: {}", kind.node);
				let error = E::error().message(message).label(span.label());
				return error.result(scene.scope).ok();
			}
		}
		HNode::Field(node, name) => {
			let span = scene.value[*node].span;
			let kind = synthesize(scene, node)?;
			let undefined = |scene: &mut Scene, node| E::error()
				.label(span.other().message(node)).label(name.span.label())
				.message(format!("undefined field: {}", name.node))
				.result(scene.scope).ok();

			if let RType::Structure(path) = &kind.node {
				let scope = &mut scene.scope.span(kind.span);
				let structure = crate::parse::structure(scope, path).ok()?;
				match structure.fields.get(&name.node) {
					Some((_, kind)) => scene.lift(kind)?.node,
					None => undefined(scene, &kind.node)?,
				}
			} else if let RType::Slice(target, other) = kind.node {
				match name.node.as_ref() {
					"address" => RType::Pointer(target, other),
					"size" => RType::IntegralSize(target, Sign::Unsigned),
					_ => undefined(scene, &RType::Slice(target, other))?,
				}
			} else {
				let message = format!("expected structure, found: {}", kind.node);
				let error = E::error().message(message).label(span.label());
				return error.result(scene.scope).ok();
			}
		}
		HNode::New(path, fields) => {
			let path = path.path();
			let scope = &mut scene.scope.span(span);
			let structure = crate::parse::structure(scope, &path);
			for (name, (span, kind)) in &structure.ok()?.fields {
				if let Some((_, field)) = fields.get(name) {
					let kind = scene.lift(kind);
					super::checked(scene, field, kind);
				} else {
					let error = E::error().label(span.label());
					let message = format!("undefined field: {}", name);
					error.message(message).emit(scene.scope);
				}
			}
			RType::Structure(path)
		}
		HNode::SliceNew(target, kind, fields) => {
			let index = target.map(|target| {
				let kind = Some(target.node);
				let kind = RType::IntegralSize(kind, Sign::Unsigned);
				raise(S::new(kind, target.span))
			}).unwrap_or_else(|| super::index(scene.target));

			let name = &Identifier("size".into());
			fields.get(name).map(|(_, field)|
				super::check(scene, field, index));

			let kind = scene.lift(kind)?;
			let name = &Identifier("address".into());
			if let Some((span, field)) = fields.get(name) {
				let kind = RType::Pointer(scene.target, Box::new(kind.clone()));
				super::check(scene, field, raise(S::new(kind, *span)));
			}

			let target = target.map(|target| target.node);
			RType::Slice(target, Box::new(kind))
		}
		HNode::Slice(node, left, right) => {
			let (kind, target) = sequence(scene, node)?;
			left.map(|node| super::check(scene, &node, super::index(target)));
			right.map(|node| super::check(scene, &node, super::index(target)));
			RType::Slice(target, kind)
		}
		HNode::Index(node, index) => {
			let (kind, target) = sequence(scene, node)?;
			super::check(scene, index, super::index(target));
			return Some(*kind);
		}
		HNode::Compound(dual, target, node) => {
			let target = synthesize(scene, target);
			if let HDual::Add | HDual::Minus = dual {
				if let Some(S { node: RType::Pointer(target, _), .. }) = target {
					super::check(scene, node, IType::IntegralSize(target));
					return Some(S::new(RType::Void, span));
				}
			}

			super::checked(scene, node, target);
			RType::Void
		}
		HNode::Binary(binary, left, right) => {
			if let HBinary::And | HBinary::Or = binary {
				super::check(scene, left, TRUTH);
				super::check(scene, right, TRUTH);
				return Some(S::new(RType::Truth, span));
			}

			let left = synthesize(scene, left);
			if let HBinary::Equal | HBinary::NotEqual = binary {
				super::checked(scene, right, left);
				return Some(S::new(RType::Truth, span));
			}

			if let HBinary::Dual(HDual::Add | HDual::Minus) = binary {
				if let Some(S { node: RType::Pointer(target, _), .. }) = left {
					super::check(scene, right, IType::IntegralSize(target));
					return left;
				}
			}

			let mut compare = false;
			compare |= matches!(binary, HBinary::Equal | HBinary::NotEqual);
			compare |= matches!(binary, HBinary::Less | HBinary::LessEqual);
			compare |= matches!(binary, HBinary::Greater | HBinary::GreaterEqual);

			// Operation validity is checked at node lowering.
			super::checked(scene, right, left.clone());
			match compare {
				true => RType::Truth,
				false => left?.node,
			}
		}
		HNode::Unary(unary, node) => {
			let kind = synthesize(scene, node)?;
			match unary {
				HUnary::Not | HUnary::Negate => kind.node,
				HUnary::Reference => {
					let kind = Box::new(kind);
					RType::Pointer(scene.target, kind)
				}
				HUnary::Dereference => match kind.node {
					RType::Pointer(_, kind) => kind.node,
					other => E::error().message("expected pointer")
						.label(scene.value[*node].span.label().message(other))
						.result(scene.scope).ok()?,
				},
			}
		}
		HNode::Variable(variable) => return scene
			.types.variables.get(variable).cloned(),
		HNode::Function(path) => {
			let path = path.path();
			let scope = &mut scene.scope.span(span);
			let candidates = super::signatures(scope, &path).ok()?;

			if candidates.len() == 1 {
				scene.types.functions.insert(*index, 0);
				let candidate = candidates.into_iter().next();
				RType::Function(Box::new(candidate.unwrap()))
			} else {
				let candidates = candidates.iter().enumerate();
				let error = E::error().message("ambiguous function");
				let error = candidates.fold(error, |error, (index, _)|
					error.note(FPath(path.clone(), index).to_string()));
				error.label(span.label()).result(scene.scope).ok()?
			}
		}
		HNode::Static(path) => {
			let scope = &mut scene.scope.span(span);
			super::statics(scope, &path.path()).ok()?
		}
		HNode::String(string) => {
			let kind = RType::Integral(Sign::Unsigned, Width::B);
			RType::Array(Box::new(S::new(kind, span)), string.len())
		}
		HNode::Register(_) => scene.unknown(span)?,
		HNode::Array(nodes) => match nodes.split_first() {
			None => scene.unknown(span)?,
			Some((first, nodes)) => {
				let kind = synthesize(scene, first)?;
				let check = raise(kind.clone());
				nodes.iter().for_each(|node|
					super::check(scene, node, check.clone()));
				RType::Array(Box::new(kind), nodes.len())
			}
		}
		HNode::Integral(_) => E::error()
			.message("unknown integral type").label(span.label())
			.note("explicitly cast the integer: <literal> as <type>")
			.result(scene.scope).ok()?,
		HNode::Truth(_) => RType::Truth,
		HNode::Rune(_) => RType::Rune,
		HNode::Break => RType::Never,
		HNode::Continue => RType::Never,
		HNode::Unresolved(_) => return None,
		HNode::Error(_) => return None,
	}, span))
}

fn sequence(scene: &mut Scene, base: &HIndex)
			-> Option<(Box<S<RType>>, Option<Target>)> {
	Some(match synthesize(scene, base)?.node {
		Type::Slice(target, kind) => (kind, target),
		Type::Array(kind, _) => (kind, scene.target),
		other => E::error().label(scene.value[*base].span.label())
			.message(format!("expected sequence, found: {}", other))
			.result(scene.scope).ok()?,
	})
}
