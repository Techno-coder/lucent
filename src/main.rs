#![feature(option_unwrap_none)]
#![feature(bindings_after_at)]
#![feature(bool_to_option)]
#![feature(box_patterns)]
#![feature(or_patterns)]
#![feature(once_cell)]

mod node;
// mod lower;
mod generate;
mod compile;
mod query;
mod parse;
mod analysis;
mod inference;
mod interface;
mod source;
mod server;

pub type FilePath = std::path::PathBuf;
pub type Result<T> = std::result::Result<T, query::QueryError>;
pub type GenericResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn main() -> GenericResult {
	interface::interface()
}
