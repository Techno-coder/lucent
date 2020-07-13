use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::{FunctionPath, Path, Symbol};
use crate::query::QueryError;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Key {
	SymbolFile(std::path::PathBuf),
	TypeFunction(FunctionPath),
	TypeVariable(Path),
	Offsets(Path),
	TraverseRoots,
	SymbolSize(Symbol),
	LoadAddress(Symbol),
	VirtualAddress(Symbol),
	Generate(FunctionPath),
	Analyze(FunctionPath),
}

impl Key {
	fn action(&self) -> &'static str {
		match self {
			Key::SymbolFile(_) => "in parsing symbols from file",
			Key::TypeFunction(_) => "in type checking function",
			Key::TypeVariable(_) => "in type checking static variable",
			Key::Offsets(_) => "in deriving structure offsets",
			Key::TraverseRoots => "in traversing root functions",
			Key::SymbolSize(_) => "in deriving symbol size",
			Key::LoadAddress(_) => "in deriving load address",
			Key::VirtualAddress(_) => "in deriving virtual address",
			Key::Generate(_) => "in generating function",
			Key::Analyze(_) => "in analyzing function",
		}
	}
}

pub fn emit<T>(context: &Context, result: crate::Result<T>) {
	if let Err(QueryError::Cycle(keys)) = result {
		let diagnostic = Diagnostic::error().message("compilation cycle");
		let diagnostic = keys.into_iter().rev().filter_map(|(key, span)|
			span.map(|span| (key, span))).fold(diagnostic, |diagnostic, (key, span)|
			diagnostic.label(span.other().with_message(key.action())));
		context.emit(diagnostic);
	}
}
