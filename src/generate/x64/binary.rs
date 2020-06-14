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
				Type::Signed(size) | Type::Unsigned(size) => match size {
					Size::Byte => Code::Cmp_r8_rm8,
					Size::Word => Code::Cmp_r16_rm16,
					Size::Double => Code::Cmp_r32_rm32,
					Size::Quad => Code::Cmp_r64_rm64,
				}
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
		Binary::Dual(dual) => {
			let code = match dual {
				Dual::Add => match &types[left] {
					Type::Signed(size) | Type::Unsigned(size) => match size {
						Size::Byte => Code::Add_r8_rm8,
						Size::Word => Code::Add_r16_rm16,
						Size::Double => Code::Add_r32_rm32,
						Size::Quad => Code::Add_r64_rm64,
					},
					Type::Pointer(_) => unimplemented!(),
					other => panic!("invalid arithmetic type: {}", other),
				}
				_ => unimplemented!(),
			};

			note(I::with_reg_reg(code, register, alternate));
		}
		_ => unimplemented!()
	})
}
