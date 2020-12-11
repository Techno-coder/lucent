use std::collections::HashSet;
use std::sync::Arc;
use std::thread::ThreadId;

use dashmap::DashMap;
use dashmap::mapref::entry;

use super::*;

#[derive(Debug)]
pub enum QueryError {
	Cycle(Vec<(ESpan, Key)>),
	Cancelled,
	Failure,
}

/// A concurrent table that contains entries for
/// query results. Guaranteed to be deadlock free.
#[derive(Debug)]
pub struct Table<K: QueryKey> {
	table: DashMap<K, Entry<K>>,
}

impl<K: QueryKey> Table<K> {
	pub fn scope<P>(&self, scope: QScope, key: impl Into<K>,
					provide: P) -> Result<Arc<K::Value>, QueryError>
		where P: FnOnce(MScope) -> Result<Arc<K::Value>, QueryError> {
		if scope.cancelled() { return Err(QueryError::Cancelled); }
		let key: K = key.into();

		// Add this query as a dependency of the parent query.
		scope.dependencies.push(key.clone().into());

		// Retrieve query entry. Effectively
		// takes lock on entire table.
		match self.table.entry(key.clone()) {
			entry::Entry::Occupied(mut lock) => {
				let entry = lock.get_mut();
				entry.dependents.extend(scope.parent.clone());
				match &mut entry.kind {
					EntryKind::Value(value) => Ok(value.clone()),
					EntryKind::Failure => Err(QueryError::Failure),
					EntryKind::Pending(set) => {
						let kind = std::thread::current().id();
						if set.contains(&kind) {
							// Thread has already initiated pending query.
							let trace = (scope.span.clone(), key.into());
							Err(QueryError::Cycle(vec![trace]))
						} else {
							// Different thread initiating pending query.
							// To make progress we duplicate the computation but
							// we do not competitively store the result. This
							// means a query can be executed more than once
							// for a particular key.
							set.insert(kind);
							drop(lock);

							let mut scoped = Scope::new(scope.ctx,
								scope.handle, Some(key.clone().into()));
							match provide(&mut scoped) {
								Ok(value) => Ok(value),
								Err(QueryError::Failure) => Err(QueryError::Failure),
								Err(QueryError::Cancelled) => Err(QueryError::Cancelled),
								Err(QueryError::Cycle(mut keys)) => {
									keys.push((scope.span.clone(), key.into()));
									Err(QueryError::Cycle(keys))
								}
							}
						}
					}
				}
			}
			entry::Entry::Vacant(lock) => {
				let mut entry = lock.insert(Entry::pending());
				entry.dependents.extend(scope.parent.clone());
				drop(entry);

				let mut scoped = Scope::new(scope.ctx,
					scope.handle, Some(key.clone().into()));
				let mut result = provide(&mut scoped);
				let cancelled = || match scope.cancelled() {
					false => panic!("table invalidation before cancellation"),
					true => Err(QueryError::Cancelled),
				};

				match self.table.entry(key.clone()) {
					// Entry has been invalidated.
					entry::Entry::Vacant(_) => cancelled(),
					entry::Entry::Occupied(mut lock) => {
						let entry = lock.get_mut();
						if let EntryKind::Pending(set) = &entry.kind {
							if !set.contains(&std::thread::current().id()) {
								// Entry originated from different thread. This
								// means another query has started after this
								// entry was invalidated.
								cancelled()
							} else if scope.cancelled() {
								// Entry has not been mutated since
								// this query has started.
								let _ = lock.remove();
								cancelled()
							} else {
								match &mut result {
									Ok(value) => entry.kind =
										EntryKind::Value(value.clone()),
									Err(QueryError::Failure) =>
										entry.kind = EntryKind::Failure,
									Err(QueryError::Cycle(keys)) => {
										entry.kind = EntryKind::Failure;
										keys.push((scope.span.clone(), key.into()));
									}
									Err(QueryError::Cancelled) => {
										let _ = lock.remove();
										return cancelled();
									}
								}

								entry.errors = scoped.errors;
								entry.dependencies = scoped.dependencies;
								result
							}
						} else {
							// Entry is not pending. This means another query
							// has completed after this entry was invalidated.
							cancelled()
						}
					}
				}
			}
		}
	}

	pub fn errors(&self, key: &K) -> (Vec<E>, Vec<Key>) {
		self.table.get(key)
			.filter(|entry| !matches!(entry.kind, EntryKind::Pending(_)))
			.map(|entry| (entry.errors.clone(), entry.dependencies.clone()))
			.unwrap_or_else(|| (vec![], vec![]))
	}

	/// Invalidates and removes the entry associated
	/// with a key. Returns a list of dependent keys
	/// for further invalidation.
	///
	/// All pending queries into this table must be
	/// cancelled before calling this function.
	pub fn invalidate(&self, key: &K) -> Vec<Key> {
		self.table.remove(key)
			.map(|(_, entry)| entry.dependents)
			.unwrap_or_default()
	}
}

impl<K: QueryKey> Default for Table<K> {
	fn default() -> Self {
		Table { table: DashMap::new() }
	}
}

#[derive(Debug)]
pub struct Entry<K: QueryKey> {
	kind: EntryKind<K::Value>,
	pub dependents: Vec<Key>,
	pub dependencies: Vec<Key>,
	pub errors: Vec<E>,
}

impl<K: QueryKey> Entry<K> {
	/// Creates a new pending entry. Initializes
	/// the thread access set with the current thread.
	pub fn pending() -> Self {
		let mut kind = HashSet::new();
		kind.insert(std::thread::current().id());

		Entry {
			kind: EntryKind::Pending(kind),
			dependents: vec![],
			dependencies: vec![],
			errors: vec![],
		}
	}
}

#[derive(Debug)]
enum EntryKind<V> {
	Value(Arc<V>),
	Pending(HashSet<ThreadId>),
	Failure,
}
