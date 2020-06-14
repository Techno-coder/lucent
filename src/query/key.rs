use crate::context::Context;
use crate::error::Diagnostic;
use crate::node::{FunctionKind, Path};
use crate::query::QueryError;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Key {
	SymbolFile(std::path::PathBuf),
	TypeFunction(Path, FunctionKind),
	TypeVariable(Path),
	Offsets(Path),
	Generate(Path, FunctionKind),
}

impl Key {
	fn action(&self) -> &'static str {
		match self {
			Key::SymbolFile(_) => "in parsing symbols from file",
			Key::TypeFunction(_, _) => "in type checking function",
			Key::TypeVariable(_) => "in type checking static variable",
			Key::Offsets(_) => "in computing structure offsets",
			Key::Generate(_, _) => "in generating function",
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
