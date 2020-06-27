use crate::node::{FunctionPath, Path, Size};

pub type Offset = usize;

#[derive(Debug, Default)]
pub struct Section {
	pub bytes: Vec<u8>,
	pub relative: Vec<(Offset, Size, FunctionPath)>,
	pub intrinsics: Vec<(Offset, Intrinsic)>,
}

#[derive(Debug)]
pub enum Intrinsic {
	Size(Path),
	Start(Path),
	End(Path),
}
