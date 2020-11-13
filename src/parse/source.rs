use std::sync::Arc;

use codespan::{FileId, Files};
use parking_lot::Mutex;

use crate::FilePath;
use crate::query::{E, Key, key, QScope, Table};

#[derive(Debug, Default)]
pub struct Sources {
	paths: Table<key::Source>,
	files: Mutex<Files<Arc<str>>>,
}

impl Sources {
	pub fn files<F, R>(&self, function: F) -> R
		where F: FnOnce(&Files<Arc<str>>) -> R {
		function(&self.files.lock())
	}

	pub fn errors(&self, key: &key::Source) -> (Vec<E>, Vec<Key>) {
		self.paths.errors(key)
	}

	pub fn invalidate(&self, key: &key::Source) -> Vec<Key> {
		// TODO: remove key entry from files
		self.paths.invalidate(key)
	}
}

#[derive(Debug)]
pub struct Source {
	pub text: Arc<str>,
	pub file: FileId,
}

impl Source {
	pub fn reference(&self) -> PSource {
		PSource {
			text: &self.text,
			file: self.file,
		}
	}
}

/// Contains references to `Source` instances.
/// Designed to be copied without overhead during parsing.
#[derive(Debug, Copy, Clone)]
pub struct PSource<'a> {
	pub text: &'a str,
	pub file: FileId,
}

pub fn source(parent: QScope, path: &FilePath) -> crate::Result<Source> {
	let label = parent.span.other();
	let path = &path.canonicalize().map_err(|error| E::error()
		.message(format!("failed to canonicalize path: {}", path.display()))
		.note(error.to_string()).label(label.clone()).to(parent))?;

	let paths = &parent.ctx.source.paths;
	let file = *paths.scope(parent, path.clone(), |scope| {
		let string = std::fs::read_to_string(path).map_err(|error| E::error()
			.message(format!("failed to read file: {}", path.display()))
			.note(error.to_string()).label(label).to(scope))?;
		let (name, string) = (path.file_name().unwrap(), string.into());
		Ok(Arc::new(scope.ctx.source.files.lock().add(name, string)))
	})?;

	let text = parent.ctx.source.files.lock().source(file).clone();
	Ok(Source { text, file })
}
