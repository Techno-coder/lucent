use crate::node::*;
use crate::query::{E, S};

use super::{INDEX, IType, raise, Scene, TRUTH};

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
			return nodes.iter().map(synthesize).last().unwrap();
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
			let mut first = None;
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
			let kind = synthesize(scene, node)?;
			if let RType::Function(signature) = kind.node {
				Iterator::zip(signature.parameters.into_iter()
					.map(raise), arguments).for_each(|(parameter, node)|
					super::check(scene, node, parameter));
				signature.return_type.node
			} else {
				E::error().message("value is not a function")
					.label(kind.span.label()).emit(scene.scope);
				return None;
			}
		}
		HNode::Field(node, name) => {
			let kind = synthesize(scene, node)?;
			if let RType::Structure(path) = kind.node {
				let scope = &mut scene.scope.span(kind.span);
				let structure = crate::parse::structure(scope, &path).ok()?;
				let (_, kind) = structure.fields.get(&name.node)?;
				scene.lift(kind)?.node
			} else if let RType::Slice(kind) = kind.node {
				match name.node.as_ref() {
					"address" => RType::Pointer(kind),
					"size" => RType::IntegralSize(Sign::Unsigned),
					_ => return None,
				}
			} else {
				return None;
			}
		}
		HNode::New(path, fields) => {
			let path = path.path();
			let scope = &mut scene.scope.span(span);
			let structure = crate::parse::structure(scope, &path);
			for (name, (_, kind)) in &structure.ok()?.fields {
				if let Some((_, field)) = fields.get(name) {
					let kind = scene.lift(kind);
					super::checked(scene, field, kind);
				}
			}
			RType::Structure(path)
		}
		HNode::SliceNew(kind, fields) => {
			let name = &Identifier("size".into());
			fields.get(name).map(|(_, field)|
				super::check(scene, field, INDEX));

			let kind = scene.lift(kind)?;
			let name = &Identifier("address".into());
			if let Some((span, field)) = fields.get(name) {
				let kind = RType::Pointer(Box::new(kind.clone()));
				super::check(scene, field, raise(S::new(kind, *span)));
			}
			kind.node
		}
		HNode::Slice(node, left, right) => {
			left.map(|node| super::check(scene, &node, INDEX));
			right.map(|node| super::check(scene, &node, INDEX));
			synthesize(scene, node)?.node
		}
		HNode::Index(node, index) => {
			super::check(scene, index, INDEX);
			synthesize(scene, node)?.node
		}
		HNode::Compound(dual, target, node) => {
			let target = synthesize(scene, target);
			if let HDual::Add | HDual::Minus = dual {
				if let Some(S { node: RType::Pointer(_), .. }) = target {
					super::check(scene, node, IType::IntegralSize);
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
				if let Some(S { node: RType::Pointer(_), .. }) = left {
					super::check(scene, right, IType::IntegralSize);
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
				Unary::Not | Unary::Negate => kind.node,
				Unary::Reference => RType::Pointer(Box::new(kind)),
				Unary::Dereference => match kind.node {
					RType::Pointer(kind) => kind.node,
					_ => E::error().message("value is not a pointer")
						.label(scene.value[*node].span.label())
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
