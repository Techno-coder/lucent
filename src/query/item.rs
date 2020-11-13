use std::sync::Arc;

use super::{QScope, QueryError, QueryKey, Table};

impl<K: QueryKey> Table<K> {
	pub fn inherit<P>(&self, scope: QScope, key: impl Into<K>,
					  provide: P) -> Result<Arc<K::Value>, QueryError>
		where P: FnOnce(QScope) -> Result<Arc<K::Value>, QueryError> {
		let span = scope.span.clone();
		self.scope(scope, key, |scope| {
			let scope = &mut scope.span(span);
			provide(scope)
		})
	}
}
