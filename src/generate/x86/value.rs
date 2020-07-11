use iced_x86::Code;
use iced_x86::Instruction as I;
use iced_x86::MemoryOperand as M;

use crate::context::Context;
use crate::error::Diagnostic;
use crate::inference::Types;
use crate::node::*;

use super::{Mode, Scene, Translation};

pub fn value(context: &Context, scene: &mut Scene, prime: &mut Translation,
			 types: &Types, value: &Value, index: &ValueIndex) -> crate::Result<()> {
	let span = &value[*index].span;
	define_note!(note, prime, span);
	Ok(match &value[*index].node {
		ValueNode::Block(block) => block.iter().try_for_each(|index|
			self::value(context, scene, prime, types, value, index))?,
		ValueNode::Let(_, _, None) => unimplemented!(),
		ValueNode::Let(variable, _, Some(index)) => {
			self::value(context, scene, prime, types, value, index)?;
			let size = crate::node::size(context, scene.parent.clone(),
				&types.variables[&variable.node], Some(span.clone()))?;
			let offset = scene.variable(variable.node.clone(), size);
			let memory = M::with_base_displ(scene.mode.base(), offset as i32);
			let register = super::size(context, scene, &types[index], span)?;
			super::set(prime, &types[index], size, memory,
				scene.primary[register], span);
		}
		ValueNode::Set(target, index) => {
			self::value(context, scene, prime, types, value, index)?;
			let size = super::size(context, scene, &types[index], span)?;
			let stack = super::stack(size);

			define_note!(note, prime, span);
			note(I::with_reg(super::code_push(stack), scene.primary[stack]));
			let target = super::target(context, scene, prime, types, value, target)?;

			define_note!(note, prime, span);
			let node_size = crate::node::size(context, scene
				.parent.clone(), &types[index], Some(span.clone()))?;
			note(I::with_reg(super::code_pop(stack), scene.alternate[stack]));
			super::set(prime, &types[index], node_size, target,
				scene.alternate[size], span);
		}
		ValueNode::While(condition, index) => {
			let entry = *prime.pending_label.get_or_insert_with(|| scene.label());
			self::value(context, scene, prime, types, value, condition)?;

			let exit = scene.label();
			define_note!(note, prime, span);
			let register = scene.primary[Size::Byte];
			note(I::with_reg_reg(Code::Test_rm8_r8, register, register));
			note(I::with_branch(relative!(scene.mode, Je), exit));
			self::value(context, scene, prime, types, value, index)?;

			define_note!(note, prime, span);
			note(I::with_branch(relative!(scene.mode, Jmp), entry));
			prime.set_pending_label(exit, span);
		}
		ValueNode::When(branches) => {
			let mut complete = false;
			let labels: Result<Vec<_>, _> = branches.iter().map(|(condition, _)| {
				self::value(context, scene, prime, types, value, condition)?;
				complete |= matches!(value[*condition].node, ValueNode::Truth(true));

				let label = scene.label();
				define_note!(note, prime, span);
				let register = scene.primary[Size::Byte];
				note(I::with_reg_reg(Code::Test_rm8_r8, register, register));
				note(I::with_branch(relative!(scene.mode, Jne), label));
				Ok(label)
			}).collect();

			let exit = scene.label();
			define_note!(note, prime, span);
			if !complete { note(I::with_branch(relative!(scene.mode, Jmp), exit)); }
			let iterator = Iterator::zip(labels?.into_iter(), branches.iter());
			for (index, (label, (_, branch))) in iterator.enumerate() {
				prime.set_pending_label(label, span);
				self::value(context, scene, prime, types, value, branch)?;

				if index + 1 != branches.len() {
					define_note!(note, prime, span);
					note(I::with_branch(relative!(scene.mode, Jmp), exit));
				}
			}

			prime.set_pending_label(exit, span);
		}
		ValueNode::Cast(index, target) => super::cast(context,
			scene, prime, types, value, index, target, span)?,
		ValueNode::Return(index) => super::render(context,
			scene, prime, types, value, *index, span)?,
		ValueNode::Compile(_) => unimplemented!(),
		ValueNode::Inline(_) => unimplemented!(),
		ValueNode::Call(path, arguments) => {
			// TODO: provide argument for structure returns
			let size = arguments.iter().rev().try_fold(0, |size, argument| {
				// TODO: copy structures onto stack
				self::value(context, scene, prime, types, value, argument)?;
				let stack = super::size(context, scene,
					&types[argument], span).map(super::stack)?;

				define_note!(note, prime, span);
				note(I::with_reg(super::code_push(stack), scene.primary[stack]));
				Ok(size + crate::node::size(context, scene.parent.clone(),
					&types[argument], Some(span.clone()))?)
			})? as i32;

			let call_index = prime.instructions.len();
			let (path, kind) = (path.node.clone(), types.functions[&index]);
			prime.calls.push((call_index, FunctionPath(path, kind)));

			define_note!(note, prime, span);
			note(I::with_branch(relative!(scene.mode, Call), 0));
			note(I::with_reg_i32(match scene.mode {
				Mode::Protected => Code::Add_rm32_imm32,
				Mode::Long => Code::Add_rm64_imm32,
				Mode::Real => Code::Add_rm16_imm16,
			}, scene.mode.stack(), size));
			// TODO: set structure return as pointer

			if !matches!(types[index], Type::Void) {
				// TODO: calling convention dependent
				let size = super::size(context, scene, &types[index], span)?;
				let (register, target) = (register!(size, A), scene.primary[size]);
				super::transfer(prime, register, target, size, span);
			}
		}
		ValueNode::Field(_, _) => unimplemented!(),
		ValueNode::Create(_, _) => unimplemented!(),
		ValueNode::Slice(_, _, _) => unimplemented!(),
		ValueNode::Index(_, _) => unimplemented!(),
		ValueNode::Compound(dual, target, index) => {
			super::binary(context, scene, prime, types,
				value, &Binary::Dual(*dual), target, index, span)?;
			let size = super::size(context, scene, &types[index], span)?;
			let stack = super::stack(size);

			define_note!(note, prime, span);
			note(I::with_reg(super::code_push(stack), scene.primary[stack]));
			let target = super::target(context, scene, prime, types, value, target)?;

			define_note!(note, prime, span);
			let node_size = crate::node::size(context, scene
				.parent.clone(), &types[index], Some(span.clone()))?;
			note(I::with_reg(super::code_pop(stack), scene.alternate[stack]));
			super::set(prime, &types[index], node_size, target,
				scene.alternate[size], span);
		}
		ValueNode::Binary(binary, left, right) => super::binary(context,
			scene, prime, types, value, binary, left, right, span)?,
		ValueNode::Unary(unary, index) => {
			if let Unary::Reference = unary {
				let instruction = I::with_reg_mem(super::load(scene.mode),
					scene.mode_primary(), super::target(context,
						scene, prime, types, value, index)?);
				return Ok(prime.push(instruction, span));
			}

			let size = super::size(context, scene, &types[index], span)?;
			self::value(context, scene, prime, types, value, index)?;
			let primary = scene.primary[size];
			define_note!(note, prime, span);
			match unary {
				Unary::Reference => unreachable!(),
				Unary::Not => note(match types[index] {
					Type::Truth => I::with_reg_i32(Code::Xor_rm8_imm8,
						scene.primary[Size::Byte], 1),
					_ => I::with_reg(code_m!(size, Not_r), primary),
				}),
				Unary::Negate => note(I::with_reg(code_m!(size, Neg_r), primary)),
				Unary::Dereference => if types[index].composite() {
					let node_size = crate::node::size(context, scene
						.parent.clone(), &types[index], Some(span.clone()))?;
					let offset = scene.reserve(node_size) as i32;
					let memory = M::with_base_displ(scene.mode.base(), offset);
					super::set(prime, &types[index], node_size,
						memory, primary, span);
				},
			}
		}
		ValueNode::Variable(variable) => {
			let offset = scene.variables[variable] as i32;
			let memory = M::with_base_displ(scene.mode.base(), offset);
			let path = &types.variables[variable];

			let size = super::size(context, scene, path, span)?;
			note(I::with_reg_mem(match path.composite() {
				false => code_rm!(size, Mov_, _r),
				true => super::load(scene.mode),
			}, scene.primary[size], memory));
		}
		ValueNode::Path(_) => unimplemented!(),
		ValueNode::String(_) => unimplemented!(),
		ValueNode::Register(_) => unimplemented!(),
		ValueNode::Array(_) => unimplemented!(),
		ValueNode::Integral(integral) => match types[index] {
			Type::Signed(size) | Type::Unsigned(size) => {
				let integral = *integral as i64;
				let register = scene.primary[size];
				let code = code_rm!(size, Mov_, _im);
				note(I::with_reg_i64(code, register, integral));
			}
			_ => panic!("type is not integral"),
		},
		ValueNode::Truth(truth) =>
			note(I::with_reg_u32(Code::Mov_r8_imm8,
				scene.primary[Size::Byte], *truth as u32)),
		ValueNode::Rune(rune) => match scene.mode {
			Mode::Real => return context.pass(Diagnostic::error()
				.message("rune type unsupported for architecture x16")
				.label(span.label()).note("use byte literals")),
			_ => note(I::with_reg_u32(Code::Mov_r32_imm32,
				scene.primary[Size::Double], *rune as u32)),
		}
		ValueNode::Break => unimplemented!(),
	})
}
