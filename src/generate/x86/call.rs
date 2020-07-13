use iced_x86::Code;
use iced_x86::Instruction as I;
use iced_x86::MemoryOperand as M;

use crate::context::Context;
use crate::inference::Types;
use crate::node::{FunctionPath, Path, Size, Type, Value, ValueIndex};
use crate::span::{S, Span};

use super::{Mode, Scene, Translation};

pub fn call(context: &Context, scene: &mut Scene, prime: &mut Translation,
			types: &Types, value: &Value, index: &ValueIndex, path: &S<Path>,
			arguments: &[ValueIndex], span: &Span) -> crate::Result<()> {
	let reserved: Vec<_> = scene.reserved.iter().cloned().collect();
	reserved.iter().rev().for_each(|registers|
		prime.push(I::with_reg(super::code_push(scene.mode.size()),
			registers[scene.mode.size()]), span));

	// TODO: move and return registers
	let mut size = arguments.iter().rev().try_fold(0, |size, argument| {
		super::value(context, scene, prime, types, value, argument)?;
		if types[argument].composite() {
			let stack = crate::node::size(context, scene.parent
				.clone(), &types[argument], Some(span.clone()))?;
			super::stack_reserve(scene, prime, stack, span);
			let memory = M::with_base(scene.mode.stack());
			super::set(scene, prime, &types[argument], stack,
				memory, scene.mode_primary(), span);
			Ok(size + stack)
		} else {
			let stack = super::size(context, scene,
				&types[argument], span).map(super::stack)?;
			prime.push(I::with_reg(super::code_push(stack),
				scene.primary[stack]), span);
			Ok(size + stack.bytes())
		}
	})? as i32;

	let mut composite = None;
	if types[index].composite() {
		size += scene.mode.size().bytes() as i32;
		let size = crate::node::size(context, scene.parent
			.clone(), &types[index], Some(span.clone()))?;
		let offset = *composite.get_or_insert(scene.reserve(size));

		define_note!(note, prime, span);
		let memory = M::with_base_displ(scene.mode.base(), offset as i32);
		note(I::with_reg_mem(super::load(scene.mode), scene.mode_primary(), memory));
		note(I::with_reg(super::code_push(scene.mode.size()), scene.mode_primary()));
	}

	let call_index = prime.instructions.len();
	let path = FunctionPath(path.node.clone(), types.functions[index]);
	prime.calls.push((call_index, path));

	define_note!(note, prime, span);
	note(I::with_branch(relative!(scene.mode, Call), 0));
	note(I::with_reg_i32(match scene.mode {
		Mode::Protected => Code::Add_rm32_imm32,
		Mode::Long => Code::Add_rm64_imm32,
		Mode::Real => Code::Add_rm16_imm16,
	}, scene.mode.stack(), size));

	// TODO: calling convention dependent
	if let Some(offset) = composite {
		let memory = M::with_base_displ(scene.mode.base(), offset as i32);
		prime.push(I::with_reg_mem(super::load(scene.mode),
			scene.mode_primary(), memory), span);
	} else if !matches!(types[index], Type::Void | Type::Never) {
		let size = super::size(context, scene, &types[index], span)?;
		let (register, target) = (register!(size, A), scene.primary[size]);
		super::transfer(prime, register, target, size, span);
	}

	Ok(reserved.iter().for_each(|registers|
		prime.push(I::with_reg(super::code_pop(scene.mode.size()),
			registers[scene.mode.size()]), span)))
}
