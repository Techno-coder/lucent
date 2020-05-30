#![feature(option_unwrap_none)]
#![feature(never_type)]

mod error;
mod context;
mod arena;
mod query;
mod parse;
mod node;
mod span;

type Result<T> = std::result::Result<T, query::QueryError>;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	use query::QueryError;
	let context = &context::Context::default();
	let path = std::path::Path::new("examples/main.lc");
	if let Err(QueryError::Cycle(spans)) = parse::parse(context, path) {
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
		println!("{:#?}", context);
	}

	context::display(context)?;
	Ok(())
}
