use std::sync::Arc;

use crate::FilePath;
use crate::node::Path;
use crate::parse::{ModuleLocation, SymbolTable};
use crate::query::{E, QScope};

type FileTableEntry = (FilePath, Arc<Path>, Arc<FileTable>);

#[derive(Debug, Default)]
pub struct FileTable {
	table: Vec<FileTableEntry>,
}

impl FileTable {
	fn find(&self, file: &FilePath) -> Option<Arc<Path>> {
		for (path, segment, table) in &self.table {
			if path == file { return Some(segment.clone()); }
			if let Some(other) = table.find(file) {
				return Some(Arc::new(segment.append(&other)));
			}
		}
		None
	}
}

/// Returns the module path associated with a
/// source file. The root path present in the
/// context must be in its canonical form.
pub fn file_module(scope: QScope, file: &FilePath)
				   -> crate::Result<Arc<Path>> {
	if file == &scope.ctx.root {
		Ok(Arc::new(Path::Root))
	} else {
		let path = file_table(scope)?.find(file);
		path.ok_or_else(|| E::error()
			.message("file absent from module tree")
			.label(scope.span.label()).to(scope))
	}
}

fn file_table(scope: QScope) -> crate::Result<Arc<FileTable>> {
	load_table(scope, Arc::new(Path::Root), &scope.ctx.root)
}

fn load_table(scope: QScope, path: Arc<Path>,
			  file: &FilePath) -> crate::Result<Arc<FileTable>> {
	let root = Arc::new(Path::Root);
	scope.ctx.file_table.inherit(scope, file.clone(), |scope| {
		let mut table = FileTable::default();
		let symbols = crate::parse::symbols(scope, &path)?;
		load_symbols(scope, &mut table, path, root, &symbols);
		Ok(Arc::new(table))
	})
}

fn load_symbols(scope: QScope, table: &mut FileTable, path: Arc<Path>,
				segment: Arc<Path>, symbols: &SymbolTable) {
	symbols.modules.iter().map(|(name, (_, module))| crate::Result::Ok({
		let segment = Arc::new(Path::Node(segment.clone(), name.clone()));
		let path = Arc::new(Path::Node(path.clone(), name.clone()));
		match module {
			ModuleLocation::Inline(symbols) =>
				load_symbols(scope, table, path, segment, symbols),
			ModuleLocation::External(file) => {
				let external = load_table(scope, path, file)?;
				let file = crate::source::canonicalize(scope, file)?;
				table.table.push((file, segment, external));
			}
		}
	})).last();
}
