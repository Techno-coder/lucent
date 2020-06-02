#![feature(option_unwrap_none)]
#![feature(entry_insert)]
#![feature(never_type)]

mod error;
mod context;
mod inference;
mod arena;
mod query;
mod parse;
mod node;
mod span;

type Result<T> = std::result::Result<T, query::QueryError>;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	let context = &context::Context::default();
	let path = std::path::Path::new("examples/main.lc");
	query::emit(context, parse::parse(context, path));

	query::emit(context,
		inference::type_function(context, None,
			crate::node::Path(vec![
				crate::node::Identifier("Loader".to_string()),
				crate::node::Identifier("Main".to_string()),
				crate::node::Identifier("start".to_string()),
			]), 0, None));

	context::display(context)?;
	Ok(())
}
