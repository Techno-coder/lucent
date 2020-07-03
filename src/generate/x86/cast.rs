use iced_x86::Code;
use iced_x86::Instruction as I;

use crate::context::Context;
use crate::error::Diagnostic;
use crate::inference::Types;
use crate::node::{Size, Type, Value, ValueIndex};
use crate::span::{S, Span};

use super::{Scene, Translation};

pub fn cast(context: &Context, scene: &mut Scene, prime: &mut Translation,
			types: &Types, value: &Value, index: &ValueIndex, target: &S<Type>,
			span: &Span) -> crate::Result<()> {
	super::value(context, scene, prime, types, value, index)?;
	define_note!(note, prime, span);
	Ok(match (&types[index], &target.node) {
		(Type::Signed(size), Type::Signed(target)) |
		(Type::Signed(size), Type::Unsigned(target)) =>
			sign_extend(scene, *size, *target)
				.into_iter().for_each(note),
		(Type::Unsigned(size), Type::Signed(target)) |
		(Type::Unsigned(size), Type::Unsigned(target)) =>
			zero_extend(scene, *size, *target)
				.into_iter().for_each(note),
		// TODO: other casts
		(path, node) => return context.pass(Diagnostic::error()
			.label(span.label().with_message(path.to_string()))
			.label(target.span.label().with_message(node.to_string()))
			.message("cannot cast types")),
	})
}

pub fn zero_extend(scene: &Scene, size: Size, target: Size) -> Option<I> {
	Some(I::with_reg_reg(match (size, target) {
		(Size::Byte, Size::Word) => Code::Movzx_r16_rm8,
		(Size::Byte, Size::Double) => Code::Movzx_r32_rm8,
		(Size::Byte, Size::Quad) => Code::Movzx_r64_rm8,
		(Size::Word, Size::Double) => Code::Movzx_r32_rm16,
		(Size::Word, Size::Quad) => Code::Movzx_r64_rm16,
		_ => return None,
	}, scene.primary[target], scene.primary[size]))
}

pub fn sign_extend(scene: &Scene, size: Size, target: Size) -> Option<I> {
	Some(I::with_reg_reg(match (size, target) {
		(Size::Byte, Size::Word) => Code::Movsx_r16_rm8,
		(Size::Byte, Size::Double) => Code::Movsx_r32_rm8,
		(Size::Byte, Size::Quad) => Code::Movsx_r64_rm8,
		(Size::Word, Size::Double) => Code::Movsx_r32_rm16,
		(Size::Word, Size::Quad) => Code::Movsx_r64_rm16,
		(Size::Double, Size::Quad) => Code::Movsxd_r64_rm32,
		_ => return None,
	}, scene.primary[target], scene.primary[size]))
}
