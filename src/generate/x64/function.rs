use iced_x86::{Code, Register};
use iced_x86::Instruction as I;
use iced_x86::MemoryOperand as M;

use crate::context::Context;
use crate::node::{Function, Parameter, Size, Type};
use crate::span::Span;

use super::{Scene, Translation};

pub fn entry(prime: &mut Translation, internal: &Span) {
	define_note!(note, prime, internal);
	note(I::with_reg(Code::Push_r64, Register::RBP));
	note(I::with_reg_reg(Code::Mov_r64_rm64, Register::RBP, Register::RSP));
	note(I::with_reg_i32(Code::Sub_rm64_imm32, Register::RSP, 0));
}

pub fn parameters(context: &Context, function: &Function,
				  scene: &mut Scene) -> crate::Result<()> {
	let mut offset = 16;
	Ok(for parameter in &function.parameters {
		if let Parameter::Variable(variable, path) = &parameter.node {
			scene.variables.insert(variable.node.clone(), offset as isize);
			offset += crate::node::size(context, scene.parent
				.clone(), &path.node, Some(variable.span.clone()))?;
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

pub fn push_value(prime: &mut Translation, path: &Type, span: &Span) -> Option<Register> {
	let (code, stack, register) = match path {
		Type::Void | Type::Never => return None,
		Type::Truth => (Code::Push_r16, Register::AX, Register::AL),
		Type::Rune => (Code::Push_r32, Register::EAX, Register::EAX),
		Type::Array(_, _) | Type::Slice(_) | Type::Structure(_)
		| Type::Pointer(_) => (Code::Push_r64, Register::RAX, Register::RAX),
		Type::Signed(size) | Type::Unsigned(size) => match size {
			Size::Byte => (Code::Push_r16, Register::AX, Register::AL),
			Size::Word => (Code::Push_r16, Register::AX, Register::AX),
			Size::Double => (Code::Push_r32, Register::EAX, Register::EAX),
			Size::Quad => (Code::Push_r64, Register::RAX, Register::RAX),
		}
	};

	define_note!(note, prime, span);
	note(I::with_reg(code, stack));
	Some(register)
}

pub fn alternate(prime: &mut Translation, path: &Type, span: &Span) -> Option<Register> {
	let (code, stack, alternate) = match path {
		Type::Void | Type::Never => return None,
		Type::Truth => (Code::Pop_r16, Register::BX, Register::BL),
		Type::Rune => (Code::Pop_r32, Register::EBX, Register::EBX),
		Type::Array(_, _) | Type::Slice(_) | Type::Structure(_)
		| Type::Pointer(_) => (Code::Pop_r64, Register::RBX, Register::RBX),
		Type::Signed(size) | Type::Unsigned(size) => match size {
			Size::Byte => (Code::Pop_r16, Register::BX, Register::BL),
			Size::Word => (Code::Pop_r16, Register::BX, Register::BX),
			Size::Double => (Code::Pop_r32, Register::EBX, Register::EBX),
			Size::Quad => (Code::Pop_r64, Register::RBX, Register::RBX),
		}
	};

	define_note!(note, prime, span);
	note(I::with_reg(code, stack));
	Some(alternate)
}
