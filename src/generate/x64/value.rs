use iced_x86::{Code, Register};
use iced_x86::Instruction as I;
use iced_x86::MemoryOperand as M;

use crate::context::Context;
use crate::error::Diagnostic;
use crate::inference::Types;
use crate::node::{Binary, FunctionPath, Size, Type, Value, ValueIndex, ValueNode};
use crate::span::Span;

use super::{Scene, Translation};

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
			let memory = M::with_base_displ(Register::RBP, offset as i32);
			super::set(prime, &types[index], size, memory, match types[index] {
				Type::Signed(size) | Type::Unsigned(size) => register!(size, A),
				Type::Pointer(_) | Type::Slice(_) | Type::Array(_, _)
				| Type::Structure(_) => Register::RAX,
				Type::Void | Type::Never => return Ok(()),
				Type::Truth => Register::AL,
				Type::Rune => Register::EAX,
			}, span);
		}
		ValueNode::Set(target, index) => {
			self::value(context, scene, prime, types, value, index)?;
			super::push_value(prime, &types[index], span).ok_or_else(||
				context.error(Diagnostic::error().label(value[*index].span.label())
					.message(format!("cannot assign value of type: {}", types[index]))))?;
			super::target(context, scene, prime, types, value, target)?;

			let size = crate::node::size(context, scene.parent
				.clone(), &types[index], Some(span.clone()))?;
			let memory = M::with_base_index(Register::RBP, Register::RAX);
			let alternate = super::alternate(prime, &types[index], span).unwrap();
			super::set(prime, &types[index], size, memory, alternate, span);
		}
		ValueNode::While(condition, index) => {
			let entry = *prime.pending_label.get_or_insert_with(|| scene.label());
			self::value(context, scene, prime, types, value, condition)?;

			let exit = scene.label();
			define_note!(note, prime, span);
			note(I::with_reg_reg(Code::Test_rm8_r8, Register::AL, Register::AL));
			note(I::with_branch(Code::Je_rel32_64, exit));
			self::value(context, scene, prime, types, value, index)?;

			define_note!(note, prime, span);
			note(I::with_branch(Code::Jmp_rel32_64, entry));
			prime.set_pending_label(exit, span);
		}
		ValueNode::When(branches) => {
			let mut complete = false;
			let labels: Result<Vec<_>, _> = branches.iter().map(|(condition, _)| {
				self::value(context, scene, prime, types, value, condition)?;
				complete |= matches!(value[*condition].node, ValueNode::Truth(true));

				let label = scene.label();
				define_note!(note, prime, span);
				note(I::with_reg_reg(Code::Test_rm8_r8, Register::AL, Register::AL));
				note(I::with_branch(Code::Jne_rel32_64, label));
				Ok(label)
			}).collect();

			let exit = scene.label();
			define_note!(note, prime, span);
			if !complete { note(I::with_branch(Code::Jmp_rel32_64, exit)); }
			let iterator = Iterator::zip(labels?.into_iter(), branches.iter());
			for (index, (label, (_, branch))) in iterator.enumerate() {
				prime.set_pending_label(label, span);
				self::value(context, scene, prime, types, value, branch)?;

				if index + 1 != branches.len() {
					define_note!(note, prime, span);
					note(I::with_branch(Code::Jmp_rel32_64, exit));
				}
			}

			prime.set_pending_label(exit, span);
		}
		ValueNode::Cast(index, target) => super::cast(context,
			scene, prime, types, value, index, target, span)?,
		ValueNode::Return(index) => render(context,
			scene, prime, types, value, *index, span)?,
		ValueNode::Compile(_) => unimplemented!(),
		ValueNode::Inline(_) => unimplemented!(),
		ValueNode::Call(path, arguments) => {
			// TODO: provide argument for structure returns
			let size = arguments.iter().rev().try_fold(0, |size, argument| {
				// TODO: copy structures onto stack
				self::value(context, scene, prime, types, value, argument)?;
				super::push_value(prime, &types[argument], span).unwrap();
				Ok(size + crate::node::size(context, scene.parent.clone(),
					&types[argument], Some(span.clone()))?)
			})? as i32;

			let call_index = prime.instructions.len();
			let (path, kind) = (path.node.clone(), types.functions[&index]);
			prime.calls.push((call_index, FunctionPath(path, kind)));

			define_note!(note, prime, span);
			note(I::with_branch(Code::Call_rel32_64, 0));
			note(I::with_reg_i32(Code::Add_rm64_imm32, Register::RSP, size));
			// TODO: set structure return as pointer
		}
		ValueNode::Field(_, _) => unimplemented!(),
		ValueNode::Create(_, _) => unimplemented!(),
		ValueNode::Slice(_, _, _) => unimplemented!(),
		ValueNode::Index(_, _) => unimplemented!(),
		ValueNode::Compound(dual, target, index) => {
			super::binary(context, scene, prime, types, value,
				&Binary::Dual(*dual), target, index, span)?;
			super::push_value(prime, &types[index], span).unwrap_or_else(||
				panic!("cannot assign value of type: {}", &types[index]));
			super::target(context, scene, prime, types, value, target)?;

			let size = crate::node::size(context, scene.parent
				.clone(), &types[index], Some(span.clone()))?;
			let memory = M::with_base_index(Register::RBP, Register::RAX);
			let alternate = super::alternate(prime, &types[index], span).unwrap();
			super::set(prime, &types[index], size, memory, alternate, span);
		}
		ValueNode::Binary(binary, left, right) => super::binary(context,
			scene, prime, types, value, binary, left, right, span)?,
		ValueNode::Unary(_, _) => unimplemented!(),
		ValueNode::Variable(variable) => {
			let offset = scene.variables[variable] as i32;
			let memory = M::with_base_displ(Register::RBP, offset);
			let (code, register) = match types.variables[variable] {
				Type::Void | Type::Never => return Ok(()),
				Type::Truth => (Code::Mov_r8_rm8, Register::AL),
				Type::Rune => (Code::Mov_r32_rm32, Register::EAX),
				Type::Pointer(_) => (Code::Mov_r64_rm64, Register::RAX),
				Type::Signed(size) | Type::Unsigned(size) =>
					(code_rm!(size, Mov_, _r), register!(size, A)),
				Type::Slice(_) | Type::Array(_, _) |
				Type::Structure(_) => (Code::Lea_r64_m, Register::RAX)
			};

			note(I::with_reg_mem(code, register, memory));
		}
		ValueNode::Path(_) => unimplemented!(),
		ValueNode::String(_) => unimplemented!(),
		ValueNode::Register(_) => unimplemented!(),
		ValueNode::Array(_) => unimplemented!(),
		ValueNode::Integral(integral) => match types[index] {
			Type::Signed(size) | Type::Unsigned(size) => {
				let integral = *integral as i64;
				let register = register!(size, A);
				let code = code_rm!(size, Mov_, _im);
				note(I::with_reg_i64(code, register, integral));
			}
			_ => panic!("type is not integral"),
		},
		ValueNode::Truth(truth) =>
			note(I::with_reg_u32(Code::Mov_r8_imm8,
				Register::AL, *truth as u32)),
		ValueNode::Rune(rune) =>
			note(I::with_reg_u32(Code::Mov_r32_imm32,
				Register::EAX, *rune as u32)),
		ValueNode::Break => unimplemented!(),
	})
}

pub fn render(context: &Context, scene: &mut Scene, prime: &mut Translation,
			  types: &Types, value: &Value, index: Option<ValueIndex>,
			  span: &Span) -> crate::Result<()> {
	if let Some(index) = index {
		// TODO: return large value by pointer set
		self::value(context, scene, prime, types, value, &index)?;
	}

	define_note!(note, prime, span);
	note(I::with(Code::Leaveq));
	note(I::with(Code::Retnq));
	Ok(())
}
