mod node;
mod lower;
mod generate;
mod compile;
mod query;
mod parse;
mod analysis;

pub type FilePath = std::path::PathBuf;
pub type Result<T> = std::result::Result<T, query::QueryError>;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	println!("Hello, refactor!");
	compile::compile("examples/main.lc".into());
	Ok(())
}
