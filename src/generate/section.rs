use crate::node::{FunctionPath, Path, Size};

pub type Offset = usize;

#[derive(Debug, Default, Clone)]
pub struct Section {
	pub bytes: Vec<u8>,
	pub relative: Vec<Relative>,
	pub intrinsics: Vec<(Offset, Size, Intrinsic)>,
}

#[derive(Debug, Clone)]
pub struct Relative {
	pub size: Size,
	pub offset: Offset,
	pub target: Offset,
	pub path: FunctionPath,
}

#[derive(Debug, Clone)]
pub enum Intrinsic {
	Size(Path),
	Start(Path),
	End(Path),
}
