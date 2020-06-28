use iced_x86::{Code, Register};
use iced_x86::Instruction as I;

use crate::context::Context;
use crate::inference::Types;
use crate::node::{Binary, Compare, Dual, Size, Type, Value, ValueIndex};
use crate::span::Span;

use super::{Scene, Translation};

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
	if matches!(binary, Binary::And) || matches!(binary, Binary::Or) {
		return short(context, scene, prime, types, value, binary, left, right, span);
	}

	super::value(context, scene, prime, types, value, right)?;
	let register = super::push_value(prime, &types[right], span).unwrap();
	super::value(context, scene, prime, types, value, left)?;
	let alternate = super::alternate(prime, &types[right], span).unwrap();
	define_note!(note, prime, span);

	Ok(match binary {
		Binary::Or => unreachable!(),
		Binary::And => unreachable!(),
		Binary::Compare(compare) => {
			let code = match &types[left] {
				Type::Truth => Code::Cmp_r8_rm8,
				Type::Rune => Code::Cmp_r32_rm32,
				Type::Pointer(_) => Code::Cmp_r64_rm64,
				Type::Signed(size) => code_rm!(size, Cmp_, _r),
				Type::Unsigned(size) => code_rm!(size, Cmp_, _r),
				other => panic!("invalid comparison type: {}", other),
			};

			note(I::with_reg_reg(code, register, alternate));
			note(I::with_reg(match compare {
				Compare::Less => Code::Setl_rm8,
				Compare::Greater => Code::Setg_rm8,
				Compare::LessEqual => Code::Setle_rm8,
				Compare::GreaterEqual => Code::Setge_rm8,
				Compare::NotEqual => Code::Setne_rm8,
				Compare::Equal => Code::Sete_rm8,
			}, Register::AL));
		}
		Binary::Dual(dual @ Dual::Divide) |
		Binary::Dual(dual @ Dual::Modulo) |
		Binary::Dual(dual @ Dual::Multiply) => {
			if dual != &Dual::Multiply {
				note(match &types[left] {
					Type::Signed(size) => I::with(match size {
						Size::Byte => Code::Cbw,
						Size::Word => Code::Cwd,
						Size::Double => Code::Cdq,
						Size::Quad => Code::Cqo,
					}),
					Type::Unsigned(size) => {
						let register = register!(size, D);
						let code = code_rm!(size, Xor_, _r);
						I::with_reg_reg(code, register, register)
					}
					other => panic!("invalid arithmetic type: {}", other),
				})
			}

			note(I::with_reg(match dual {
				Dual::Multiply => match &types[left] {
					Type::Signed(size) => code_m!(size, Imul_r),
					Type::Unsigned(size) => code_m!(size, Mul_r),
					other => panic!("invalid arithmetic type: {}", other),
				}
				Dual::Divide | Dual::Modulo => match &types[left] {
					Type::Signed(size) => code_m!(size, Idiv_r),
					Type::Unsigned(size) => code_m!(size, Div_r),
					other => panic!("invalid arithmetic type: {}", other),
				}
				_ => unreachable!(),
			}, alternate));

			if dual == &Dual::Modulo {
				let size = match &types[left] {
					Type::Signed(size) | Type::Unsigned(size) => size,
					other => panic!("invalid arithmetic type: {}", other),
				};

				let alternate = register!(size, D);
				let code = code_rm!(size, Mov_, _r);
				note(I::with_reg_reg(code, register, alternate));
			}
		}
		Binary::Dual(dual @ Dual::ShiftLeft) |
		Binary::Dual(dual @ Dual::ShiftRight) => {
			note(I::with_reg_reg(Code::Mov_r8_rm8, Register::CL, Register::BL));
			note(I::with_reg_reg(match (dual, &types[left]) {
				(Dual::ShiftLeft, Type::Signed(size)) => code_m!(size, Sal_r, _CL),
				(Dual::ShiftLeft, Type::Unsigned(size)) => code_m!(size, Shl_r, _CL),
				(Dual::ShiftRight, Type::Signed(size)) => code_m!(size, Sar_r, _CL),
				(Dual::ShiftRight, Type::Unsigned(size)) => code_m!(size, Shr_r, _CL),
				(_, other) => panic!("invalid arithmetic type: {}", other),
			}, register, Register::CL));
		}
		Binary::Dual(dual) => {
			if let Type::Pointer(path) = &types[left] {
				let scale = crate::node::size(context, scene
					.parent.clone(), &path.node, Some(span.clone()))?;
				note(I::with_reg_reg(Code::Xchg_r64_RAX, Register::RBX, Register::RAX));

				match &types[right] {
					Type::Signed(size) => super::sign_extend(size, &Size::Quad)
						.into_iter().for_each(|instruction| note(instruction)),
					Type::Unsigned(size) => super::zero_extend(size, &Size::Quad)
						.into_iter().for_each(|instruction| note(instruction)),
					other => panic!("invalid arithmetic type: {}", other),
				}

				let (code, register) = (Code::Imul_r64_rm64_imm32, Register::RAX);
				note(I::with_reg_reg_i32(code, register, register, scale as i32));
				if dual == &Dual::Minus { note(I::with_reg(Code::Neg_rm64, Register::RAX)); }
				note(I::with_reg_reg(Code::Add_r64_rm64, Register::RAX, Register::RBX));
				return Ok(());
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
	let exit = scene.label();
	super::value(context, scene, prime, types, value, left)?;

	define_note!(note, prime, span);
	note(I::with_reg_reg(Code::Test_rm8_r8,
		Register::AL, Register::AL));
	note(I::with_branch(match binary {
		Binary::And => Code::Je_rel32_64,
		Binary::Or => Code::Jne_rel32_64,
		_ => unreachable!(),
	}, exit));

	super::value(context, scene, prime, types, value, right)?;
	Ok(prime.set_pending_label(exit, span))
}
