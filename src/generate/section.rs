use crate::node::Path;

#[derive(Debug, Default)]
pub struct Section {
	pub bytes: Vec<u8>,
	pub intrinsics: Vec<(usize, Intrinsic)>,
}

#[derive(Debug)]
pub enum Intrinsic {
	Size(Path),
	Start(Path),
	End(Path),
}
