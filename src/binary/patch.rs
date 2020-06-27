use crate::context::Context;
use crate::generate::Relative;
use crate::node::{Size, Symbol};

use super::{Entity, Entry};

pub fn patch(context: &Context, entries: &mut [Entry]) -> crate::Result<()> {
	entries.iter_mut().map(|Entry { address, entity, .. }| Ok(match entity {
		Entity::Function(section) => {
			for Relative { size, offset, target, path } in &section.relative {
				let symbol = Symbol::Function(path.clone());
				let other = crate::node::address::start(context, None, &symbol, None)?;
				let slice = &mut section.bytes[*offset..*offset + size.bytes()];
				let relative = other as isize - (*address + *target) as isize;

				match size {
					Size::Byte => slice.copy_from_slice(&(relative as i8).to_le_bytes()),
					Size::Word => slice.copy_from_slice(&(relative as i16).to_le_bytes()),
					Size::Double => slice.copy_from_slice(&(relative as i32).to_le_bytes()),
					Size::Quad => slice.copy_from_slice(&(relative as i64).to_le_bytes()),
				}
			}
		}
		Entity::Variable(_) => (),
	})).filter(Result::is_err).last().unwrap_or(Ok(()))
}
