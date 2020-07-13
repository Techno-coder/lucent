use std::collections::HashSet;
use std::ops::Index;

use iced_x86::Register::{self, *};

use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::{Function, Identifier, Parameter, Size, ValueNode};
use crate::span::Span;

#[derive(Debug, Copy, Clone)]
pub enum Mode {
	Protected = 32,
	Long = 64,
	Real = 16,
}

impl Mode {
	pub fn size(&self) -> Size {
		match self {
			Mode::Protected => Size::Double,
			Mode::Long => Size::Quad,
			Mode::Real => Size::Word,
		}
	}

	pub fn base(&self) -> Register {
		match self {
			Mode::Protected => Register::EBP,
			Mode::Long => Register::RBP,
			Mode::Real => Register::BP,
		}
	}

	pub fn stack(&self) -> Register {
		match self {
			Mode::Protected => Register::ESP,
			Mode::Long => Register::RSP,
			Mode::Real => Register::SP,
		}
	}

	pub fn destination(&self) -> Register {
		match self {
			Mode::Protected => Register::EDI,
			Mode::Long => Register::RDI,
			Mode::Real => Register::DI,
		}
	}
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Registers([Register; 4]);

impl Index<Size> for Registers {
	type Output = Register;

	fn index(&self, index: Size) -> &Self::Output {
		let Registers(registers) = self;
		match index {
			Size::Byte => &registers[0],
			Size::Word => &registers[1],
			Size::Double => &registers[2],
			Size::Quad => &registers[3],
		}
	}
}

macro_rules! define_registers {
    ($set:ident, $string:ident, $table:ident,
    [$(($($variant:ident: $identifier:expr),*
    $(; $other:ident: $extra:expr)?),)*]) => {
    	const $set: &'static [Registers] =
			&[$(Registers([$($variant,)*]),)*];

    	fn $string(string: &str) -> Option<Register> {
    		Some(match string {
    			$($($identifier => $variant,)* $($extra => $other,)?)*
    			_ => return std::option::Option::None,
    		})
    	}

		#[allow(unreachable_patterns)]
		fn $table(register: Register) -> Option<Registers> {
			Some(match register {
				$($($variant |)* $($other |)?
				None => Registers([$($variant,)*]),)*
    			_ => return std::option::Option::None,
			})
		}
    };
}

define_registers!(SET, string, table, [
	(AL: "al", AX: "ax", EAX: "eax", RAX: "rax"; AH: "ah"),
	(BL: "bl", BX: "bx", EBX: "ebx", RBX: "rbx"; BH: "bh"),
	(CL: "cl", CX: "cx", ECX: "ecx", RCX: "rcx"; CH: "ch"),
	(DL: "dl", DX: "dx", EDX: "edx", RDX: "rdx"; DH: "dh"),
	(SIL: "sil", SI: "si", ESI: "esi", RSI: "rsi"),
	(DIL: "dil", DI: "di", EDI: "edi", RDI: "rdi"),
]);

define_registers!(_STACK_SET, stack_string, stack_table, [
	(BPL: "bpl", BP: "bp", EBP: "ebp", RBP: "rbp"),
	(SPL: "spl", SP: "sp", ESP: "esp", RSP: "rsp"),
]);

define_registers!(LONG_SET, long_string, long_table, [
	(R8L: "r8l", R8W: "r8w", R8D: "r8d", R8: "r8"),
	(R9L: "r9l", R9W: "r9w", R9D: "r9d", R9: "r9"),
	(R10L: "r10l", R10W: "r10w", R10D: "r10d", R10: "r10"),
	(R11L: "r11l", R11W: "r11w", R11D: "r11d", R11: "r11"),
	(R12L: "r12l", R12W: "r12w", R12D: "r12d", R12: "r12"),
	(R13L: "r13l", R13W: "r13w", R13D: "r13d", R13: "r13"),
	(R14L: "r14l", R14W: "r14w", R14D: "r14d", R14: "r14"),
	(R15L: "r15l", R15W: "r15w", R15D: "r15d", R15: "r15"),
]);

pub fn register_set(register: Register) -> Registers {
	table(register).or(stack_table(register)).or(long_table(register))
		.unwrap_or_else(|| panic!("invalid register: {:?}", register))
}

pub fn register(context: &Context, mode: Mode, register: &Identifier,
				span: &Span) -> crate::Result<Register> {
	let Identifier(register) = register;
	let long = matches!(mode, Mode::Long)
		.then_some(long_string(register)).flatten();
	string(register).or(stack_string(register)).or(long)
		.ok_or_else(|| context.error(Diagnostic::error()
			.message("invalid register").label(span.label())))
}

pub fn reserved(context: &Context, function: &Function,
				mode: Mode) -> crate::Result<HashSet<Registers>> {
	let mut reserved = HashSet::new();
	function.parameters.iter().try_for_each(|parameter|
		Ok(if let Parameter::Register(register) = &parameter.node {
			self::register(context, mode, &register.node, &register.span)
				.map(|register| reserved.insert(register_set(register)))?;
		}))?;
	function.value.values.iter().try_for_each(|value|
		Ok(if let ValueNode::Register(register) = &value.node {
			self::register(context, mode, register, &value.span)
				.map(|register| reserved.insert(register_set(register)))?;
		}))?;
	Ok(reserved)
}

pub fn registers(context: &Context, reserved: &HashSet<Registers>, mode: Mode,
				 span: &Span) -> crate::Result<(Registers, Registers)> {
	let long = matches!(mode, Mode::Long).then_some(LONG_SET.iter());
	let mut iterator = Iterator::chain(SET.iter(), long.into_iter().flatten())
		.filter(|registers| !reserved.contains(registers)).cloned();
	let (primary, alternate) = (iterator.next(), iterator.next());
	Iterator::zip(primary.into_iter(), alternate.into_iter())
		.next().ok_or_else(|| context.error(Diagnostic::error()
		.message("unable to allocate registers for function")
		.note("two free registers required")
		.label(span.label())))
}
