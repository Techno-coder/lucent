use crate::node::{HType, Identifier, Path, Sign, Width};
use crate::query::{E, MScope, S};

use super::{Inclusions, Node, TSpan};

/// Parses a type.
pub fn kind<'a>(scope: MScope, inclusions: &Inclusions, span: &TSpan,
				node: impl Node<'a>) -> crate::Result<S<HType>> {
	Ok(S::new(match node.kind() {
		"signature_type" => {
			let signature = super::signature(scope, inclusions, span, &node)?;
			HType::Function(Box::new(signature))
		}
		"array_type" => {
			let kind = node.field(scope, "type")?;
			let size = node.field(scope, "size")?;
			let kind = self::kind(scope, inclusions, span, kind)?;
			let size = super::value(scope, inclusions, span, size);
			HType::Array(Box::new(kind), size)
		}
		"slice_type" => {
			let kind = node.field(scope, "type")?;
			let kind = self::kind(scope, inclusions, span, kind)?;
			HType::Slice(Box::new(kind))
		}
		"pointer" => {
			let kind = node.field(scope, "type")?;
			let kind = self::kind(scope, inclusions, span, kind)?;
			HType::Pointer(Box::new(kind))
		}
		"path" => path_kind(scope, inclusions, span, &node)?,
		_ => node.invalid(scope)?,
	}, TSpan::offset(span, node.span())))
}

fn path_kind<'a>(scope: MScope, inclusions: &Inclusions, span: &TSpan,
				 node: &impl Node<'a>) -> crate::Result<HType> {
	let path = super::path(scope, span, node)?.node;
	if let Path::Node(module, Identifier(name)) = &path {
		if matches!(module.as_ref(), Path::Root) {
			match name.as_ref() {
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
	let path = inclusions.structure(scope, &path)
		.ok_or_else(|| E::error().message("undefined type")
			.label(scope.span.label()).to(scope))?;
	Ok(HType::Structure(path))
}
