use crate::node::*;
use crate::query::{E, QueryError, S};

use super::*;

/// Lowers a node into a statement.
pub fn unit(scene: &mut Scene, index: &HIndex,
			looped: bool) -> crate::Result<S<LUnit>> {
	let span = scene.node[*index].span;
	let kind = bail!(scene.types.nodes.get(index))?;
	let lower = |scene: &mut _, index| lower(scene, index, looped);
	let unit = |scene: &mut _, index| unit(scene, index, looped);
	if !super::size(scene.scope, kind)?.node.zero() {
		let node = LUnit::Node(lower(scene, index)?);
		return Ok(S::new(node, span));
	}

	Ok(S::new(match &scene.node[*index].node {
		HNode::Block(nodes) => LUnit::Block(nodes
			.iter().map(|node| unit(scene, node))
			.collect::<Result<_, _>>()?),
		HNode::Let(variable, _, node) => {
			let kind = bail!(scene.types.variables.get(&variable.node))?;
			let size = super::size(scene.scope, kind)?;
			let target = variable.node.clone();
			let target = scene.variable(target, size);
			if !size.node.zero() {
				if let Some(index) = node {
					let target = S::new(LNode::Target(target), variable.span);
					LUnit::Set(LPlace(Box::new(target)), lower(scene, index)?)
				} else { LUnit::Zero(target) }
			} else {
				match node {
					Some(node) => LUnit::Node(lower(scene, node)?),
					None => LUnit::Block(Box::new([])),
				}
			}
		}
		HNode::Set(target, node) => {
			let place = place(scene, target, looped, false)?;
			LUnit::Set(place, lower(scene, node)?)
		}
		HNode::While(condition, node) => {
			let condition = lower(scene, condition)?;
			let exit = Box::new(S::new(LUnit::Break, span));
			let node = Box::new(self::unit(scene, node, true)?);
			let unit = LUnit::If(condition, node, Some(exit));
			LUnit::Loop(Box::new(S::new(unit, span)))
		}
		HNode::When(branches) => {
			let mut node: Option<S<LUnit>> = None;
			for (condition, index) in branches.iter().rev() {
				let truth = lower(scene, condition)?;
				let unit = Box::new(unit(scene, index)?);
				let other = node.map(Box::new);

				let unit = LUnit::If(truth, unit, other);
				let span = scene.node[*condition].span;
				node = Some(S::new(unit, span));
			}

			let node = node.unwrap();
			node.node
		}
		HNode::Cast(node, _) => {
			let origin = bail!(scene.types.nodes.get(node))?;
			match (&origin.node, &kind.node) {
				(RType::Void, RType::Void) => unit(scene, node)?.node,
				(RType::Never, RType::Never) => unit(scene, node)?.node,
				(_, kind) => invalid_cast(scene, &origin.node,
					kind, scene.node[*node].span, span)?,
			}
		}
		HNode::Return(node) => LUnit::Return(node.as_ref()
			.map(|node| lower(scene, node)).transpose()?),
		HNode::Compile(index) => LUnit::Compile(*index),
		HNode::Inline(index) => LUnit::Inline(*index),
		HNode::Call(path, arguments) => {
			let index = bail!(scene.types.functions.get(index))?;
			let receiver = functional(scene, path, *index, span)?;
			let arguments = arguments.iter().map(|node|
				lower(scene, node)).collect::<Result<_, _>>()?;
			LUnit::Call(receiver, arguments)
		}
		HNode::Method(base, arguments) => {
			let arguments = arguments.iter().map(|node|
				lower(scene, node)).collect::<Result<_, _>>()?;
			LUnit::Call(method(scene, base, looped)?, arguments)
		}
		HNode::Field(base, _) => unit(scene, base)?.node,
		HNode::New(path, fields) => {
			let scope = &mut scene.scope.span(span);
			let offsets = super::offsets(scope, &path.path())?;
			LUnit::Block(offsets.fields.keys().map(|name|
				fields.get(name).ok_or_else(|| E::error()
					.message(format!("missing field: {}", name))
					.label(span.label()).to(scene.scope))
					.and_then(|(_, field)| unit(scene, field)))
				.collect::<Result<_, _>>()?)
		}
		HNode::Index(node, index) => {
			if let RType::Slice(target, _) = &kind.node {
				self::target(scene, target, span, "index slice")?;
			}

			let block = [unit(scene, node)?, unit(scene, index)?];
			LUnit::Block(Box::new(block))
		}
		HNode::Compound(dual, target, index) => {
			let dual = &HBinary::Dual(*dual);
			let place = place(scene, target, looped, false)?;
			let node = binary(scene, dual, target, index, looped, span)?;
			LUnit::Set(place, node)
		}
		HNode::Binary(_, left, _) => {
			let left = bail!(scene.types.nodes.get(left))?;
			return invalid_binary(scene, &left.node, span);
		}
		HNode::Unary(HUnary::Dereference, node) => {
			let kind = bail!(scene.types.nodes.get(node))?;
			if let RType::Pointer(target, _) = &kind.node {
				self::target(scene, target, span, "dereference pointer")?;
				LUnit::Node(lower(scene, node)?)
			} else { panic!("type: {}, not pointer", kind.node); }
		}
		HNode::Unary(_, _) => invalid_unary(scene, &kind.node, span)?,
		HNode::Break if looped => LUnit::Break,
		HNode::Continue if looped => LUnit::Continue,
		HNode::Break | HNode::Continue => E::error()
			.message("break or continue not inside loop")
			.label(span.label()).result(scene.scope)?,
		HNode::Static(_) => LUnit::Block(Box::new([])),
		HNode::Variable(_) => LUnit::Block(Box::new([])),
		HNode::Unresolved(_) | HNode::Error(_) => bail!(None)?,
		other => panic!("lowering: {:?}, as statement", other),
	}, span))
}
