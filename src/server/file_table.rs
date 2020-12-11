use std::sync::Arc;

use crate::FilePath;
use crate::node::Path;
use crate::parse::{ModuleLocation, SymbolTable};
use crate::query::QScope;

type FileTableEntry = (FilePath, Arc<Path>, Arc<FileTable>);

#[derive(Debug, Default)]
pub struct FileTable {
	table: Vec<FileTableEntry>,
}

impl FileTable {
	fn find(&self, file: &FilePath, base: Arc<Path>,
			paths: &mut Vec<Arc<Path>>) {
		for (path, segment, table) in &self.table {
			let module = base.append(segment);
			table.find(file, module.clone(), paths);
			(path == file).then(|| paths.push(module));
		}
	}
}

/// Returns all module paths associated with a
/// source file. The root path present in the
/// context must be in its canonical form.
pub fn file_modules(scope: QScope, file: &FilePath)
					-> crate::Result<Vec<Arc<Path>>> {
	if file == &scope.ctx.root {
		Ok(vec![Arc::new(Path::Root)])
	} else {
		let mut paths = Vec::new();
		let table = file_table(scope)?;
		let base = Arc::new(Path::Root);
		table.find(file, base, &mut paths);
		Ok(paths)
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
	symbols.modules.iter().map(|(name, (_, module))| {
		let segment = segment.push(name.clone());
		let path = path.push(name.clone());
		crate::Result::Ok(match module {
			ModuleLocation::Inline(symbols) =>
				load_symbols(scope, table, path, segment, symbols),
			ModuleLocation::External(file) => {
				let external = load_table(scope, path, file)?;
				let file = crate::source::canonicalize(scope, file)?;
				table.table.push((file, segment, external));
			}
		})
	}).for_each(drop);
}
