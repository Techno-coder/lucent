use iced_x86::{Instruction, Register};

use crate::query::S;

pub type Label = usize;
pub type Entry = usize;
pub type Exit = usize;

#[derive(Debug)]
pub struct Scene {
	primary: Option<Register>,
	alternate: Option<Register>,
	instructions: Vec<S<Instruction>>,
	labels: Vec<(Entry, Exit)>,
}
