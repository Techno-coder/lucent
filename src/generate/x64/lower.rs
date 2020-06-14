use std::collections::HashMap;
use std::sync::Arc;

use iced_x86::{BlockEncoder, BlockEncoderOptions, Instruction, InstructionBlock};

use crate::context::Context;
use crate::generate::Section;
use crate::node::{FunctionKind, Parameter, Path, Variable};
use crate::query::{Key, QueryError};
use crate::span::Span;

macro_rules! define_note {
    ($note:ident, $prime:expr, $span:expr) => {
		let $note = &mut |instruction| $prime.push(instruction, $span);
    };
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

pub fn translate(context: &Context, parent: Option<Key>, path: &Path,
				 kind: FunctionKind, span: Option<Span>) -> crate::Result<Translation> {
	let functions = context.functions.get(&path);
	let function = functions.as_ref().and_then(|table|
		table.get(kind)).ok_or(QueryError::Failure)?;
	let types = crate::inference::type_function(context,
		parent.clone(), path, kind, span)?;

	let mut translation = Translation::default();
	let scene = &mut Scene { parent, ..Scene::default() };

	// TODO: parameters
	for parameter in &function.parameters {
		match &parameter.node {
			Parameter::Variable(variable, _) => {
				scene.variable(variable.node.clone(), 8);
			}
			_ => (),
		}
	}

	super::value(context, scene, &mut translation, &types,
		&function.value, &function.value.root)?;

	// TODO: return value
	Ok(translation)
}

pub fn lower(context: &Context, parent: Option<Key>, path: &Path,
			 kind: FunctionKind, span: Option<Span>) -> crate::Result<Arc<Section>> {
	let key = Key::Generate(path.clone(), kind);
	context.sections.scope(parent, key.clone(), span.clone(), || {
		let parent = Some(key.clone());
		let translation = translate(context, parent, path, kind, span)?;

		// TODO: remove display code
		use iced_x86::Formatter;
		let buffer = &mut String::new();
		let mut formatter = iced_x86::NasmFormatter::new();
		for instruction in &translation.instructions {
			formatter.format(instruction, buffer);
			println!("{}", buffer);
			buffer.clear();
		}

		// TODO: set instruction pointer
		let block = InstructionBlock::new(&translation.instructions, 0);
		let block = BlockEncoder::encode(64, block, BlockEncoderOptions::NONE).unwrap();

		let mut section = Section::default();
		section.bytes = block.code_buffer;
		Ok(section)
	})
}
