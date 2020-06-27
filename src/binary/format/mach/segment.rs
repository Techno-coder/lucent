use goblin::mach::load_command;

#[derive(Debug, Default)]
pub struct BinarySegment {
	name: [u8; 16],
	address: u64,
	size: u64,
	offset: u64,
	file_size: u64,
	protections: u32,
}

impl BinarySegment {
	pub fn name(mut self, name: &[u8]) -> Self {
		self.name[..name.len()].copy_from_slice(name);
		self
	}

	pub fn address(mut self, address: u64) -> Self {
		self.address = address;
		self
	}

	pub fn size(mut self, size: u64) -> Self {
		self.size = size;
		self
	}

	pub fn file_size(mut self, file_size: u64) -> Self {
		self.file_size = file_size;
		self
	}

	pub fn protections(mut self, protection: u32) -> Self {
		self.protections |= protection;
		self
	}

	pub fn build(self) -> load_command::SegmentCommand64 {
		load_command::SegmentCommand64 {
			cmd: load_command::LC_SEGMENT_64,
			cmdsize: load_command::SIZEOF_SEGMENT_COMMAND_64 as u32,
			segname: self.name,
			vmaddr: self.address,
			vmsize: self.size,
			fileoff: self.offset,
			filesize: self.file_size,
			maxprot: self.protections,
			initprot: self.protections,
			nsects: 0,
			flags: 0,
		}
	}
}
