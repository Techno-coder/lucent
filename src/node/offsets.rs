use std::collections::HashMap;
use std::sync::Arc;

use crate::context::Context;
use crate::node::{Identifier, Path, Size, Type};
use crate::query::{Key, QueryError};
use crate::span::Span;

#[derive(Debug, Default)]
pub struct Offsets {
	pub fields: HashMap<Identifier, usize>,
	pub size: usize,
}

pub fn size(context: &Context, parent: Option<Key>, path: &Type,
			span: Option<Span>) -> crate::Result<usize> {
	Ok(match path {
		Type::Void | Type::Never => 0,
		Type::Truth => Size::Byte.bytes(),
		Type::Rune => Size::Double.bytes(),
		Type::Structure(path) => offsets(context, parent, path, span)?.size,
		Type::Signed(size) | Type::Unsigned(size) => size.bytes(),
		// TODO: dependent on architecture
		Type::Pointer(_) => unimplemented!(),
		// TODO: dependent on compile time evaluation
		Type::Array(_, _) => unimplemented!(),
		// TODO: dependent on pointer size
		Type::Slice(_) => unimplemented!(),
	})
}

// TODO: consider C and packed representation
pub fn offsets(context: &Context, parent: Option<Key>, path: &Path,
			   span: Option<Span>) -> crate::Result<Arc<Offsets>> {
	let key = Key::Offsets(path.clone());
	context.offsets.scope(parent, key.clone(), span, || {
		let mut offsets = Offsets::default();
		let structure = context.structures.get(&path)
			.ok_or(QueryError::Failure)?;

		for (field, path) in &structure.fields {
			offsets.fields.insert(field.clone(), offsets.size);
			offsets.size += size(context, Some(key.clone()),
				&path.node, Some(path.span.clone()))?;
		}

		Ok(offsets)
	})
}

