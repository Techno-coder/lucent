use std::collections::HashMap;
use std::sync::Arc;

use crate::node::{HPath, Identifier, Path};
use crate::parse::{SymbolTable, TSpan};
use crate::query::{E, QScope};

#[derive(Debug)]
pub struct Inclusions {
	pub parent: Option<Arc<Inclusions>>,
	pub specific: HashMap<Identifier, HPath>,
	pub wildcard: Vec<HPath>,
	pub module: Arc<Path>,
}

impl Inclusions {
	fn new(parent: Option<Arc<Inclusions>>,
		   module: Arc<Path>) -> Self {
		Inclusions {
			parent,
			specific: HashMap::new(),
			wildcard: vec![],
			module,
		}
	}

	pub fn root(module: Arc<Path>) -> Self {
		Self::new(None, module)
	}

	pub fn scope(self: &Arc<Self>, module: Identifier) -> Self {
		let parent = Some(self.clone());
		let module = self.module.clone().push(module);
		Self::new(parent, module)
	}

	pub fn wildcard(self: &mut Arc<Self>, scope: QScope,
					path: HPath) -> crate::Result<()> {
		crate::parse::symbols(scope, &path.path())?;
		Ok(self.modify(scope)?.wildcard.push(path))
	}

	pub fn specific(self: &mut Arc<Self>, scope: QScope,
					base: &TSpan, target: HPath) -> crate::Result<()> {
		let name = match &target {
			HPath::Root(_) => return E::error()
				.message("cannot import empty path")
				.label(scope.span.label()).result(scope),
			HPath::Node(module, name) => {
				crate::parse::symbols(scope, &module.path())?;
				name.clone()
			}
		};

		let inclusions = self.modify(scope)?;
		let specific = &mut inclusions.specific;
		match specific.get(&name.node) {
			None => Ok(specific.insert(name.node, target).unwrap_none()),
			Some(HPath::Root(_)) => panic!("empty path inclusion"),
			Some(HPath::Node(_, other)) => E::error()
				.label(TSpan::lift(base, other.span).label())
				.label(TSpan::lift(base, name.span).other())
				.message("conflicting imports").result(scope),
		}
	}

	fn modify<'a>(self: &'a mut Arc<Self>, scope: QScope) -> crate::Result<&'a mut Self> {
		Arc::get_mut(self).ok_or_else(|| E::error()
			.message("imports must appear before nested modules")
			.label(scope.span.label()).to(scope))
	}
}

impl Inclusions {
	pub fn structure(&self, scope: QScope, base: &TSpan,
					 path: &HPath) -> crate::Result<Option<HPath>> {
		self.resolve(scope, |table, name| table.structures
			.contains_key(name), base, path)
	}

	pub fn statics(&self, scope: QScope, base: &TSpan,
				   path: &HPath) -> crate::Result<Option<HPath>> {
		self.resolve(scope, |table, name| table.statics
			.contains_key(name), base, path)
	}

	pub fn function(&self, scope: QScope, base: &TSpan,
					path: &HPath) -> crate::Result<Option<HPath>> {
		self.resolve(scope, |table, name| table.functions
			.contains_key(name), base, path)
	}

	pub fn library(&self, scope: QScope, base: &TSpan,
				   path: &HPath) -> crate::Result<Option<HPath>> {
		self.resolve(scope, |table, name| table.libraries
			.contains_key(name), base, path)
	}

	fn resolve<F>(&self, scope: QScope, predicate: F, base: &TSpan,
				  path: &HPath) -> crate::Result<Option<HPath>>
		where F: Fn(&SymbolTable, &Identifier) -> bool {
		let head = &path.head().unwrap();

		// Find import directly matching identifier.
		if let Some(target) = self.specific.get(&head.node) {
			let prefix = target.parent().unwrap();
			let prefix = HPath::Root(prefix.path()).path();
			return Ok(Some(path.clone().rebase(prefix)));
		}

		let valid = &mut |path: &HPath| {
			if let HPath::Node(module, name) = &path {
				let table = super::try_symbols(scope, &module.path());
				table.map(|table| predicate(&table, &name.node))
			} else {
				panic!("invalid symbol path");
			}.unwrap_or(false)
		};

		// Traverse all possible wildcard imports.
		let mut candidate: Option<HPath> = None;
		for prefix in &self.wildcard {
			let path = path.clone().rebase(prefix.path());
			if !valid(&path) { continue; }

			if let Some(other) = &candidate {
				let other_span = other.head().unwrap().span;
				return E::error().message("conflicting resolutions")
					.label(TSpan::lift(base, other_span).other())
					.label(TSpan::lift(base, head.span).other())
					.label(scope.span.label()).result(scope);
			}

			candidate = Some(path);
		}

		// Implicit module scope item.
		if candidate.is_some() { return Ok(candidate); }
		let candidate = path.clone().rebase(self.module.clone());
		match valid(&candidate) {
			false => Ok(self.parent.as_ref().map(|parent|
				parent.resolve(scope, predicate, base, path))
				.transpose()?.flatten()),
			true => Ok(Some(candidate)),
		}
	}
}
