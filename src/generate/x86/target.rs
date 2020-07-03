use iced_x86::{Code, Instruction as I, Register};

use crate::context::Context;
use crate::inference::Types;
use crate::node::{Value, ValueIndex, ValueNode};

use super::{Scene, Translation};

pub fn target(_context: &Context, scene: &mut Scene, prime: &mut Translation,
			  _types: &Types, value: &Value, index: &ValueIndex) -> crate::Result<()> {
	let span = &value[*index].span;
	define_note!(note, prime, span);
	Ok(match &value[*index].node {
		ValueNode::Block(_) => unimplemented!(),
		ValueNode::Let(_, _, _) => unimplemented!(),
		ValueNode::Set(_, _) => unimplemented!(),
		ValueNode::While(_, _) => unimplemented!(),
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
		ValueNode::Compound(_, _, _) => unimplemented!(),
		ValueNode::Binary(_, _, _) => unimplemented!(),
		ValueNode::Unary(_, _) => unimplemented!(),
		ValueNode::Variable(variable) => {
			let offset = scene.variables[variable] as i64;
			note(I::with_reg_i64(Code::Mov_r64_imm64, Register::RAX, offset));
		}
		ValueNode::Path(_) => unimplemented!(),
		ValueNode::String(_) => unimplemented!(),
		ValueNode::Register(_) => unimplemented!(),
		ValueNode::Array(_) => unimplemented!(),
		ValueNode::Integral(_) => unimplemented!(),
		ValueNode::Truth(_) => unimplemented!(),
		ValueNode::Rune(_) => unimplemented!(),
		ValueNode::Break => unimplemented!(),
	})
}
