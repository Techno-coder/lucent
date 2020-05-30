use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use codespan::FileId;
use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};

use crate::error::Diagnostic;
use crate::node::{Function, Parameter, Path, Static};
use crate::query::{QueryError, Table};

#[derive(Debug, Default)]
pub struct Context {
	pub files: RwLock<Files>,
	pub symbol_files: Table<()>,
	pub statics: DashMap<Path, Static>,
	pub functions: DashMap<Path, Vec<(Vec<Parameter>, Function)>>,
	diagnostics: Mutex<Vec<Diagnostic>>,
}

impl Context {
	pub fn pass<T>(&self, diagnostic: Diagnostic) -> crate::Result<T> {
		self.diagnostics.lock().push(diagnostic);
		Err(QueryError::Failure)
	}

	pub fn emit(&self, diagnostic: Diagnostic) {
		let _ = self.pass::<!>(diagnostic);
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
