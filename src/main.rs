#![feature(option_unwrap_none)]
#![feature(entry_insert)]
#![feature(never_type)]
#![feature(concat_idents)]
#![feature(bindings_after_at)]

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
	query::emit(context, execute(context, path));
	Ok(context::display(context)?)
}

fn execute(context: &context::Context, path: &std::path::Path) -> Result<()> {
	parse::parse(context, path)?;
	node::positions(context);
	node::present_all(context)?;

	dbg!(node::address::size(context, None,
		&node::Symbol::Function(node::FunctionPath(crate::node::Path(vec![
			node::Identifier("Test".to_string()),
			node::Identifier("fibonacci".to_string()),
		]), 0)), Some(context.files.read().internal.clone()))?);
	dbg!(node::address::size(context, None,
		&node::Symbol::Module(crate::node::Path(vec![
			node::Identifier("Test".to_string()),
		])), Some(context.files.read().internal.clone()))?);
	Ok(())
}
