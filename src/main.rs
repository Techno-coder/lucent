#![feature(option_unwrap_none)]

mod error;
mod context;
mod arena;
mod query;
mod parse;
mod node;
mod span;

type Result<T> = std::result::Result<T, query::QueryError>;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	let context = &context::Context::default();
	let string = "examples/main.lc".to_string();
	let path = std::path::PathBuf::from(string);
	let symbols = parse::Symbols::root(context, &path);

	if let Err(query::QueryError::Cycle(spans)) = symbols {
		let mut diagnostic = error::Diagnostic::error()
			.message("compilation cycle");
		for (_, span) in spans.iter().rev() {
			if let Some(span) = span {
				diagnostic = diagnostic.label(span.other()
					.with_message("in parsing symbols from file"));
			}
		}

		context.emit(diagnostic);
	} else {
		println!("{:#?}", symbols.unwrap().items);
	}

	context::display(context)?;
	Ok(())
}
