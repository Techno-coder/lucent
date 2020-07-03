use iced_x86::{Code, Register};
use iced_x86::Instruction as I;
use iced_x86::MemoryOperand as M;

use crate::context::Context;
use crate::inference::Types;
use crate::node::{Function, Parameter, Size, Type, Value, ValueIndex};
use crate::span::Span;

use super::{Mode, Scene, Translation};

pub fn entry(scene: &Scene, prime: &mut Translation, internal: &Span) {
	let size = scene.mode.size();
	let base = scene.mode.base();
	define_note!(note, prime, internal);
	note(I::with_reg(super::code_push(size), base));
	super::transfer(prime, scene.mode.stack(), base, size, internal);

	define_note!(note, prime, internal);
	note(I::with_reg_i32(match scene.mode {
		Mode::Protected => Code::Sub_rm32_imm32,
		Mode::Long => Code::Sub_rm64_imm32,
		Mode::Real => Code::Sub_rm16_imm16,
	}, scene.mode.stack(), 0));
}

pub fn render(context: &Context, scene: &mut Scene, prime: &mut Translation,
			  types: &Types, value: &Value, index: Option<ValueIndex>,
			  span: &Span) -> crate::Result<()> {
	if let Some(index) = index {
		// TODO: return large value by pointer set
		super::value(context, scene, prime, types, value, &index)?;
		if matches!(types[&index], Type::Never) { return Ok(()); }
		if !matches!(types[&index], Type::Void) {
			// TODO: calling convention dependent
			let size = super::size(context, scene, &types[&index], span)?;
			let (register, target) = (scene.primary[size], register!(size, A));
			super::transfer(prime, register, target, size, span);
		}
	}

	define_note!(note, prime, span);
	note(I::with(match scene.mode {
		Mode::Protected => Code::Leaved,
		Mode::Long => Code::Leaveq,
		Mode::Real => Code::Leavew,
	}));

	Ok(note(I::with(match scene.mode {
		Mode::Protected => Code::Retnd,
		Mode::Long => Code::Retnq,
		Mode::Real => Code::Retnw,
	})))
}

pub fn parameters(context: &Context, function: &Function,
				  scene: &mut Scene) -> crate::Result<()> {
	let mut offset = scene.mode.size().bytes() * 2;
	Ok(for parameter in &function.parameters {
		if let Parameter::Variable(variable, path) = &parameter.node {
			scene.variables.insert(variable.node.clone(), offset as isize);
			let size = crate::node::size(context, scene.parent.clone(),
				&path.node, Some(variable.span.clone()))?;
			offset += if size == 1 { 2 } else { size }
		}
	})
}

pub fn set(prime: &mut Translation, path: &Type, _size: usize,
		   mut memory: M, register: Register, span: &Span) {
	let code = match path {
		Type::Truth => Code::Mov_rm8_r8,
		Type::Rune => Code::Mov_rm32_r32,
		Type::Pointer(_) => Code::Mov_rm64_r64,
		Type::Signed(size) | Type::Unsigned(size) => match size {
			Size::Byte => Code::Mov_rm8_r8,
			Size::Word => Code::Mov_rm16_r16,
			Size::Double => Code::Mov_rm32_r32,
			Size::Quad => Code::Mov_rm64_r64,
		},
		Type::Slice(_) => unimplemented!(),
		Type::Array(_, _) => unimplemented!(),
		Type::Structure(_) => unimplemented!(),
		Type::Void | Type::Never => unreachable!(),
	};

	memory.displ_size = 1;
	define_note!(note, prime, span);
	note(I::with_mem_reg(code, memory, register));
}
