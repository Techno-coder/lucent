#![feature(option_unwrap_none)]
#![feature(entry_insert)]
#![feature(never_type)]
#![feature(concat_idents)]

mod error;
mod context;
mod inference;
mod generate;
mod arena;
mod query;
mod parse;
mod node;
mod span;

type Result<T> = std::result::Result<T, query::QueryError>;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	let context = &context::Context::default();
	let path = std::path::Path::new("examples/fibonacci.lc");
	query::emit(context, parse::parse(context, path));

	query::emit(context,
		generate::x64::lower(context, None,
			&crate::node::Path(vec![
				crate::node::Identifier("Main".to_string()),
				crate::node::Identifier("fibonacci".to_string()),
			]), 0, None));

	context::display(context)?;
	Ok(())
}
