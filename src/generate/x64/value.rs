use iced_x86::{Code, Register};
use iced_x86::Instruction as I;
use iced_x86::MemoryOperand as M;

use crate::context::Context;
use crate::error::Diagnostic;
use crate::inference::Types;
use crate::node::{Binary, Size, Type, Value, ValueIndex, ValueNode};
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
			set(prime, &types[index], size, memory, match types[index] {
				Type::Truth => Register::AL,
				Type::Rune => Register::EAX,
				Type::Signed(size) | Type::Unsigned(size) => match size {
					Size::Byte => Register::AL,
					Size::Word => Register::AX,
					Size::Double => Register::EAX,
					Size::Quad => Register::RAX,
				}
				Type::Pointer(_) | Type::Slice(_) | Type::Array(_, _)
				| Type::Structure(_) => Register::RAX,
				Type::Void | Type::Never => return Ok(()),
			}, span);
		}
		ValueNode::Set(target, index) => {
			self::value(context, scene, prime, types, value, index)?;
			push_value(prime, &types[index], span).ok_or_else(|| context
				.error(Diagnostic::error().label(value[*index].span.label())
					.message(format!("cannot assign value of type: {}", types[index]))))?;
			super::target(context, scene, prime, types, value, target)?;

			let size = crate::node::size(context, scene.parent
				.clone(), &types[index], Some(span.clone()))?;
			let memory = M::with_base_index(Register::RBP, Register::RAX);
			let alternate = alternate(prime, &types[index], span).unwrap();
			set(prime, &types[index], size, memory, alternate, span);
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
			prime.pending_label = Some(exit);
		}
		ValueNode::When(_) => unimplemented!(),
		ValueNode::Cast(_, _) => unimplemented!(),
		ValueNode::Return(_) => unimplemented!(),
		ValueNode::Compile(_) => unimplemented!(),
		ValueNode::Inline(_) => unimplemented!(),
		ValueNode::Call(_, _) => unimplemented!(),
		ValueNode::Field(_, _) => unimplemented!(),
		ValueNode::Create(_, _) => unimplemented!(),
		ValueNode::Slice(_, _, _) => unimplemented!(),
		ValueNode::Index(_, _) => unimplemented!(),
		ValueNode::Compound(dual, target, index) => {
			super::binary(context, scene, prime, types, value,
				&Binary::Dual(*dual), target, index, span)?;
			push_value(prime, &types[index], span).unwrap_or_else(||
				panic!("cannot assign value of type: {}", &types[index]));
			super::target(context, scene, prime, types, value, target)?;

			let size = crate::node::size(context, scene.parent
				.clone(), &types[index], Some(span.clone()))?;
			let memory = M::with_base_index(Register::RBP, Register::RAX);
			let alternate = alternate(prime, &types[index], span).unwrap();
			set(prime, &types[index], size, memory, alternate, span);
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
				Type::Signed(size) | Type::Unsigned(size) => match size {
					Size::Byte => (Code::Mov_r8_rm8, Register::AL),
					Size::Word => (Code::Mov_r16_rm16, Register::AX),
					Size::Double => (Code::Mov_r32_rm32, Register::EAX),
					Size::Quad => (Code::Mov_r64_rm64, Register::RAX),
				}
				Type::Slice(_) | Type::Array(_, _) |
				Type::Structure(_) => (Code::Lea_r64_m, Register::RAX)
			};

			note(I::with_reg_mem(code, register, memory));
		}
		ValueNode::Path(_) => unimplemented!(),
		ValueNode::String(_) => unimplemented!(),
		ValueNode::Register(_) => unimplemented!(),
		ValueNode::Array(_) => unimplemented!(),
		ValueNode::Integral(integral) => {
			let size = match types[index] {
				Type::Signed(size) => size,
				Type::Unsigned(size) => size,
				_ => panic!("type is not integral"),
			};

			let (code, register) = match size {
				Size::Byte => (Code::Mov_r8_imm8, Register::AL),
				Size::Word => (Code::Mov_r16_imm16, Register::AX),
				Size::Double => (Code::Mov_r32_imm32, Register::EAX),
				Size::Quad => (Code::Mov_r64_imm64, Register::RAX),
			};

			let integral = *integral as i64;
			note(I::with_reg_i64(code, register, integral));
		}
		ValueNode::Truth(truth) =>
			note(I::with_reg_u32(Code::Mov_r8_imm8, Register::AL, *truth as u32)),
		ValueNode::Rune(rune) =>
			note(I::with_reg_u32(Code::Mov_r32_imm32, Register::EAX, *rune as u32)),
		ValueNode::Break => unimplemented!(),
	})
}

pub fn set(prime: &mut Translation, path: &Type, _size: usize,
		   memory: M, register: Register, span: &Span) {
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

	define_note!(note, prime, span);
	note(I::with_mem_reg(code, memory, register));
}

pub fn push_value(prime: &mut Translation, path: &Type, span: &Span) -> Option<Register> {
	let (code, register) = match path {
		Type::Void | Type::Never => return None,
		Type::Truth => (Code::Push_r16, Register::AX),
		Type::Rune => (Code::Push_r32, Register::EAX),
		Type::Array(_, _) | Type::Slice(_) | Type::Structure(_)
		| Type::Pointer(_) => (Code::Push_r64, Register::RAX),
		Type::Signed(size) | Type::Unsigned(size) => match size {
			Size::Byte => (Code::Push_r16, Register::AX),
			Size::Word => (Code::Push_r16, Register::AX),
			Size::Double => (Code::Push_r32, Register::EAX),
			Size::Quad => (Code::Push_r64, Register::RAX),
		}
	};

	define_note!(note, prime, span);
	note(I::with_reg(code, register));
	Some(register)
}

pub fn alternate(prime: &mut Translation, path: &Type, span: &Span) -> Option<Register> {
	let (code, alternate) = match path {
		Type::Void | Type::Never => return None,
		Type::Truth => (Code::Pop_r16, Register::BX),
		Type::Rune => (Code::Pop_r32, Register::EBX),
		Type::Array(_, _) | Type::Slice(_) | Type::Structure(_)
		| Type::Pointer(_) => (Code::Pop_r64, Register::RBX),
		Type::Signed(size) | Type::Unsigned(size) => match size {
			Size::Byte => (Code::Pop_r16, Register::BX),
			Size::Word => (Code::Pop_r16, Register::BX),
			Size::Double => (Code::Pop_r32, Register::EBX),
			Size::Quad => (Code::Pop_r64, Register::RBX),
		}
	};

	define_note!(note, prime, span);
	note(I::with_reg(code, alternate));
	Some(alternate)
}
