use tree_sitter::{Language, Parser, Query};

extern { fn tree_sitter_lucent() -> Language; }

pub fn parser() -> Parser {
	let mut parser = Parser::new();
	let language = unsafe { tree_sitter_lucent() };
	parser.set_language(language).unwrap();
	parser
}

/// Returns a query that finds syntax errors.
pub fn errors_query() -> Query {
	let language = unsafe { tree_sitter_lucent() };
	Query::new(language, "(ERROR) @error").unwrap()
}
