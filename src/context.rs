use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use codespan::FileId;
use parking_lot::{Mutex, RwLock};

use crate::error::Diagnostic;
use crate::query::Table;

#[derive(Debug, Default)]
pub struct Context {
	pub files: RwLock<Files>,
	pub symbol_files: Table<()>,
	diagnostics: Mutex<Vec<Diagnostic>>,
}

impl Context {
	pub fn emit(&self, diagnostic: Diagnostic) {
		self.diagnostics.lock().push(diagnostic);
	}
}

#[derive(Debug, Default)]
pub struct Files {
	files: codespan::Files<Arc<str>>,
	paths: HashMap<PathBuf, FileId>,
}

impl Files {
	pub fn query(&mut self, path: &std::path::Path) -> Option<(FileId, Arc<str>)> {
		match self.paths.get(path) {
			Some(file) => Some((file.clone(), self.files.source(*file).clone())),
			None => {
				let mut string = String::new();
				let mut file = std::fs::File::open(path).ok()?;
				file.read_to_string(&mut string).ok()?;

				let file = self.files.add(path.file_name().unwrap(), string.into());
				self.paths.insert(path.to_owned(), file);
				Some((file, self.files.source(file).clone()))
			}
		}
	}
}

pub fn display(context: &Context) -> std::io::Result<()> {
	use codespan_reporting::term;
	let files = &context.files.read().files;
	let configuration = &term::Config::default();
	let colors = term::termcolor::ColorChoice::Auto;
	let writer = &mut term::termcolor::StandardStream::stderr(colors);
	context.diagnostics.lock().iter().try_for_each(|Diagnostic(diagnostic)|
		term::emit(writer, configuration, files, diagnostic))
}
