use std::collections::HashMap;
use std::sync::Arc;

use crate::generate::Target;
use crate::node::{Identifier, Path, RType, Size, Symbol, Type, Width};
use crate::query::{E, IScope, ItemScope, QScope, S};

#[derive(Debug)]
pub struct Offsets {
	pub fields: HashMap<Identifier, usize>,
	pub size: Size,
}

pub fn size(scope: IScope, kind: &S<RType>) -> crate::Result<S<Size>> {
	let pointer = |scope, target: &Option<Target>| match target {
		Some(target) => Ok(Size(target.pointer.bytes())),
		None => E::error().message("unknown architecture")
			.label(kind.span.label()).result(scope),
	};

	Ok(S::new(match &kind.node {
		Type::Void | Type::Never => Size(0),
		Type::Rune => Size(Width::D.bytes()),
		Type::Truth => Size(Width::B.bytes()),
		Type::Integral(_, width) => Size(width.bytes()),
		Type::IntegralSize(target, _) => pointer(scope, target)?,
		Type::Pointer(target, _) => pointer(scope, target)?,
		Type::Structure(path) => {
			let scope = &mut scope.span(kind.span);
			offsets(scope, path)?.size
		}
		Type::Function(signature) => {
			let target = &signature.target;
			pointer(scope, target)?
		}
		Type::Slice(target, _) => {
			let Size(size) = pointer(scope, target)?;
			Size(size * 2)
		}
		Type::Array(kind, length) => {
			let Size(size) = size(scope, kind)?.node;
			Size(size * length)
		}
	}, kind.span))
}

// TODO: consider alternative aligned representations
pub fn offsets(scope: QScope, data: &Arc<Path>)
			   -> crate::Result<Arc<Offsets>> {
	scope.ctx.offsets.inherit(scope, data.clone(), |scope| {
		let symbol = Symbol::Structure(data.clone());
		let data = crate::parse::structure(scope, data)?;
		let mut fields = HashMap::new();
		let mut total = 0;

		let scope = &mut ItemScope::new(scope, symbol);
		for (name, (_, kind)) in &data.fields {
			fields.insert(name.clone(), total);
			let kind = crate::inference::lift(scope, kind)?;
			let Size(size) = size(scope, &kind)?.node;
			total += size;
		}

		let size = Size(total);
		Ok(Arc::new(Offsets { fields, size }))
	})
}
