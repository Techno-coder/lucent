use iced_x86::{Code, Register};
use iced_x86::Instruction as I;

use crate::context::Context;
use crate::inference::Types;
use crate::node::{Binary, Compare, Dual, Size, Type, Value, ValueIndex};
use crate::span::Span;

use super::{Mode, Scene, Translation};

macro_rules! integer_dual {
    ($type:expr, $left:ident, $right: ident) => {
		match $type {
			Type::Signed(size) => code_rm!(size, $left, $right),
			Type::Unsigned(size) => code_rm!(size, $left, $right),
			other => panic!("invalid arithmetic type: {}", other),
		}
    };
}

macro_rules! binary_dual {
    ($type:expr, $left:ident, $right: ident) => {{
    	use Code::*;
		match $type {
			Type::Truth => concat_idents!($left, r8, $right, m8),
			Type::Signed(size) => code_rm!(size, $left, $right),
			Type::Unsigned(size) => code_rm!(size, $left, $right),
			other => panic!("invalid arithmetic type: {}", other),
		}
    }};
}

pub fn binary(context: &Context, scene: &mut Scene, prime: &mut Translation,
			  types: &Types, value: &Value, binary: &Binary, left: &ValueIndex,
			  right: &ValueIndex, span: &Span) -> crate::Result<()> {
	if matches!(binary, Binary::Or) || matches!(binary, Binary::And) {
		return short(context, scene, prime, types, value, binary, left, right, span);
	}

	super::value(context, scene, prime, types, value, left)?;
	let size = super::size(context, scene, &types[left], span)?;
	let (register, alternate) = (scene.primary[size], scene.alternate[size]);
	let stack = super::size(context, scene, &types[left], span).map(super::stack)?;
	let stack_primary = scene.primary[stack];

	define_note!(note, prime, span);
	note(I::with_reg(super::code_push(stack), stack_primary));
	std::mem::swap(&mut scene.primary, &mut scene.alternate);
	super::value(context, scene, prime, types, value, right)?;

	define_note!(note, prime, span);
	std::mem::swap(&mut scene.primary, &mut scene.alternate);
	note(I::with_reg(super::code_pop(stack), stack_primary));

	Ok(match binary {
		Binary::Or | Binary::And => unreachable!(),
		Binary::Compare(compare) => {
			note(I::with_reg_reg(code_rm!(size,
				Cmp_, _r), register, alternate));
			note(I::with_reg(match compare {
				Compare::Less => Code::Setl_rm8,
				Compare::Greater => Code::Setg_rm8,
				Compare::LessEqual => Code::Setle_rm8,
				Compare::GreaterEqual => Code::Setge_rm8,
				Compare::NotEqual => Code::Setne_rm8,
				Compare::Equal => Code::Sete_rm8,
			}, scene.primary[Size::Byte]));
		}
		Binary::Dual(Dual::Multiply) => match size {
			Size::Byte => super::reserve(scene, prime, Register::AL, |scene, prime| {
				super::convey(scene, prime, alternate, Register::BL, |_, prime, other| {
					super::transfer(prime, register, Register::AL, size, span);
					prime.push(I::with_reg(Code::Imul_rm8, other), span);
				}, size, span);
				super::transfer(prime, Register::AL, register, size, span);
			}, size, span),
			other => note(I::with_reg_reg(match other {
				Size::Byte => unreachable!(),
				Size::Word => Code::Imul_r16_rm16,
				Size::Double => Code::Imul_r32_rm32,
				Size::Quad => Code::Imul_r64_rm64,
			}, register, alternate)),
		},
		Binary::Dual(dual @ Dual::Divide) |
		Binary::Dual(dual @ Dual::Modulo) => {
			let (target, clear) = (register!(size, A), register!(size, D));
			super::reserve(scene, prime, target, |scene, prime| {
				super::reserve(scene, prime, clear, |scene, prime| {
					super::convey(scene, prime, alternate,
						register!(size, B), |_, prime, other| {
							super::transfer(prime, register, target, size, span);
							define_note!(note, prime, span);
							match &types[left] {
								Type::Signed(size) => {
									note(I::with(super::code_sign_extend(*size)));
									note(I::with_reg(code_m!(size, Idiv_r), other));
								}
								Type::Unsigned(size) => {
									let code = code_rm!(size, Xor_, _r);
									note(I::with_reg_reg(code, clear, clear));
									note(I::with_reg(code_m!(size, Div_r), other));
								}
								other => panic!("invalid arithmetic type: {}", other),
							}
						}, size, span);
					super::transfer(prime, match dual {
						Dual::Divide => register!(size, A),
						Dual::Modulo => register!(size, D),
						_ => unreachable!(),
					}, register, size, span);
				}, size, span)
			}, size, span)
		}
		Binary::Dual(dual @ Dual::ShiftLeft) |
		Binary::Dual(dual @ Dual::ShiftRight) =>
			super::reserve(scene, prime, Register::CL, |scene, prime| {
				super::convey(scene, prime, register,
					register!(size, A), |scene, prime, other| {
						let alternate = scene.alternate[Size::Byte];
						super::transfer(prime, alternate, Register::CL, Size::Byte, span);
						prime.push(I::with_reg_reg(match (dual, &types[left]) {
							(Dual::ShiftLeft, Type::Signed(size)) => code_m!(size, Sal_r, _CL),
							(Dual::ShiftLeft, Type::Unsigned(size)) => code_m!(size, Shl_r, _CL),
							(Dual::ShiftRight, Type::Signed(size)) => code_m!(size, Sar_r, _CL),
							(Dual::ShiftRight, Type::Unsigned(size)) => code_m!(size, Shr_r, _CL),
							(_, other) => panic!("invalid arithmetic type: {}", other),
						}, other, Register::CL), span);
						super::transfer(prime, other, register, size, span);
					}, size, span);
			}, size, span),
		Binary::Dual(dual) => {
			if let Type::Pointer(path) = &types[left] {
				let scale = crate::node::size(context, scene
					.parent.clone(), &path.node, Some(span.clone()))?;
				let target = scene.mode.size();
				match &types[right] {
					Type::Signed(size) => super::sign_extend(scene, *size, target)
						.into_iter().for_each(|instruction| note(instruction)),
					Type::Unsigned(size) => super::zero_extend(scene, *size, target)
						.into_iter().for_each(|instruction| note(instruction)),
					other => panic!("invalid arithmetic type: {}", other),
				}

				let alternate = scene.alternate[target];
				note(I::with_reg_reg_i32(match scene.mode {
					Mode::Protected => Code::Imul_r32_rm32_imm32,
					Mode::Long => Code::Imul_r64_rm64_imm32,
					Mode::Real => Code::Imul_r16_rm16_imm16,
				}, alternate, alternate, scale as i32));
				return Ok(note(I::with_reg_reg(match dual {
					Dual::Add => code_rm!(target, Add_, _r),
					Dual::Minus => code_rm!(target, Sub_, _r),
					other => panic!("invalid pointer dual: {:?}", other),
				}, scene.primary[target], alternate)));
			}

			note(I::with_reg_reg(match dual {
				Dual::Add => integer_dual!(&types[left], Add_, _r),
				Dual::Minus => integer_dual!(&types[left], Sub_, _r),
				Dual::Divide | Dual::Modulo | Dual::Multiply => unreachable!(),
				Dual::ShiftLeft | Dual::ShiftRight => unreachable!(),
				Dual::BinaryOr => binary_dual!(&types[left], Or_, _r),
				Dual::BinaryAnd => binary_dual!(&types[left], And_, _r),
				Dual::ExclusiveOr => binary_dual!(&types[left], Xor_, _r),
			}, register, alternate));
		}
	})
}

fn short(context: &Context, scene: &mut Scene, prime: &mut Translation,
		 types: &Types, value: &Value, binary: &Binary, left: &ValueIndex,
		 right: &ValueIndex, span: &Span) -> crate::Result<()> {
	super::value(context, scene, prime, types, value, left)?;
	define_note!(note, prime, span);
	let register = scene.primary[Size::Byte];
	note(I::with_reg_reg(Code::Test_rm8_r8, register, register));

	let exit = scene.label();
	note(I::with_branch(match binary {
		Binary::Or => relative!(scene.mode, Jne),
		Binary::And => relative!(scene.mode, Je),
		_ => unreachable!(),
	}, exit));

	super::value(context, scene, prime, types, value, right)?;
	Ok(prime.set_pending_label(exit, span))
}
