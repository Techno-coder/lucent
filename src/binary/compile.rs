use std::fs::File;
use std::io::Write;

use crate::context::Context;
use crate::error::Diagnostic;

pub fn compile(context: &Context) -> crate::Result<()> {
	// TODO: derive from global annotation
	let path = "binary.bin";

	// TODO: verify no overlaps
	let mut entries = super::entries(context);
	super::patch(context, &mut entries);
	if crate::context::failed(context) {
		return Err(crate::query::QueryError::Failure);
	}

	let segments = super::segments(entries);
	for segment in &segments {
		match &segment.kind {
			super::SegmentKind::Text(data) => {
				for byte in data.iter().flatten() {
					print!("{:02x} ", byte);
				}
				println!();
			}
			_ => (),
		}
	}

	// TODO: derive format from global annotation
	let data = super::format::mach::compile(segments);

	let data = data.map_err(|error| context.error(Diagnostic::error()
		.message("failed to compile binary").note(format!("error: {}", error))))?;
	File::create(path).and_then(|mut file| file.write_all(&data))
		.map_err(|error| context.error(Diagnostic::error()
			.message("failed to write binary to file")
			.note(format!("error: {}", error))))
}
