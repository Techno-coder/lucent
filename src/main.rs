#![feature(option_unwrap_none)]
#![feature(entry_insert)]
#![feature(never_type)]
#![feature(concat_idents)]
#![feature(bindings_after_at)]
#![feature(array_value_iter)]
#![feature(bool_to_option)]

mod error;
mod other;
mod context;
mod inference;
mod generate;
mod analysis;
mod binary;
mod arena;
mod query;
mod parse;
mod node;
mod span;

type Result<T> = std::result::Result<T, query::QueryError>;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	let context = &context::Context::default();
	let path = std::path::Path::new("examples/structures.lc");
	query::emit(context, execute(context, path));
	Ok(context::display(context)?)
}

fn execute(context: &context::Context, path: &std::path::Path) -> Result<()> {
	parse::parse(context, path)?;
	node::positions(context);
	node::present_all(context)?;
	binary::compile(context)
}
