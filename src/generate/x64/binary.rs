use iced_x86::{Code, Register};
use iced_x86::Instruction as I;

use crate::context::Context;
use crate::inference::Types;
use crate::node::{Binary, Compare, Dual, Size, Type, Value, ValueIndex};
use crate::span::Span;

use super::{Scene, Translation};

pub fn binary(context: &Context, scene: &mut Scene, prime: &mut Translation,
			  types: &Types, value: &Value, binary: &Binary, left: &ValueIndex,
			  right: &ValueIndex, span: &Span) -> crate::Result<()> {
	super::value(context, scene, prime, types, value, right)?;
	let register = super::push_value(prime, &types[right], span).unwrap();
	super::value(context, scene, prime, types, value, left)?;
	let alternate = super::alternate(prime, &types[right], span).unwrap();
	define_note!(note, prime, span);

	Ok(match binary {
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
		Binary::Dual(dual) => {
			note(I::with_reg_reg(match dual {
				Dual::Add => match &types[left] {
					Type::Pointer(_) => unimplemented!(),
					Type::Signed(size) => code_rm!(size, Add_, _r),
					Type::Unsigned(size) => code_rm!(size, Add_, _r),
					other => panic!("invalid arithmetic type: {}", other),
				}
				Dual::Divide | Dual::Modulo |
				Dual::Multiply => unreachable!(),
				_ => unimplemented!(),
			}, register, alternate));
		}
		_ => unimplemented!()
	})
}
