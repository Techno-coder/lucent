use std::io::{self, Cursor, Write};

use goblin::mach::{constants, header, load_command};
use scroll::IOwrite;

use crate::binary::{Segment, SegmentKind};
use crate::other::ceiling;

use super::{BinarySegment, LoadCommands, UnixThreadCommand};

// TODO: derive from architecture
pub const PAGE_SIZE: usize = 4096;

pub fn compile(segments: Vec<Segment>) -> io::Result<Vec<u8>> {
	let entry = 1024 * 1024; // TODO: derive from annotation
	let thread = UnixThreadCommand::new(entry);
	let binary_segments = self::segments(&segments);
	let mut commands = LoadCommands::new(binary_segments, thread);
	let (size, command_count) = commands.size_count();

	let mut bytes = Vec::new();
	let mut offset = header::SIZEOF_HEADER_64 + size as usize;
	commands.header.filesize = offset as u64;
	commands.header.vmsize = offset as u64;

	let command_segments = commands.segments.iter_mut();
	let iterator = Iterator::zip(segments.into_iter(), command_segments);
	for (segment, binary_segment) in iterator {
		fill_page(&mut offset, &mut bytes);
		binary_segment.fileoff = offset as u64;

		match segment.kind {
			SegmentKind::Text(data) | SegmentKind::Data(data) => data
				.into_iter().try_for_each(|data| bytes.write(&data)
				.map(|bytes| offset += bytes))?,
			SegmentKind::Reserve(_) => (),
		}
	}

	fill_page(&mut offset, &mut bytes);
	let mut target = Cursor::new(Vec::new());
	target.iowrite(header::Header {
		magic: header::MH_MAGIC_64,
		cputype: constants::cputype::CPU_TYPE_X86_64,
		cpusubtype: constants::cputype::CPU_SUBTYPE_X86_64_ALL,
		filetype: header::MH_EXECUTE,
		ncmds: command_count,
		sizeofcmds: size,
		flags: header::MH_NOUNDEFS,
		reserved: 0,
	})?;

	commands.write(&mut target)?;
	target.write_all(&bytes)?;
	Ok(target.into_inner())
}

fn fill_page(offset: &mut usize, bytes: &mut Vec<u8>) {
	let padding = ceiling(*offset, PAGE_SIZE) - *offset;
	bytes.resize(bytes.len() + padding, 0);
	*offset += padding;
}

fn segments(segments: &[Segment]) -> Vec<load_command::SegmentCommand64> {
	segments.iter().map(|segment| match &segment.kind {
		SegmentKind::Text(data) => {
			let size = data.iter().map(Vec::len).sum::<usize>() as u64;
			BinarySegment::default().name(b"__TEXT")
				.address(segment.address as u64).size(size).file_size(size)
				.protections(constants::VM_PROT_EXECUTE | constants::VM_PROT_READ)
		}
		SegmentKind::Data(data) => {
			let size = data.iter().map(Vec::len).sum::<usize>() as u64;
			BinarySegment::default().name(b"__DATA")
				.address(segment.address as u64).size(size).file_size(size)
				.protections(constants::VM_PROT_READ | constants::VM_PROT_WRITE)
		}
		SegmentKind::Reserve(size) => BinarySegment::default().name(b"__DATA")
			.address(segment.address as u64).size(*size as u64).file_size(0)
			.protections(constants::VM_PROT_READ | constants::VM_PROT_WRITE),
	}.build()).collect()
}
