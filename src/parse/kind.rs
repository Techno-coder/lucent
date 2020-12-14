use crate::generate::Target;
use crate::node::{HPath, HType, Sign, Width};
use crate::query::{E, MScope, S};

use super::{Node, Scene, TSpan};

/// Parses a type.
pub fn kind<'a>(scope: MScope, scene: &mut Scene, span: &TSpan,
				node: impl Node<'a>) -> crate::Result<S<HType>> {
	Ok(S::new(match node.kind() {
		"signature_type" => {
			let signature = super::signature(scope, scene, span, &node)?;
			HType::Function(Box::new(signature))
		}
		"array_type" => {
			let kind = node.field(scope, "type")?;
			let size = node.field(scope, "size")?;
			let kind = self::kind(scope, scene, span, kind)?;
			let size = super::valued(scope, scene, span, size);
			HType::Array(Box::new(kind), size)
		}
		"slice_type" => {
			let kind = node.field(scope, "type")?;
			let kind = self::kind(scope, scene, span, kind)?;
			let target = target(scope, span, &node)?;
			HType::Slice(target, Box::new(kind))
		}
		"pointer" => {
			let kind = node.field(scope, "type")?;
			let kind = self::kind(scope, scene, span, kind)?;
			let target = target(scope, span, &node)?;
			HType::Pointer(target, Box::new(kind))
		}
		"size_type" => {
			let target = target(scope, span, &node)?;
			let kind = node.field(scope, "kind")?;
			match kind.text() {
				"isize" => HType::IntegralSize(target, Sign::Signed),
				"usize" => HType::IntegralSize(target, Sign::Unsigned),
				other => return E::error()
					.message(format!("invalid size kind: {}", other))
					.label(kind.span().label()).result(scope),
			}
		}
		"path" => path_kind(scope, scene, span, &node)?,
		_ => node.invalid(scope)?,
	}, TSpan::offset(span, node.span())))
}

pub fn target<'a>(scope: MScope, base: &TSpan, node: &impl Node<'a>)
				  -> crate::Result<Option<S<Target>>> {
	node.attribute("target").map(|node| {
		let target = Target::parse(node.string());
		let target = target.ok_or_else(|| E::error()
			.message(format!("unknown architecture: {}", node.text()))
			.label(node.span().label()).to(scope))?;
		Ok(S::new(target, node.offset(base)))
	}).transpose()
}

fn path_kind<'a>(scope: MScope, scene: &Scene, base: &TSpan,
				 node: &impl Node<'a>) -> crate::Result<HType> {
	let path = super::path(scope, base, node)?;
	if let HPath::Node(module, name) = &path {
		if module.as_ref() == &HPath::root() {
			match name.node.as_ref() {
				"void" => return Ok(HType::Void),
				"rune" => return Ok(HType::Rune),
				"truth" => return Ok(HType::Truth),
				"never" => return Ok(HType::Never),
				"i8" => return Ok(HType::Integral(Sign::Signed, Width::B)),
				"i16" => return Ok(HType::Integral(Sign::Signed, Width::W)),
				"i32" => return Ok(HType::Integral(Sign::Signed, Width::D)),
				"i64" => return Ok(HType::Integral(Sign::Signed, Width::Q)),
				"u8" => return Ok(HType::Integral(Sign::Unsigned, Width::B)),
				"u16" => return Ok(HType::Integral(Sign::Unsigned, Width::W)),
				"u32" => return Ok(HType::Integral(Sign::Unsigned, Width::D)),
				"u64" => return Ok(HType::Integral(Sign::Unsigned, Width::Q)),
				_ => (),
			}
		}
	}

	let scope = &mut scope.span(node.span());
	let path = scene.inclusions.structure(scope, base, &path)?
		.ok_or_else(|| E::error().message("undefined type")
			.label(scope.span.label()).to(scope))?;
	Ok(HType::Structure(path))
}
