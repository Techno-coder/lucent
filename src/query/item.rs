use std::sync::Arc;

use crate::node::{Symbol, VPath};

use super::*;

pub type IScope<'a, 'b, 'c> = &'c mut ItemScope<'a, 'b>;

#[derive(Debug)]
pub struct ItemScope<'a, 'b> {
	scope: MScope<'a, 'b>,
	pub ctx: &'a Context,
	pub symbol: Symbol,
}

impl<'a, 'b> ItemScope<'a, 'b> {
	pub fn new(scope: MScope<'a, 'b>, symbol: Symbol) -> Self {
		ItemScope { ctx: scope.ctx, scope, symbol }
	}

	pub fn path(scope: MScope<'a, 'b>, VPath(symbol, _): VPath) -> Self {
		Self::new(scope, symbol)
	}

	pub fn span(&mut self, span: ISpan) -> QueryScope<'a, '_> {
		let span = self.lift(span);
		QueryScope { scope: self.scope, span }
	}

	fn lift(&self, span: ISpan) -> ESpan {
		ESpan::Item(self.symbol.clone(), span)
	}
}

impl EScope<ISpan> for ItemScope<'_, '_> {
	fn emit(&mut self, error: E<ISpan>) {
		let labels = error.labels.into_iter()
			.map(|label| Label {
				style: label.style,
				message: label.message,
				span: self.lift(label.span),
			}).collect();

		let error = E { error: error.error, labels };
		self.scope.emit(error);
	}
}

impl<K: QueryKey> Table<K> {
	pub fn inherit<P>(&self, scope: QScope, key: impl Into<K>,
					  provide: P) -> Result<Arc<K::Value>, QueryError>
		where P: FnOnce(QScope) -> Result<Arc<K::Value>, QueryError> {
		let span = scope.span.clone();
		self.scope(scope, key, |scope|
			provide(&mut scope.span(span)))
	}
}
