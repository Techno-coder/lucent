#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Key {
	SymbolFile(std::path::PathBuf)
}
