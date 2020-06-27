use std::collections::HashMap;
use std::sync::Arc;

use iced_x86::{BlockEncoder, BlockEncoderOptions, Instruction, InstructionBlock};

use crate::context::Context;
use crate::generate::Section;
use crate::node::{FunctionPath, Size, Variable};
use crate::query::{Key, QueryError};
use crate::span::Span;

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

macro_rules! code_m {
    ($size:expr, $identifier:ident) => {{
    	use iced_x86::Code::*;
    	match $size {
    		Size::Byte => concat_idents!($identifier, m8),
    		Size::Word => concat_idents!($identifier, m16),
    		Size::Double => concat_idents!($identifier, m32),
    		Size::Quad => concat_idents!($identifier, m64),
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

#[derive(Debug, Default)]
pub struct Scene {
	pub parent: Option<Key>,
	pub variables: HashMap<Variable, isize>,
	next_offset: isize,
	next_label: u64,
}

impl Scene {
	pub fn variable(&mut self, variable: Variable, size: usize) -> isize {
		self.next_offset -= size as isize;
		self.variables.insert(variable, self.next_offset).unwrap_none();
		self.next_offset
	}

	pub fn label(&mut self) -> u64 {
		self.next_label += 1;
		self.next_label - 1
	}
}

#[derive(Debug, Default)]
pub struct Translation {
	pub pending_label: Option<u64>,
	pub instructions: Vec<Instruction>,
	pub calls: Vec<(usize, FunctionPath)>,
	pub spans: Vec<Span>,
}

impl Translation {
	pub fn push(&mut self, mut instruction: Instruction, span: &Span) {
		self.pending_label.take().into_iter()
			.for_each(|label| instruction.set_ip(label));
		self.instructions.push(instruction);
		self.spans.push(span.clone());
	}
}

pub fn lower(context: &Context, parent: Option<Key>, path: &FunctionPath,
			 span: Option<Span>) -> crate::Result<Arc<Section>> {
	let key = Key::Generate(path.clone());
	context.sections.scope(parent, key.clone(), span.clone(), || {
		let parent = Some(key.clone());
		let translation = translate(context, parent, path, span)?;
		let block = InstructionBlock::new(&translation.instructions, 0);

		let mut encoder = BlockEncoderOptions::RETURN_CONSTANT_OFFSETS;
		encoder |= BlockEncoderOptions::RETURN_NEW_INSTRUCTION_OFFSETS;
		let block = BlockEncoder::encode(64, block, encoder).unwrap_or_else(|error|
			panic!("encoding failure in: {:?}, where: {}", path, error));

		let mut section = Section::default();
		for (index, path) in translation.calls {
			let offset = block.new_instruction_offsets[index] as usize;
			let offset = offset + block.constant_offsets[index].immediate_offset();
			section.relative.push((offset, Size::Double, path));
		}

		section.bytes = block.code_buffer;
		Ok(section)
	})
}

pub fn translate(context: &Context, parent: Option<Key>, path: &FunctionPath,
				 span: Option<Span>) -> crate::Result<Translation> {
	let FunctionPath(function, kind) = path;
	let functions = context.functions.get(&function);
	let function = functions.as_ref().and_then(|table|
		table.get(*kind)).ok_or(QueryError::Failure)?;
	let types = crate::inference::type_function(context,
		parent.clone(), path, span)?;

	let mut translation = Translation::default();
	let scene = &mut Scene { parent, ..Scene::default() };
	let internal = context.files.read().internal.clone();
	super::entry(&mut translation, &internal);
	super::parameters(context, function, scene)?;

	super::value(context, scene, &mut translation, &types,
		&function.value, &function.value.root)?;
	// TODO: return value

	// TODO: Is the exit redundant?
	super::exit(&mut translation, &internal);
	let frame_size = -scene.next_offset as i32;
	translation.instructions[2].set_immediate_i32(1, frame_size);
	Ok(translation)
}
