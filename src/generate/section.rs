use crate::node::{FunctionPath, Size};

pub type Offset = usize;

#[derive(Debug, Default, Clone)]
pub struct Section {
	pub bytes: Vec<u8>,
	// TODO: replace with compile time execution nodes
	pub relative: Vec<Relative>,
}

#[derive(Debug, Clone)]
pub struct Relative {
	pub size: Size,
	pub offset: Offset,
	pub target: Offset,
	pub path: FunctionPath,
}
