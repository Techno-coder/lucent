use crate::generate::Target;
use crate::node::Symbol;
use crate::query::QScope;

/// Derives the target architecture for a symbol.
pub fn target(_scope: QScope, _symbol: &Symbol)
			  -> crate::Result<Option<Target>> {
	// TODO: implement target architecture
	Ok(Some(Target::Host))
}
