use crate::context::Context;
use crate::error::Diagnostic;
use crate::query::Key;
use crate::span::{S, Span};

use super::{FunctionPath, Identifier, Symbol};

pub type Address = usize;
pub type SymbolSize = usize;

pub fn load(context: &Context, parent: Option<Key>, symbol: &Symbol,
			span: Option<Span>) -> crate::Result<Address> {
	let key = Key::LoadAddress(symbol.clone());
	context.address.scope(parent, key.clone(), span.clone(), || {
		let targets = &[Identifier("load".to_string())];
		let annotation = annotation(context, symbol, targets);
		if let Some(_annotation) = annotation {
			// TODO: load annotation address
			return Ok(1024 * 1024);
		}

		let span = self::span(context, symbol);
		let other = &previous(context, Some(key.clone()),
			symbol, targets, Some(span.clone()))?
			.ok_or_else(|| context.error(Diagnostic::error()
				.message("unable to derive load address")
				.note("add an annotation: @load <address>")
				.label(span.label())))?;

		let base = load(context, Some(key.clone()), other, Some(span.clone()))?;
		if let Symbol::Module(_) = other { return Ok(base); }
		let size = size(context, Some(key), other, Some(span))?;
		Ok(align(other, symbol, base + size))
	}).map(|address| *address)
}

pub fn start(context: &Context, parent: Option<Key>, symbol: &Symbol,
			 span: Option<Span>) -> crate::Result<Address> {
	let key = Key::VirtualAddress(symbol.clone());
	context.address.scope(parent, key.clone(), span.clone(), || {
		let load = Identifier("load".to_string());
		let targets = &[Identifier("virtual".to_string()), load];
		let annotation = annotation(context, symbol, targets);
		if let Some(_annotation) = annotation {
			// TODO: load annotation address
			return Ok(1024 * 1024);
		}

		let span = Some(self::span(context, symbol));
		let previous = previous(context, Some(key.clone()),
			symbol, targets, span.clone())?;
		let other = match previous.as_ref() {
			None => return self::load(context,
				Some(key.clone()), symbol, span),
			Some(previous) => previous,
		};

		let base = start(context, Some(key.clone()), other, span.clone())?;
		if let Symbol::Module(_) = other { return Ok(base); }
		let size = size(context, Some(key), other, span)?;
		Ok(align(other, symbol, base + size))
	}).map(|address| *address)
}

pub fn end(context: &Context, parent: Option<Key>, symbol: &Symbol,
		   span: Option<Span>) -> crate::Result<Address> {
	let base = start(context, parent.clone(), symbol, span.clone())?;
	Ok(base + size(context, parent, symbol, span)?)
}

pub fn size(context: &Context, parent: Option<Key>, symbol: &Symbol,
			span: Option<Span>) -> crate::Result<SymbolSize> {
	let key = Key::SymbolSize(symbol.clone());
	context.address.scope(parent, key.clone(),
		span.clone(), || match symbol {
			Symbol::Variable(path) => {
				let path = crate::inference::type_variable(context,
					Some(key.clone()), path.clone(), span.clone())?;
				super::size(context, Some(key), &path.node, span)
			}
			// TODO: use architecture generation
			Symbol::Function(path) => Ok(crate::generate::x86::lower(context,
				Some(key.clone()), path, span)?.bytes.len()),
			Symbol::Module(path) => {
				let module = context.modules.get(path).unwrap();
				Iterator::zip(module.first.iter(), module.last.iter()).map(|(first, last)| {
					let start = start(context, Some(key.clone()), first, span.clone())?;
					Ok(end(context, Some(key.clone()), last, span.clone())? - start)
				}).next().transpose().map(Option::unwrap_or_default)
			}
		}).map(|size| *size)
}

fn align(other: &Symbol, symbol: &Symbol, address: Address) -> Address {
	match (other, symbol) {
		(Symbol::Function(_), Symbol::Function(_)) => address,
		(Symbol::Variable(_), Symbol::Variable(_)) => address,
		_ => {
			// TODO: derive alignment from annotation
			let alignment = 4 * 1024;
			crate::other::ceiling(address, alignment)
		}
	}
}

fn span(context: &Context, symbol: &Symbol) -> Span {
	match symbol {
		Symbol::Function(FunctionPath(path, kind)) =>
			context.functions.get(path).unwrap()[*kind]
				.identifier.span.clone(),
		Symbol::Variable(path) => context.statics.get(path)
			.unwrap().identifier.span.clone(),
		Symbol::Module(path) => context.modules.get(path)
			.unwrap().identifier.span.clone(),
	}
}

fn annotation(context: &Context, symbol: &Symbol,
			  targets: &[Identifier]) -> Option<S<super::Value>> {
	targets.iter().find_map(|target| match symbol {
		Symbol::Function(FunctionPath(path, kind)) =>
			context.functions.get(path).unwrap()[*kind]
				.annotations.get(target).cloned(),
		Symbol::Variable(path) => context.statics.get(path)
			.unwrap().annotations.get(target).cloned(),
		Symbol::Module(path) => context.modules.get(path)
			.unwrap().annotations.get(target).cloned(),
	})
}

fn previous(context: &Context, parent: Option<Key>, symbol: &Symbol,
			targets: &[Identifier], span: Option<Span>) -> crate::Result<Option<Symbol>> {
	let positions = context.positions.read();
	let position = &positions[symbol];
	match &position.previous {
		Some(symbol) => find(context, parent, symbol, targets, span),
		None => position.parent.as_ref()
			.map(|path| match targets.iter().any(|target| context.modules
				.get(path).unwrap().annotations.contains_key(target)) {
				false => previous(context, parent,
					&Symbol::Module(path.clone()), targets, span),
				true => Ok(Some(Symbol::Module(path.clone()))),
			}).transpose().map(Option::flatten),
	}
}

fn find(context: &Context, parent: Option<Key>, symbol: &Symbol,
		targets: &[Identifier], span: Option<Span>) -> crate::Result<Option<Symbol>> {
	if let Symbol::Module(path) = symbol {
		let module = context.modules.get(path).unwrap();
		let contains = |target| !module.annotations.contains_key(target);
		if targets.iter().any(contains) {
			if let Some(last) = module.last.as_ref() {
				return find(context, parent, last, targets, span);
			}
		}
	}

	match symbol {
		Symbol::Function(function @ FunctionPath(path, kind)) if
		super::present(context, parent.clone(), function, span.clone())?
			&& targets.iter().any(|target| !context.functions.get(path).unwrap()[*kind]
			.annotations.contains_key(target)) => return Ok(Some(symbol.clone())),
		Symbol::Variable(path) if targets.iter().any(|target| context.statics.get(path)
			.unwrap().annotations.contains_key(target)) => return Ok(Some(symbol.clone())),
		_ => previous(context, parent, symbol, targets, span),
	}
}
