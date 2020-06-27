use std::io::{self, Cursor};

use goblin::mach::{constants, load_command};
use scroll::{IOwrite, Pwrite, SizeWith};

use super::{BinarySegment, PAGE_SIZE};

#[repr(C)]
#[derive(Debug, Pwrite, IOwrite, SizeWith)]
pub struct UnixThreadCommand {
	command: u32,
	command_size: u32,
	flavor: u32,
	count: u32,
	thread_state: [u64; 21],
}

impl UnixThreadCommand {
	const STATE_COUNT: usize = 21;

	pub fn new(instruction_address: u64) -> Self {
		use std::mem::size_of;
		const X86_THREAD_STATE64: u32 = 4;
		const STATE_SIZE: usize = size_of::<[u64; UnixThreadCommand::STATE_COUNT]>();
		const STATE_COUNT: usize = STATE_SIZE / size_of::<u32>();

		const INSTRUCTION_REGISTER: usize = 16;
		let mut thread_state = [0; Self::STATE_COUNT];
		thread_state[INSTRUCTION_REGISTER] = instruction_address;

		UnixThreadCommand {
			command: load_command::LC_UNIXTHREAD,
			command_size: size_of::<Self>() as u32,
			flavor: X86_THREAD_STATE64,
			count: STATE_COUNT as u32,
			thread_state,
		}
	}
}

#[derive(Debug)]
pub struct LoadCommands {
	pub zero: load_command::SegmentCommand64,
	pub header: load_command::SegmentCommand64,
	pub segments: Vec<load_command::SegmentCommand64>,
	pub thread: UnixThreadCommand,
}

impl LoadCommands {
	pub fn new(segments: Vec<load_command::SegmentCommand64>, thread: UnixThreadCommand) -> Self {
		let zero = BinarySegment::default().name(b"__PAGEZERO").size(PAGE_SIZE as u64).build();
		let header = BinarySegment::default().name(b"__TEXT").address(PAGE_SIZE as u64)
			.protections(constants::VM_PROT_EXECUTE | constants::VM_PROT_READ).build();
		LoadCommands { zero, header, segments, thread }
	}

	pub fn size_count(&self) -> (u32, usize) {
		std::iter::once(self.zero.cmdsize)
			.chain(std::iter::once(self.header.cmdsize))
			.chain(std::iter::once(self.thread.command_size))
			.chain(self.segments.iter().map(|command| command.cmdsize))
			.fold((0, 0), |(size, count), command_size| (size + command_size, count + 1))
	}

	pub fn write(self, target: &mut Cursor<Vec<u8>>) -> io::Result<()> {
		Iterator::chain(std::array::IntoIter::new([self.zero, self.header]),
			self.segments).try_for_each(|segment| target.iowrite(segment))?;
		target.iowrite(self.thread)
	}
}
