use std::collections::HashMap;
use std::sync::Arc;

use crate::node::*;
use crate::parse::{PStatic, Universal};
use crate::query::{IScope, ItemScope, QScope, S};

use super::{IType, Scene};

#[derive(Debug, Default)]
pub struct Types {
	pub nodes: HashMap<HIndex, S<RType>>,
	pub variables: HashMap<Variable, S<RType>>,
	pub functions: HashMap<HIndex, FIndex>,
}

/// Infers the types in a function root value.
/// The referenced function must be local.
pub fn function(scope: QScope, path: &FLocal)
				-> crate::Result<Arc<Types>> {
	scope.ctx.typed.inherit(scope, path.clone(), |scope| {
		let local = crate::parse::local(scope, path)?;
		let symbol = Symbol::Function(path.as_ref().clone());
		let scope = &mut ItemScope::new(scope, symbol.clone());
		let hint = lift(scope, &local.signature.return_type)?;

		let mut types = Types::default();
		for (name, (_, kind)) in &local.signature.parameters {
			let variable = Variable(name.clone(), 0);
			types.variables.insert(variable, lift(scope, kind)?);
		}

		let (value, return_type) = (&local.value, Some(hint.clone()));
		let mut scene = Scene { scope, return_type, value, types };
		super::check(&mut scene, &value.root, super::raise(hint));
		Ok(Arc::new(scene.types))
	})
}

/// Infers the types in a value. See `hint`
/// for details on invoking this query.
pub fn types(scope: QScope, path: &VPath)
			 -> crate::Result<Arc<Types>> {
	let VPath(symbol, index) = path;
	if let Symbol::Static(statics) = symbol {
		let statics = crate::parse::statics(scope, statics)?;
		if let PStatic::Local(local) = statics.as_ref() {
			let valued = Some(*index) == local.value;
			if let (true, Some(kind)) = (valued, &local.kind) {
				let path = &VPath(symbol.clone(), *index);
				let scoped = &mut ItemScope::new(scope, symbol.clone());
				let kind = super::raise(lift(scoped, kind)?);
				return hint(scope, path, Some(kind));
			}
		}
	}

	// No hints available for symbol.
	hint(scope, path, None)
}

/// Infers the types in a value. If a hint is
/// provided then the type of the value root
/// will be matched against the hint type.
///
/// While the hint is not part of the query key,
/// the query depends on the entire item so this
/// remains valid on item invalidations. However,
/// as a consequence, functions invoking this query
/// must be careful to invoke dependent queries
/// first or else types in compile time contexts
/// may remain erroneously unresolved.
pub fn hint(scope: QScope, path: &VPath,
			hint: Option<IType>) -> crate::Result<Arc<Types>> {
	scope.ctx.types.inherit(scope, path.clone(), |scope| {
		let value = crate::parse::value(scope, path)?;
		let scope = &mut ItemScope::path(scope, path.clone());
		let (types, value) = (Types::default(), value.as_ref());
		let mut scene = Scene { scope, return_type: None, value, types };

		match hint {
			Some(hint) => super::check(&mut scene, &value.root, hint),
			None => drop(super::synthesize(&mut scene, &value.root)),
		}

		Ok(Arc::new(scene.types))
	})
}

/// Infers the type for a static variable.
pub fn statics(scope: QScope, path: &Arc<Path>) -> crate::Result<RType> {
	let symbol = &Symbol::Static(path.clone());
	let scoped = |scope| ItemScope::new(scope, symbol.clone());
	Ok(match crate::parse::statics(scope, path)?.as_ref() {
		Universal::Load(load) => lift(&mut scoped(scope), &load.kind)?,
		Universal::Local(local) => match &local.value {
			Some(value) => {
				let root = &local.values[*value].root;
				let path = &VPath(symbol.clone(), *value);
				types(scope, path)?.nodes[root].clone()
			}
			None => {
				let kind = local.kind.as_ref().unwrap();
				lift(&mut scoped(scope), kind)?
			}
		},
	}.node)
}

pub fn signatures(scope: QScope, path: &Arc<Path>) -> crate::Result<Vec<Signature>> {
	let functions = crate::parse::functions(scope, path)?;
	functions.iter().enumerate().map(|(index, function)| {
		let symbol = Symbol::Function(FPath(path.clone(), index));
		let scope = &mut ItemScope::new(scope, symbol);
		lift_signature(scope, function.signature())
	}).collect()
}

pub fn lift(scope: IScope, kind: &S<HType>) -> crate::Result<S<RType>> {
	Ok(S::new(match &kind.node {
		HType::Void => RType::Void,
		HType::Rune => RType::Rune,
		HType::Truth => RType::Truth,
		HType::Never => RType::Never,
		HType::Structure(path) => RType::Structure(path.path()),
		HType::Integral(sign, width) => RType::Integral(*sign, *width),
		HType::IntegralSize(sign) => RType::IntegralSize(*sign),
		HType::Pointer(kind) => {
			let kind = lift(scope, kind)?;
			RType::Pointer(Box::new(kind))
		}
		HType::Function(signature) => {
			let signature = lift_signature(scope, signature);
			RType::Function(Box::new(signature?))
		}
		HType::Array(kind, size) => {
			// TODO: evaluate array size
			let _size = VPath(scope.symbol.clone(), *size);
			let kind = lift(scope, kind)?;
			RType::Array(Box::new(kind), 0)
		}
		HType::Slice(kind) => {
			let kind = lift(scope, kind)?;
			RType::Slice(Box::new(kind))
		}
	}, kind.span))
}

pub fn lift_signature(scope: IScope, signature: &HSignature)
					  -> crate::Result<Signature> {
	let convention = signature.convention.clone();
	let parameters = signature.parameters.values().map(|(_, kind)|
		lift(scope, kind)).collect::<Result<_, _>>()?;
	let return_type = lift(scope, &signature.return_type)?;
	Ok(Signature { convention, parameters, return_type })
}
