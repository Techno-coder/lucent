use crate::node::{Symbol, Target};
use crate::query::QScope;

pub const TARGET_HOST: &str = "host";

/// Derives the target architecture for a symbol.
pub fn target(_scope: QScope, _symbol: &Symbol)
			  -> crate::Result<Option<Target>> {
	// TODO: implement target architecture
	Ok(None)
}
