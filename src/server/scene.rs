use std::collections::HashSet;

use lsp_server::Connection;

use crate::FilePath;
use crate::query::{Context, Key, Scope, ScopeHandle};

pub type MScene<'a, 'b> = &'b mut Scene<'a>;

pub struct Scene<'a> {
	pub ctx: &'a Context,
	pub connection: &'a Connection,
	pub watched: HashSet<FilePath>,
	scopes: Vec<ScopeHandle>,
}

impl<'a> Scene<'a> {
	pub fn new(ctx: &'a Context, connection: &'a Connection) -> Self {
		Self { ctx, connection, watched: HashSet::new(), scopes: vec![] }
	}

	pub fn scope(&mut self) -> Scope {
		self.scopes.push(ScopeHandle::default());
		let handle: &ScopeHandle = self.scopes.last().unwrap();
		Scope::root(self.ctx, Some(handle))
	}

	pub fn cancel(&mut self) {
		self.scopes.drain(..).for_each(|scope| scope.cancel());
	}

	pub fn invalidate(&mut self, key: &Key) {
		self.cancel();
		self.ctx.invalidate(key);
	}
}

/// Convenience macro for constructing a `QScope`.
macro_rules! scope {
    ($scope:ident, $scene:expr) => {
    	let mut scope = $scene.scope();
    	let span = crate::query::Span::internal();
		let $scope = &mut scope.span(span);
    };
}

