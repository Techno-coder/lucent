use iced_x86::{Code, Register};
use iced_x86::Instruction as I;

use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::{Size, Type};
use crate::span::Span;

use super::{Scene, Translation};

macro_rules! define_note {
    ($note:ident, $prime:expr, $span:expr) => {
		let $note = &mut |instruction| $prime.push(instruction, $span);
    };
}

macro_rules! register {
    ($size:expr, $index:ident) => {{
    	use iced_x86::Register::*;
    	match $size {
    		Size::Byte => concat_idents!($index, L),
    		Size::Word => concat_idents!($index, X),
    		Size::Double => concat_idents!(E, $index, X),
    		Size::Quad => concat_idents!(R, $index, X),
    	}
    }};
}

macro_rules! relative {
    ($mode:expr, $prefix:ident) => {{
    	use iced_x86::Code::*;
    	use super::Mode;
    	match $mode {
			Mode::Protected => concat_idents!($prefix, _rel32_64),
			Mode::Long => concat_idents!($prefix, _rel32_64),
			Mode::Real => concat_idents!($prefix, _rel32_32),
    	}
    }};
}

macro_rules! code_m {
    ($size:expr, $identifier:ident $(,$other:ident)*) => {{
    	use iced_x86::Code::*;
    	match $size {
    		Size::Byte => concat_idents!($identifier, m8, $($other,)*),
    		Size::Word => concat_idents!($identifier, m16, $($other,)*),
    		Size::Double => concat_idents!($identifier, m32, $($other,)*),
    		Size::Quad => concat_idents!($identifier, m64, $($other,)*),
    	}
    }};
}

macro_rules! code_rm {
    ($size:expr, $left:ident, $right:ident) => {{
    	use iced_x86::Code::*;
    	match $size {
    		Size::Byte => concat_idents!($left, r8, $right, m8),
    		Size::Word => concat_idents!($left, r16, $right, m16),
    		Size::Double => concat_idents!($left, r32, $right, m32),
    		Size::Quad => concat_idents!($left, r64, $right, m64),
    	}
    }};
}

pub fn code_push(size: Size) -> Code {
	match size {
		Size::Byte => Code::Push_r16,
		Size::Word => Code::Push_r16,
		Size::Double => Code::Push_r32,
		Size::Quad => Code::Push_r64,
	}
}

pub fn code_pop(size: Size) -> Code {
	match size {
		Size::Byte => Code::Pop_r16,
		Size::Word => Code::Pop_r16,
		Size::Double => Code::Pop_r32,
		Size::Quad => Code::Pop_r64,
	}
}

pub fn code_sign_extend(size: Size) -> Code {
	match size {
		Size::Byte => Code::Cbw,
		Size::Word => Code::Cwd,
		Size::Double => Code::Cdq,
		Size::Quad => Code::Cqo,
	}
}

pub fn transfer(prime: &mut Translation, register: Register,
				target: Register, size: Size, span: &Span) {
	if register != target {
		define_note!(note, prime, span);
		let code = code_rm!(size, Mov_, _r);
		note(I::with_reg_reg(code, target, register));
	}
}

pub fn reserve<F>(scene: &mut Scene, prime: &mut Translation,
				  target: Register, function: F, size: Size, span: &Span)
	where F: FnOnce(&mut Scene, &mut Translation) {
	let registers = super::register_set(target);
	let free = !scene.reserved.contains(&registers);

	define_note!(note, prime, span);
	if !free { note(I::with_reg(code_push(size), target)); }
	if free { scene.reserved.insert(registers.clone()); }
	function(scene, prime);

	define_note!(note, prime, span);
	if free { scene.reserved.remove(&registers); }
	if !free { note(I::with_reg(code_pop(size), target)); }
}

pub fn convey<F>(scene: &mut Scene, prime: &mut Translation, register: Register,
				 default: Register, function: F, size: Size, span: &Span)
	where F: FnOnce(&Scene, &mut Translation, Register) {
	let registers = super::register_set(register);
	match scene.reserved.contains(&registers) {
		false => function(scene, prime, register),
		true => reserve(scene, prime, default, |scene, prime| {
			transfer(prime, register, default, size, span);
			function(scene, prime, default)
		}, size, span),
	}
}

pub fn size(context: &Context, scene: &Scene, path: &Type,
			span: &Span) -> crate::Result<Size> {
	let size = match path {
		Type::Truth => Size::Byte,
		Type::Rune => Size::Double,
		Type::Signed(size) | Type::Unsigned(size) => *size,
		Type::Void | Type::Never => return context.pass(Diagnostic::error()
			.message(format!("cannot lower values of type: {}", path))
			.label(span.label())),
		Type::Structure(_) | Type::Array(_, _) | Type::Slice(_)
		| Type::Pointer(_) => scene.mode.size(),
	};

	let mode = scene.mode as u8;
	match size as u8 > mode {
		true => context.pass(Diagnostic::error()
			.label(span.label().with_message(path.to_string()))
			.message(format!("unsupported type for architecture: x{}", mode))),
		false => Ok(size),
	}
}

pub fn stack(size: Size) -> Size {
	match size {
		Size::Byte => Size::Word,
		other => other,
	}
}
