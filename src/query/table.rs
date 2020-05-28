use std::sync::Arc;

use dashmap::DashMap;

use crate::span::Span;

use super::Key;

#[derive(Debug)]
pub enum QueryError {
	Cycle(Vec<(Key, Option<Span>)>),
	Failure,
}

#[derive(Debug, Default)]
pub struct Table<V> {
	table: DashMap<Key, (Entry<V>, Vec<Key>)>,
}

impl<V> Table<V> {
	pub fn scope<F>(&self, parent: Option<Key>, key: Key, span: Option<Span>,
					function: F) -> Result<Arc<V>, QueryError>
		where F: FnOnce() -> Result<V, QueryError> {
		if !self.table.contains_key(&key) {
			self.table.insert(key.clone(), (Entry::Pending, Vec::new()));
			self.table.insert(key.clone(), (match function() {
				Ok(value) => Entry::Value(Arc::new(value)),
				Err(QueryError::Failure) => Entry::Failure,
				Err(QueryError::Cycle(mut keys)) => {
					keys.push((key.clone(), span));
					self.table.insert(key, (Entry::Failure, Vec::new()));
					return Err(QueryError::Cycle(keys));
				}
			}, Vec::new()));
		}

		let mut entry = self.table.get_mut(&key).unwrap();
		let (entry, dependents) = entry.value_mut();
		dependents.extend(parent);
		match entry {
			Entry::Value(value) => Ok(value.clone()),
			Entry::Failure => Err(QueryError::Failure),
			Entry::Pending => Err(QueryError::Cycle(vec![(key, span)])),
		}
	}
}

#[derive(Debug)]
enum Entry<V> {
	Pending,
	Failure,
	Value(Arc<V>),
}
