use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

use super::{Context, E, ESpan, Key};

pub type MScope<'a, 'b> = &'b mut Scope<'a>;
pub type QScope<'a, 'b, 'c> = &'c mut QueryScope<'a, 'b>;

#[derive(Debug, Default)]
pub struct ScopeHandle {
	cancel: AtomicBool,
}

impl ScopeHandle {
	pub fn cancel(&self) {
		self.cancel.store(true, Ordering::SeqCst);
	}

	pub fn cancelled(&self) -> bool {
		self.cancel.load(Ordering::SeqCst)
	}
}

pub trait EScope<S> {
	/// Adds an error to this query.
	fn emit(&mut self, error: E<S>);
}

#[derive(Debug)]
pub struct Scope<'a> {
	pub ctx: &'a Context,
	pub(super) handle: Option<&'a ScopeHandle>,
	pub(super) dependencies: Vec<Key>,
	pub(super) parent: Option<Key>,
	pub(super) errors: Vec<E>,
}

impl<'a> Scope<'a> {
	pub(super) fn cancelled(&self) -> bool {
		self.handle.map(ScopeHandle::cancelled).unwrap_or(false)
	}

	pub(super) fn new(ctx: &'a Context, handle: Option<&'a ScopeHandle>,
					  parent: Option<Key>) -> Self {
		Self { ctx, handle, dependencies: vec![], parent, errors: vec![] }
	}

	pub fn root(ctx: &'a Context, handle: Option<&'a ScopeHandle>) -> Self {
		Self::new(ctx, handle, None)
	}

	/// Converts this scope to a `QueryScope` by
	/// annotating it with a source location span.
	pub fn span(&mut self, span: impl Into<ESpan>) -> QueryScope<'a, '_> {
		QueryScope { scope: self, span: span.into() }
	}
}

impl EScope<ESpan> for Scope<'_> {
	fn emit(&mut self, error: E) {
		self.errors.push(error);
	}
}

/// Represents a scope to be passed as a
/// query parameter. The span field provides the
/// source location of the query invocation.
#[derive(Debug)]
pub struct QueryScope<'a, 'b> {
	pub(super) scope: MScope<'a, 'b>,
	pub span: ESpan,
}

impl EScope<ESpan> for QueryScope<'_, '_> {
	fn emit(&mut self, error: E) {
		self.scope.emit(error);
	}
}

impl<'a, 'b> Deref for QueryScope<'a, 'b> {
	type Target = Scope<'a>;

	fn deref(&self) -> &Self::Target {
		&self.scope
	}
}

impl<'a, 'b> DerefMut for QueryScope<'a, 'b> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.scope
	}
}
