use iced_x86::{Code, Register};
use iced_x86::Instruction as I;
use iced_x86::MemoryOperand as M;

use crate::context::Context;
use crate::inference::Types;
use crate::node::{Function, Parameter, ReturnType, Size, Type, Value, ValueIndex};
use crate::span::Span;

use super::{Mode, Scene, Translation};

pub fn entry(scene: &Scene, prime: &mut Translation, internal: &Span) {
	let (size, base) = (scene.mode.size(), scene.mode.base());
	prime.push(I::with_reg(super::code_push(size), base), internal);
	super::transfer(prime, scene.mode.stack(), base, size, internal);
	stack_reserve(scene, prime, 0, internal);
}

pub fn render(context: &Context, scene: &mut Scene, prime: &mut Translation,
			  types: &Types, value: &Value, index: Option<ValueIndex>,
			  span: &Span) -> crate::Result<()> {
	if let Some(index) = index {
		// TODO: calling convention dependent
		super::value(context, scene, prime, types, value, &index)?;
		match &types[&index] {
			Type::Void => (),
			Type::Never => return Ok(()),
			path if path.composite() => {
				let size = crate::node::size(context,
					scene.parent.clone(), path, Some(span.clone()))?;
				let offset = scene.parameters_offset() as i32;
				let memory = M::with_base_displ(scene.mode.base(), offset);

				region(scene, prime, |scene, prime, registers| {
					let (source, counter, target) = registers;
					super::transfer(prime, scene.mode_primary(),
						source, scene.mode.size(), span);
					define_note!(note, prime, span);
					let code = code_rm!(scene.mode.size(), Mov_, _r);
					note(I::with_reg_mem(code, target, memory));
					let code = code_rm!(scene.mode.size(), Mov_, _im);
					note(I::with_reg_i32(code, counter, size as i32));
					note(I::with_rep_movsb(scene.mode.size() as u32));
				}, span);
			}
			path => {
				let size = super::size(context, scene, path, span)?;
				let (register, target) = (scene.primary[size], register!(size, A));
				super::transfer(prime, register, target, size, span);
			}
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

// TODO: calling convention dependent
pub fn parameters(context: &Context, function: &Function,
				  scene: &mut Scene) -> crate::Result<()> {
	let mut offset = scene.parameters_offset();
	if let ReturnType::Type(path) = &function.return_type.node {
		if path.node.composite() { offset += scene.mode.size().bytes(); }
	}

	Ok(for parameter in &function.parameters {
		if let Parameter::Variable(variable, path) = &parameter.node {
			scene.variables.insert(variable.node.clone(), offset as isize);
			let size = crate::node::size(context, scene.parent.clone(),
				&path.node, Some(variable.span.clone()))?;
			offset += if size == 1 { 2 } else { size }
		}
	})
}

pub fn stack_reserve(scene: &Scene, prime: &mut Translation,
					 size: usize, span: &Span) {
	prime.push(I::with_reg_i32(match scene.mode {
		Mode::Protected => Code::Sub_rm32_imm32,
		Mode::Long => Code::Sub_rm64_imm32,
		Mode::Real => Code::Sub_rm16_imm16,
	}, scene.mode.stack(), size as i32), span);
}

pub fn zero(scene: &mut Scene, prime: &mut Translation,
			offset: isize, size: usize, span: &Span) {
	let target = scene.mode.destination();
	let counter = register!(scene.mode.size(), C);
	super::reserve(scene, prime, counter, |scene, prime| {
		super::reserve(scene, prime, target, |scene, prime| {
			super::reserve(scene, prime, Register::AL, |scene, prime| {
				define_note!(note, prime, span);
				let memory = M::with_base_displ(scene.mode.base(), offset as i32);
				note(I::with_reg_reg(Code::Xor_r8_rm8, Register::AL, Register::AL));

				let code = code_rm!(scene.mode.size(), Mov_, _im);
				note(I::with_reg_i32(code, counter, size as i32));
				note(I::with_reg_mem(super::load(scene.mode), target, memory));
				note(I::with_rep_stosb(scene.mode.size() as u32));
			}, Size::Byte, span);
		}, Size::Byte, span);
	}, Size::Byte, span);
}

pub fn set(scene: &mut Scene, prime: &mut Translation, path: &Type,
		   size: usize, mut memory: M, register: Register, span: &Span) {
	memory.displ_size = 1;
	match path.composite() {
		false => prime.push(I::with_mem_reg(match path {
			Type::Truth => Code::Mov_rm8_r8,
			Type::Rune => Code::Mov_rm32_r32,
			Type::Pointer(_) => Code::Mov_rm64_r64,
			Type::Signed(size) | Type::Unsigned(size) => match size {
				Size::Byte => Code::Mov_rm8_r8,
				Size::Word => Code::Mov_rm16_r16,
				Size::Double => Code::Mov_rm32_r32,
				Size::Quad => Code::Mov_rm64_r64,
			},
			Type::Slice(_) | Type::Array(_, _) | Type::Structure(_)
			| Type::Void | Type::Never => unreachable!(),
		}, memory, register), span),
		true => region(scene, prime, |scene, prime, registers| {
			let (source, counter, target) = registers;
			let default = register!(scene.mode.size(), A);
			super::convey(scene, prime, register, default, |scene, prime, register| {
				prime.push(I::with_reg_mem(super::load(scene.mode), target, memory), span);
				super::transfer(prime, register, source, scene.mode.size(), span);

				define_note!(note, prime, span);
				let code = code_rm!(scene.mode.size(), Mov_, _im);
				note(I::with_reg_i32(code, counter, size as i32));
				note(I::with_rep_movsb(scene.mode.size() as u32));
			}, scene.mode.size(), span)
		}, span),
	}
}

fn region<F>(scene: &mut Scene, prime: &mut Translation, function: F, span: &Span)
	where F: FnOnce(&mut Scene, &mut Translation, (Register, Register, Register)) {
	let target = scene.mode.destination();
	let counter = register!(scene.mode.size(), C);
	let source = match scene.mode {
		Mode::Protected => Register::ESI,
		Mode::Long => Register::RSI,
		Mode::Real => Register::SI,
	};

	super::reserve(scene, prime, source, |scene, prime| {
		super::reserve(scene, prime, counter, |scene, prime| {
			super::reserve(scene, prime, target, |scene, prime| {
				function(scene, prime, (source, counter, target));
			}, scene.mode.size(), span);
		}, scene.mode.size(), span);
	}, scene.mode.size(), span);
}
