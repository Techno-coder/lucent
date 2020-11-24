use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use codespan_reporting::files::Files;
use dashmap::DashMap;

use crate::FilePath;
use crate::query::{E, key, Key, QScope, Table};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct File(usize);

#[derive(Debug)]
pub struct Source {
	pub file: File,
	pub text: Arc<str>,
	pub path: FilePath,
	starts: Vec<usize>,
	name: String,
}

impl Source {
	fn new(file: File, path: FilePath, text: Arc<str>) -> Self {
		use codespan_reporting::files::line_starts;
		let starts = line_starts(text.as_ref()).collect();
		let name = path.file_name().unwrap().to_string_lossy().into();
		Source { file, text, path, starts, name }
	}

	fn update(&self, text: Arc<str>) -> Self {
		Self::new(self.file, self.path.clone(), text)
	}
}

// TODO: consider possible race conditions
#[derive(Debug, Default)]
pub struct FileCache {
	table: Table<key::Read>,
	paths: DashMap<FilePath, File>,
	source: DashMap<File, Arc<Source>>,
	next: AtomicUsize,
}

impl FileCache {
	/// Creates a new file entry in the cache. Overwrites
	/// any existing entry and returns a `Key` for invalidation.
	pub fn create(&self, path: &FilePath, text: Arc<str>) -> Key {
		if self.paths.contains_key(path) {
			return self.update(path, text);
		}

		let file = self.insert(path.clone(), text);
		self.paths.insert(path.clone(), file);
		Key::Read(path.clone().into())
	}

	/// Updates a file entry in the cache and panics if the
	/// entry does not exist. Returns a `Key` for invalidation.
	pub fn update(&self, path: &FilePath, text: Arc<str>) -> Key {
		let file = *self.paths.get(path).unwrap_or_else(||
			panic!("path: {}, is absent from cache")).value();
		self.source.alter(&file, |_, source| Arc::new(source.update(text)));
		Key::Read(path.clone().into())
	}

	/// Removes a file from the cache. Note that this does not
	/// invalidate the query table. Returns a `Key` for invalidation.
	pub fn remove(&self, path: &FilePath) -> Key {
		self.paths.remove(path).and_then(|(_, file)|
			self.source.remove(&file));
		Key::Read(path.clone().into())
	}

	pub fn source(&self, file: File) -> Option<Arc<Source>> {
		self.source.get(&file).as_deref().cloned()
	}

	pub fn invalidate(&self, key: &key::Read) -> Vec<Key> {
		self.table.invalidate(key)
	}

	pub fn errors(&self, key: &key::Read) -> (Vec<E>, Vec<Key>) {
		self.table.errors(key)
	}

	fn insert(&self, path: FilePath, text: Arc<str>) -> File {
		let file = self.next();
		let source = Arc::new(Source::new(file, path, text));
		self.source.insert(file, source);
		file
	}

	fn next(&self) -> File {
		let ordering = std::sync::atomic::Ordering::SeqCst;
		File(self.next.fetch_add(1, ordering))
	}
}

impl<'a> Files<'a> for FileCache {
	type FileId = File;
	type Name = String;
	type Source = Arc<str>;

	fn name(&'a self, file: File) -> Option<Self::Name> {
		self.source.get(&file).map(|source| source.name.clone())
	}

	fn source(&'a self, file: File) -> Option<Self::Source> {
		self.source.get(&file).map(|source| source.text.clone())
	}

	fn line_index(&'a self, file: File, byte: usize) -> Option<usize> {
		let source = self.source.get(&file)?;
		Some(match source.starts.binary_search(&byte) {
			Err(next) => next as usize - 1,
			Ok(line) => line as usize,
		})
	}

	fn line_range(&'a self, file: File, line: usize) -> Option<Range<usize>> {
		let source = self.source.get(&file)?;
		let compare = |line: usize| line.cmp(&source.starts.len());
		let line_start = |line| match compare(line) {
			Ordering::Less => Some(source.starts[line]),
			Ordering::Equal => Some(source.text.len()),
			Ordering::Greater => None,
		};

		let start = line_start(line)?;
		let next = line_start(line + 1)?;
		Some(start..next)
	}
}

pub fn canonicalize(scope: QScope, path: &FilePath) -> crate::Result<FilePath> {
	path.canonicalize().map_err(|error| E::error()
		.message(format!("failed to canonicalize path: {}", path.display()))
		.note(error.to_string()).label(scope.span.other()).to(scope))
}

pub fn file(parent: QScope, path: &FilePath) -> crate::Result<File> {
	let files = &parent.ctx.files;
	let path = &canonicalize(parent, path)?;
	files.table.inherit(parent, path.clone(), |scope| {
		if let Some(file) = files.paths.get(path) {
			return Ok(Arc::new(*file.value()));
		}

		let text = std::fs::read_to_string(path).map_err(|error| E::error()
			.message(format!("failed to read file: {}", path.display()))
			.note(error.to_string()).label(scope.span.other()).to(scope))?;

		let file = files.insert(path.clone(), text.into());
		files.paths.insert(path.clone(), file);
		Ok(Arc::new(file))
	}).map(|file| *file)
}

pub fn source(parent: QScope, path: &FilePath) -> crate::Result<Arc<Source>> {
	let file = file(parent, path)?;
	let source = &parent.ctx.files.source;
	Ok(source.get(&file).unwrap().clone())
}
