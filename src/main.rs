#![feature(option_unwrap_none)]
#![feature(bool_to_option)]

mod node;
// mod lower;
// mod generate;
mod compile;
mod query;
mod parse;
// mod analysis;
mod interface;
mod source;
mod server;

pub type FilePath = std::path::PathBuf;
pub type Result<T> = std::result::Result<T, query::QueryError>;
pub type GenericResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn main() -> GenericResult {
	interface::interface()
}
