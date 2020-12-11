use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use lsp_server::Connection;
use parking_lot::RwLock;

use crate::FilePath;
use crate::node::Path;
use crate::query::{Context, Scope, ScopeHandle, Span};

pub type LScene<'a, 'b> = &'b RwLock<Scene<'a>>;
pub type RScene<'a, 'b> = &'b Scene<'a>;

pub struct Scene<'a> {
	pub connection: &'a Connection,
	pub workspace: Option<FilePath>,
	pub handle: ScopeHandle,
	pub watched: HashSet<FilePath>,
	pub unlinked: HashMap<FilePath, Context>,
	pub targets: Vec<Context>,
}

impl<'a> Scene<'a> {
	pub fn new(connection: &'a Connection,
			   workspace: Option<FilePath>) -> Self {
		Scene {
			workspace,
			connection,
			watched: HashSet::new(),
			handle: ScopeHandle::default(),
			unlinked: HashMap::new(),
			targets: vec![],
		}
	}

	pub fn scope(&'a self, ctx: &'a Context) -> Scope<'a> {
		Scope::root(ctx, Some(&self.handle))
	}

	pub fn scopes(&self, path: &FilePath) -> Vec<Scope> {
		let (targets, unlinked) = (self.targets.iter(), self.unlinked.get(path));
		Iterator::chain(targets, unlinked).map(|ctx| self.scope(ctx)).collect()
	}

	pub fn modules(&self, path: &FilePath) -> Vec<(Scope, Arc<Path>)> {
		let (targets, unlinked) = (self.targets.iter(), self.unlinked.get(path));
		Iterator::chain(targets, unlinked).map(|ctx| super::file_modules(&mut
			self.scope(ctx).span(Span::internal()), path).into_iter().flatten()
			.map(move |path| (self.scope(ctx), path))).flatten().collect()
	}
}
