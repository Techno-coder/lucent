use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use codespan::FileId;
use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};

use crate::error::Diagnostic;
use crate::generate::Section;
use crate::inference::Types;
use crate::node::*;
use crate::query::{QueryError, Table};
use crate::span::Span;

#[derive(Debug, Default)]
pub struct Context {
	pub unit: Table<()>,
	pub files: RwLock<Files>,
	pub items: RwLock<Vec<Item>>,
	pub modules: DashMap<Path, Module>,
	pub statics: DashMap<Path, Static>,
	pub structures: DashMap<Path, Structure>,
	pub functions: DashMap<Path, Vec<Arc<Function>>>,
	pub positions: RwLock<HashMap<Symbol, Position>>,
	pub present: RwLock<HashSet<FunctionPath>>,
	pub type_contexts: Table<Types>,
	pub sections: Table<Section>,
	pub offsets: Table<Offsets>,
	pub address: Table<usize>,
	diagnostics: Mutex<Vec<Diagnostic>>,
}

impl Context {
	pub fn error(&self, diagnostic: Diagnostic) -> QueryError {
		self.diagnostics.lock().push(diagnostic);
		QueryError::Failure
	}

	pub fn pass<T>(&self, diagnostic: Diagnostic) -> crate::Result<T> {
		Err(self.error(diagnostic))
	}

	pub fn emit(&self, diagnostic: Diagnostic) {
		let _ = self.pass::<!>(diagnostic);
	}
}

#[derive(Debug)]
pub struct Files {
	files: codespan::Files<Arc<str>>,
	paths: HashMap<PathBuf, FileId>,
	pub internal: Span,
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

impl Default for Files {
	fn default() -> Self {
		let paths = HashMap::new();
		let mut files = codespan::Files::new();
		let file = files.add("<internal>", "<compiler internal>".into());
		let start = files.source_span(file).start().to_usize();
		let end = files.source_span(file).end().to_usize();
		let internal = Span::new(start..end, file);
		Files { files, paths, internal }
	}
}

pub fn failed(context: &Context) -> bool {
	context.diagnostics.lock().iter()
		.any(|Diagnostic(diagnostic)| diagnostic.severity ==
			codespan_reporting::diagnostic::Severity::Error)
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
