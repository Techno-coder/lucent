use crate::node::*;
use crate::query::{QueryError, S};

use super::*;

/// Lowers a node into an expression.
/// The size of its type must not be
/// zero unless the type is `Never`.
pub fn lower(scene: &mut Scene, index: &HIndex,
			 looped: bool) -> crate::Result<S<LNode>> {
	let kind = bail!(scene.types.nodes.get(index))?;
	let size = super::size(scene.scope, kind)?;
	if let RType::Never = kind.node {
		let span = scene.node[*index].span;
		let block = Box::new(unit(scene, index, looped)?);
		return Ok(S::new(LNode::Never(block), span));
	} else { assert!(!size.node.zero()); }

	let span = scene.node[*index].span;
	let lower = |scene: &mut _, index| lower(scene, index, looped);
	Ok(S::new(match &scene.node[*index].node {
		HNode::Block(box [rest @ .., last]) => {
			let block = rest.iter().map(|node| unit(scene,
				node, looped)).collect::<Result<_, _>>()?;
			let node = Box::new(lower(scene, last)?);
			LNode::Block(block, node)
		}
		HNode::When(branches) => {
			let mut node: Option<S<LNode>> = None;
			for (condition, index) in branches.iter().rev() {
				let truth = Box::new(lower(scene, condition)?);
				let unit = Box::new(lower(scene, index)?);
				let other = node.map(Box::new);

				let unit = LNode::If(truth, unit, other);
				let span = scene.node[*condition].span;
				node = Some(S::new(unit, span));
			}

			let node = node.unwrap();
			node.node
		}
		HNode::Cast(node, _) => {
			use {Type::*, Sign::Unsigned};
			let spans = scene.node[*node].span;
			let origin = bail!(scene.types.nodes.get(node))?;
			let (sign, origin, width) = match (&origin.node, &kind.node) {
				(Pointer(origin, _), Pointer(target, _)) |
				(Pointer(origin, _), IntegralSize(target, Unsigned)) |
				(IntegralSize(origin, Unsigned), Pointer(target, _))
				if origin == target => {
					let origin = pointer(scene, origin, spans)?;
					let target = pointer(scene, target, span)?;
					(Unsigned, origin, target)
				}
				(Pointer(origin, _), Function(function)) |
				(IntegralSize(origin, Unsigned), Function(function))
				if origin == &function.target => {
					let origin = pointer(scene, origin, spans)?;
					let target = pointer(scene, &function.target, span)?;
					(Unsigned, origin, target)
				}
				(Function(function), Pointer(target, _)) |
				(Function(function), IntegralSize(target, Unsigned))
				if &function.target == target => {
					let origin = pointer(scene, &function.target, spans)?;
					let target = pointer(scene, target, span)?;
					(Unsigned, origin, target)
				}
				(Function(origin), Function(target))
				if origin.target == target.target => {
					let origin = pointer(scene, &origin.target, spans)?;
					let target = pointer(scene, &target.target, span)?;
					(Unsigned, origin, target)
				}
				(IntegralSize(origin, sign), IntegralSize(target, _)) => {
					let origin = pointer(scene, origin, spans)?;
					let target = pointer(scene, target, span)?;
					(*sign, origin, target)
				}
				(Integral(sign, width), IntegralSize(target, _)) =>
					(*sign, *width, pointer(scene, target, spans)?),
				(IntegralSize(origin, sign), Integral(_, width)) =>
					(*sign, pointer(scene, origin, spans)?, *width),
				(Integral(sign, origin), Integral(_, width)) =>
					(*sign, *origin, *width),
				(Rune, Integral(Unsigned, width @ Width::B)) |
				(Rune, Integral(Unsigned, width @ Width::D)) =>
					(Unsigned, Width::D, *width),
				(Integral(Unsigned, width @ Width::B), Rune) |
				(Integral(Unsigned, width @ Width::D), Rune) =>
					(Unsigned, *width, Width::D),
				(_, kind) => invalid_cast(scene,
					&origin.node, kind, spans, span)?,
			};

			let node = Box::new(lower(scene, node)?);
			LNode::Cast(node, (sign, origin), width)
		}
		HNode::Compile(index) => LNode::Compile(*index),
		HNode::Call(path, arguments) => {
			let index = bail!(scene.types.functions.get(index))?;
			let receiver = functional(scene, path, *index, span)?;
			let arguments = arguments.iter().map(|node|
				lower(scene, node)).collect::<Result<_, _>>()?;
			LNode::Call(receiver, arguments)
		}
		HNode::Method(base, arguments) => {
			let arguments = arguments.iter().map(|node|
				lower(scene, node)).collect::<Result<_, _>>()?;
			LNode::Call(method(scene, base, looped)?, arguments)
		}
		HNode::Field(base, name) => {
			let place = place(scene, base, looped, false)?;
			let kind = bail!(scene.types.nodes.get(base))?;
			if let RType::Structure(path) = &kind.node {
				let scope = &mut scene.scope.span(span);
				let offsets = super::offsets(scope, path)?;
				LNode::Dereference(self::offset(scene,
					place, offsets.fields[&name.node], span))
			} else if let RType::Slice(target, _) = &kind.node {
				LNode::Dereference(match name.node.as_ref() {
					"address" => place,
					"size" => {
						let width = pointer(scene, target, span)?;
						offset(scene, place, width.bytes(), span)
					}
					other => panic!("invalid slice field: {}", other),
				})
			} else { panic!("type: {}, not structure", kind.node); }
		}
		HNode::New(path, fields) => {
			let scope = &mut scene.scope.span(span);
			let offsets = super::offsets(scope, &path.path())?;
			let target = LNode::Target(scene.local(size));
			let place = LPlace(Box::new(S::new(target, span)));

			let mut block = Vec::new();
			for (name, offset) in &offsets.fields {
				let (span, field) = field(scene, fields, name, span)?;
				let kind = bail!(scene.types.nodes.get(field))?;
				let size = super::size(scene.scope, kind)?;

				block.push(if !size.node.zero() {
					let place = self::offset(scene, place.clone(), *offset, *span);
					S::new(LUnit::Set(place, lower(scene, field)?), *span)
				} else { unit(scene, field, looped)? });
			}

			let node = S::new(LNode::Dereference(place), span);
			LNode::Block(block.into(), Box::new(node))
		}
		HNode::SliceNew(_, _, fields) => {
			let target = LNode::Target(scene.local(size));
			let place = LPlace(Box::new(S::new(target, span)));
			let mut block = Vec::new();

			let name = &Identifier("address".into());
			let (other, index) = field(scene, fields, name, span)?;
			let unit = LUnit::Set(place.clone(), lower(scene, index)?);
			block.push(S::new(unit, *other));

			if let RType::Slice(target, _) = &kind.node {
				let name = &Identifier("size".into());
				let (other, index) = field(scene, fields, name, span)?;
				let offset = pointer(scene, target, span)?.bytes();
				let place = self::offset(scene, place.clone(), offset, *other);
				let unit = LUnit::Set(place, lower(scene, index)?);
				block.push(S::new(unit, *other));
			} else { panic!("type: {}, not slice", kind.node); }

			let node = S::new(LNode::Dereference(place), span);
			LNode::Block(block.into(), Box::new(node))
		}
		HNode::Slice(base, start, end) => {
			let mut block = Vec::new();
			let width = scene.target.pointer;
			let LPlace(place) = place(scene, base, looped, true)?;
			let base = scene.local(S::new(Size(width.bytes()), span));
			let base = LPlace(Box::new(S::new(LNode::Target(base), span)));
			block.push(S::new(LUnit::Set(base.clone(), *place), span));

			let end = end.as_ref().map(|end| lower(scene, end));
			let (address, end) = if let RType::Slice(target, _) = &kind.node {
				self::target(scene, target, span, "reference slice")?;
				let address = LNode::Dereference(base.clone());
				let address = LPlace(Box::new(S::new(address, span)));
				(address, end.transpose()?.unwrap_or_else(|| {
					let place = self::offset(scene, base, width.bytes(), span);
					S::new(LNode::Dereference(place), span)
				}))
			} else if let RType::Array(_, size) = &kind.node {
				(base, end.transpose()?.unwrap_or_else(||
					S::new(LNode::Integral(*size as i128), span)))
			} else { panic!("type: {}, not sequenced", kind.node) };

			let (address, sized) = if let Some(start) = start {
				let local = scene.local(S::new(Size(width.bytes()), span));
				let local = LPlace(Box::new(S::new(LNode::Target(local), span)));
				let unit = LUnit::Set(local.clone(), lower(scene, start)?);
				block.push(S::new(unit, span));

				let end = Box::new(end);
				let start = LNode::Dereference(local);
				let start = Box::new(S::new(start.clone(), span));
				let kind = match &kind.node {
					RType::Slice(_, kind) => kind,
					RType::Array(kind, _) => kind,
					_ => unreachable!(),
				};

				let Size(step) = super::size(scene.scope, kind)?.node;
				let step = Box::new(S::new(LNode::Integral(step as i128), span));
				let offset = LNode::Binary(LBinary::Multiply, width, step, start.clone());
				let address = offset_node(scene, address, offset, span);

				let size = LNode::Binary(LBinary::Minus, width, end, start);
				(address, S::new(size, span))
			} else { (address, end) };

			let LPlace(address) = address;
			let slice = LNode::Target(scene.local(size));
			let slice = LPlace(Box::new(S::new(slice, span)));
			block.push(S::new(LUnit::Set(slice.clone(), *address), span));
			block.push(S::new(LUnit::Set(slice.clone(), sized), span));
			let slice = S::new(LNode::Dereference(slice), span);
			LNode::Block(block.into(), Box::new(slice))
		}
		HNode::Index(base, index) => {
			let width = scene.target.pointer;
			let index = Box::new(lower(scene, index)?);
			let place = place(scene, base, looped, true)?;
			let kind = bail!(scene.types.nodes.get(base))?;
			let (place, kind) = match &kind.node {
				RType::Array(kind, _) => (place, kind),
				RType::Slice(target, kind) => {
					self::target(scene, target, span, "index slice")?;
					let place = S::new(LNode::Dereference(place), span);
					(LPlace(Box::new(place)), kind)
				}
				other => panic!("type: {}, not sequenced", other),
			};

			let Size(size) = super::size(scene.scope, kind)?.node;
			let size = Box::new(S::new(LNode::Integral(size as i128), span));
			let offset = LNode::Binary(LBinary::Multiply, width, size, index);
			LNode::Dereference(offset_node(scene, place, offset, span))
		}
		HNode::Binary(operator, left, right) => return
			binary(scene, operator, left, right, looped, span),
		HNode::Unary(HUnary::Not, node) => {
			let node = Box::new(lower(scene, node)?);
			LNode::Unary(LUnary::Not, match &kind.node {
				RType::Truth => Width::B,
				RType::Integral(_, width) => *width,
				RType::IntegralSize(target, _) =>
					pointer(scene, target, span)?,
				other => invalid_unary(scene, other, span)?,
			}, node)
		}
		HNode::Unary(HUnary::Negate, node) => {
			let node = Box::new(lower(scene, node)?);
			LNode::Unary(LUnary::Negate, match &kind.node {
				RType::Integral(Sign::Signed, width) => *width,
				RType::IntegralSize(target, Sign::Signed) =>
					pointer(scene, target, span)?,
				other => invalid_unary(scene, other, span)?,
			}, node)
		}
		HNode::Unary(HUnary::Reference, node) => place(scene,
			node, looped, true).map(|LPlace(node)| node.node)?,
		HNode::Unary(HUnary::Dereference, node) => {
			let kind = bail!(scene.types.nodes.get(node))?;
			if let RType::Pointer(target, _) = &kind.node {
				self::target(scene, target, span, "dereference pointer")?;
				LNode::Dereference(LPlace(Box::new(lower(scene, node)?)))
			} else { panic!("type: {}, not pointer", kind.node); }
		}
		HNode::Variable(variable) => {
			let target = &scene.targets[variable];
			let target = S::new(LNode::Target(target.clone()), span);
			LNode::Dereference(LPlace(Box::new(target)))
		}
		HNode::Function(path) => {
			let index = bail!(scene.types.functions.get(index))?;
			LNode::Function(FPath(path.path(), *index))
		}
		HNode::Static(path) => {
			let target = S::new(LNode::Static(path.path()), span);
			LNode::Dereference(LPlace(Box::new(target)))
		}
		HNode::Array(nodes) => {
			let local = scene.local(size);
			let target = S::new(LNode::Target(local), span);
			let place = LPlace(Box::new(target));
			if let RType::Array(kind, _) = &kind.node {
				let Size(size) = super::size(scene.scope, kind)?.node;
				let block = nodes.iter().enumerate().map(|(index, node)| {
					let span = scene.node[*node].span;
					let offset = offset(scene, place.clone(), size * index, span);
					Ok(S::new(LUnit::Set(offset, lower(scene, node)?), span))
				}).collect::<Result<Box<_>, _>>()?;

				let node = S::new(LNode::Dereference(place), span);
				LNode::Block(block.into(), Box::new(node))
			} else { panic!("type: {}, not array", kind.node); }
		}
		HNode::String(string) => LNode::String(string.clone()),
		HNode::Register(value) => LNode::Register(value.clone()),
		HNode::Integral(value) => LNode::Integral(*value),
		HNode::Truth(value) => LNode::Integral(*value as i128),
		HNode::Rune(value) => LNode::Integral(*value as i128),
		HNode::Unresolved(_) | HNode::Error(_) => bail!(None)?,
		other => panic!("lowering: {:?}, as expression", other),
	}, span))
}
