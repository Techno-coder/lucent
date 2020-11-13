use std::collections::HashMap;
use std::sync::Arc;

use crate::node::{Identifier, Path};
use crate::query::{E, MScope, QScope, Span};

use super::SymbolTable;

#[derive(Debug)]
pub struct Inclusions {
	frames: Vec<InclusionFrame>,
}

impl Inclusions {
	pub fn new(root: Path) -> Self {
		let mut frame = InclusionFrame::new(root);
		frame.wildcard.push(Path::Root);
		Self { frames: vec![frame] }
	}

	pub fn scope<F, R>(&mut self, module: Identifier, function: F) -> R
		where F: FnOnce(&mut Self) -> R {
		let frame = self.frames.last_mut().unwrap();
		let parent = frame.wildcard.first().unwrap().clone();
		let module = Path::Node(Arc::new(parent), module);
		self.frames.push(InclusionFrame::new(module));
		let value = function(self);
		self.frames.pop();
		value
	}

	pub fn wildcard(&mut self, path: Path) {
		self.frames.last_mut().unwrap().wildcard.push(path);
	}

	pub fn specific(&mut self, scope: MScope, name: Option<Identifier>,
					span: Span, target: Path) -> crate::Result<()> {
		let specific = &mut self.frames.last_mut().unwrap().specific;
		let name = name.map(Ok).unwrap_or_else(|| match &target {
			Path::Node(_, name) => Ok(name.clone()),
			Path::Root => E::error()
				.message("cannot import empty path")
				.label(span.label()).result(scope),
		})?;

		match specific.get(&name) {
			Some((other, _)) => E::error().message("conflicting imports")
				.label(other.label()).label(span.other()).result(scope),
			None => Ok(specific.insert(name, (span, target)).unwrap_none()),
		}
	}
}

impl Inclusions {
	pub fn structure(&self, scope: QScope, path: &Path) -> Option<Path> {
		self.resolve(scope, |table, name| table.structures.contains_key(name), path)
	}

	pub fn statics(&self, scope: QScope, path: &Path) -> Option<Path> {
		self.resolve(scope, |table, name| table.statics.contains_key(name), path)
	}

	pub fn function(&self, scope: QScope, path: &Path) -> Option<Path> {
		self.resolve(scope, |table, name| table.functions.contains_key(name), path)
	}

	fn resolve<F>(&self, scope: QScope, predicate: F,
				  path: &Path) -> Option<Path>
		where F: Fn(&SymbolTable, &Identifier) -> bool {
		let name = &path.head().unwrap();
		for frame in self.frames.iter().rev() {
			// Find import directly matching identifier.
			if let Some((_, target)) = frame.specific.get(name) {
				let tail = &path.tail().unwrap();
				return Some(target.append(tail));
			}

			// Traverse all possible wildcard imports.
			for base in &frame.wildcard {
				let path = base.append(path);
				if let Path::Node(module, name) = &path {
					if let Some(table) = super::try_symbols(scope, module) {
						if predicate(&table, name) { return Some(path); }
					}
				} else {
					panic!("invalid symbol path");
				}
			}
		}

		// No valid import found.
		None
	}
}

#[derive(Debug)]
pub struct InclusionFrame {
	specific: HashMap<Identifier, (Span, Path)>,
	wildcard: Vec<Path>,
}

impl InclusionFrame {
	fn new(module: Path) -> Self {
		InclusionFrame {
			specific: HashMap::new(),
			wildcard: vec![module],
		}
	}
}
