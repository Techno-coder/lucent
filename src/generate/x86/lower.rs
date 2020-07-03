use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use iced_x86::{BlockEncoder, BlockEncoderOptions, Instruction, InstructionBlock};

use crate::context::Context;
use crate::generate::{Relative, Section};
use crate::node::{FunctionPath, Size, Variable};
use crate::query::{Key, QueryError};
use crate::span::Span;

use super::{Mode, Registers};

#[derive(Debug)]
pub struct Scene {
	pub mode: Mode,
	pub primary: Registers,
	pub alternate: Registers,
	pub reserved: HashSet<Registers>,
	pub variables: HashMap<Variable, isize>,
	pub parent: Option<Key>,
	next_offset: isize,
	next_label: u64,
}

impl Scene {
	pub fn mode_primary(&self) -> iced_x86::Register {
		self.primary[self.mode.size()]
	}

	pub fn variable(&mut self, variable: Variable, size: usize) -> isize {
		self.next_offset -= size as isize;
		self.variables.insert(variable, self.next_offset).unwrap_none();
		self.next_offset
	}

	pub fn label(&mut self) -> u64 {
		self.next_label += 1;
		self.next_label
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
	pub fn set_pending_label(&mut self, label: u64, span: &Span) {
		if self.pending_label.is_some() {
			let code = iced_x86::Code::Nopw;
			self.push(Instruction::with(code), span);
		}

		assert!(self.pending_label.is_none());
		self.pending_label = Some(label);
	}

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
		let translation = translate(context, parent, path, Mode::Long, span)?;
		let block = InstructionBlock::new(&translation.instructions, 0);

		// TODO: remove display code
		use iced_x86::Formatter;
		let buffer = &mut String::new();
		let mut formatter = iced_x86::NasmFormatter::new();
		for instruction in &translation.instructions {
			formatter.format(instruction, buffer);
			println!("{}", buffer);
			buffer.clear();
		}
		println!();

		let mut encoder = BlockEncoderOptions::RETURN_CONSTANT_OFFSETS;
		encoder |= BlockEncoderOptions::RETURN_NEW_INSTRUCTION_OFFSETS;
		let block = BlockEncoder::encode(64, block, encoder).unwrap_or_else(|error|
			panic!("encoding failure in: {:?}, where: {}", path, error));

		let mut section = Section::default();
		for (index, path) in translation.calls {
			let offset = block.new_instruction_offsets[index] as usize;
			let offset = offset + block.constant_offsets[index].immediate_offset();
			let target = block.new_instruction_offsets.get(index + 1).cloned()
				.unwrap_or(block.code_buffer.len() as u32) as usize;

			let size = Size::Double;
			let relative = Relative { size, offset, path, target };
			section.relative.push(relative);
		}

		section.bytes = block.code_buffer;
		Ok(section)
	})
}

pub fn translate(context: &Context, parent: Option<Key>, path: &FunctionPath,
				 mode: Mode, span: Option<Span>) -> crate::Result<Translation> {
	let FunctionPath(function, kind) = path;
	let functions = context.functions.get(&function);
	let function = functions.as_ref().and_then(|table|
		table.get(*kind)).ok_or(QueryError::Failure)?;
	let types = crate::inference::type_function(context,
		parent.clone(), path, span)?;

	let reserved = super::reserved(context, function, mode)?;
	let (primary, alternate) = super::registers(context,
		&reserved, mode, &function.identifier.span)?;

	let (next_offset, next_label) = (0, 0);
	let variables = HashMap::new();
	let scene = &mut Scene {
		mode,
		primary,
		alternate,
		reserved,
		variables,
		parent,
		next_offset,
		next_label,
	};

	let mut translation = Translation::default();
	super::entry(scene, &mut translation, &function.identifier.span);
	super::parameters(context, function, scene)?;

	// TODO: remove special main
	if function.identifier.node == crate::node::Identifier("main".to_string()) {
		super::value(context, scene, &mut translation,
			&types, &function.value, &function.value.root)?;
		define_note!(note, translation, &function.identifier.span);
		note(Instruction::with_reg_reg(iced_x86::Code::Mov_r64_rm64,
			iced_x86::Register::RDI, iced_x86::Register::RAX));
		note(Instruction::with_reg_i64(iced_x86::Code::Mov_r64_imm64,
			iced_x86::Register::RAX, (2 << 24) | (!(0xff << 24) & 1)));
		note(Instruction::with(iced_x86::Code::Syscall));
		let frame_size = -scene.next_offset as i32;
		translation.instructions[2].set_immediate_i32(1, frame_size);
		return Ok(translation);
	}

	let root = function.value.root;
	super::render(context, scene, &mut translation, &types,
		&function.value, Some(root), &function.value[root].span)?;

	let frame_size = -scene.next_offset as i32;
	if frame_size != 0 {
		translation.instructions[2].set_immediate_i32(1, frame_size);
	} else {
		translation.instructions.remove(2);
		translation.calls.iter_mut().for_each(|(index, _)| *index -= 1);
	}

	Ok(translation)
}
