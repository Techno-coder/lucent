use iced_x86::Code;
use iced_x86::Instruction as I;
use iced_x86::MemoryOperand as M;

use crate::context::Context;
use crate::error::Diagnostic;
use crate::inference::Types;
use crate::node::{Dual, Size, Type, Unary, Value, ValueIndex, ValueNode};
use crate::span::Span;

use super::{Mode, Scene, Translation};

pub fn target(context: &Context, scene: &mut Scene, prime: &mut Translation,
			  types: &Types, value: &Value, index: &ValueIndex) -> crate::Result<M> {
	let span = &value[*index].span;
	match &value[*index].node {
		// TODO: path as start address intrinsic
		ValueNode::Path(_) => unimplemented!(),
		ValueNode::Variable(variable) => {
			let offset = scene.variables[variable] as i32;
			Ok(M::with_base_displ(scene.mode.base(), offset))
		}
		ValueNode::Field(index, field) => {
			let mut target = target(context, scene, prime, types, value, index)?;
			let offset = crate::node::offset(context, scene.parent.clone(),
				&types[index], &field.node, Some(span.clone()))?;
			target.displacement += offset as i32;
			assert_eq!(target.displ_size, 1);
			Ok(target)
		}
		ValueNode::Index(target, index) => match &types[target] {
			Type::Array(path, _) => {
				let scale = crate::node::size(context, scene
					.parent.clone(), &path.node, Some(span.clone()))?;
				let target = self::target(context, scene, prime, types, value, target)?;

				define_note!(note, prime, span);
				note(I::with_reg_mem(load(scene.mode), scene.mode_primary(), target));
				note(I::with_reg(super::code_push(scene.mode.size()), scene.mode_primary()));
				swap_restore(context, scene, prime, types, value, index, span)?;
				scale_index(scene, prime, &types[index], scale, Dual::Add, span)?;
				Ok(M::with_base(scene.mode_primary()))
			}
			Type::Slice(path) => {
				let scale = crate::node::size(context, scene
					.parent.clone(), &path.node, Some(span.clone()))?;
				super::value(context, scene, prime, types, value, target)?;
				prime.push(I::with_mem(super::code_push(scene.mode.size()),
					M::with_base(scene.mode_primary())), span);

				swap_restore(context, scene, prime, types, value, index, span)?;
				scale_index(scene, prime, &types[index], scale, Dual::Add, span)?;
				Ok(M::with_base(scene.mode_primary()))
			}
			other => panic!("cannot index into type: {}", other)
		}
		ValueNode::Unary(Unary::Dereference, index) => {
			super::value(context, scene, prime, types, value, index)?;
			Ok(M::with_base(scene.mode_primary()))
		}
		_ => context.pass(Diagnostic::error()
			.message("value is not addressable")
			.label(span.label())),
	}
}

pub fn scale_index(scene: &mut Scene, prime: &mut Translation, index: &Type,
				   scale: usize, dual: Dual, span: &Span) -> crate::Result<()> {
	define_note!(note, prime, span);
	let target = scene.mode.size();
	match index {
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
	Ok(note(I::with_reg_reg(match dual {
		Dual::Add => code_rm!(target, Add_, _r),
		Dual::Minus => code_rm!(target, Sub_, _r),
		other => panic!("invalid scale dual: {:?}", other),
	}, scene.primary[target], alternate)))
}

pub fn load(mode: Mode) -> Code {
	match mode {
		Mode::Protected => Code::Lea_r32_m,
		Mode::Long => Code::Lea_r64_m,
		Mode::Real => Code::Lea_r16_m,
	}
}

fn swap_restore(context: &Context, scene: &mut Scene, prime: &mut Translation,
				types: &Types, value: &Value, index: &ValueIndex,
				span: &Span) -> crate::Result<()> {
	std::mem::swap(&mut scene.primary, &mut scene.alternate);
	super::value(context, scene, prime, types, value, index)?;
	std::mem::swap(&mut scene.primary, &mut scene.alternate);
	Ok(prime.push(I::with_reg(super::code_pop(scene.mode
		.size()), scene.mode_primary()), span))
}
