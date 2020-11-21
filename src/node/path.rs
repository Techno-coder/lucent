use std::fmt;
use std::sync::Arc;

use crate::node::Identifier;
use crate::query::S;

/// Represents a sequence of identifiers that uniquely
/// references an item. Note that the order is reversed
/// as the outermost module is the deepest path element.
#[derive(Clone, Hash, Eq, PartialEq)]
pub enum Path { Root, Node(Arc<Path>, Identifier) }

impl Path {
	pub fn append(&self, other: &Path) -> Path {
		match other {
			Path::Root => self.clone(),
			Path::Node(parent, name) => {
				let path = Arc::new(self.append(parent));
				Path::Node(path, name.clone())
			}
		}
	}
}

impl fmt::Display for Path {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Path::Root => Ok(()),
			Path::Node(parent, name) => match parent.as_ref() {
				Path::Node(_, _) => write!(f, "{}.{}", parent, name),
				Path::Root => write!(f, "{}", name),
			}
		}
	}
}

impl fmt::Debug for Path {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self)
	}
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum HPath {
	Root(Arc<Path>),
	Node(Box<HPath>, S<Identifier>),
}

impl HPath {
	pub fn root() -> Self {
		HPath::Root(Arc::new(Path::Root))
	}

	/// Returns the first (deepest) identifier in the spanned
	/// path segment. Note that this operation is `O(n)`.
	pub fn head(&self) -> Option<S<Identifier>> {
		match self {
			HPath::Root(_) => None,
			HPath::Node(parent, name) => match parent.as_ref() {
				HPath::Root(_) => Some(name.clone()),
				_ => parent.head(),
			}
		}
	}

	pub fn parent(&self) -> Option<&HPath> {
		match self {
			HPath::Root(_) => None,
			HPath::Node(parent, _) => Some(&parent),
		}
	}

	pub fn rebase(self, root: Arc<Path>) -> HPath {
		match self {
			HPath::Root(_) => HPath::Root(root),
			HPath::Node(parent, name) => {
				let parent = Box::new(parent.rebase(root));
				HPath::Node(parent, name)
			}
		}
	}

	pub fn path(&self) -> Arc<Path> {
		match self {
			HPath::Root(root) => root.clone(),
			HPath::Node(parent, name) => {
				let name = name.node.clone();
				Arc::new(Path::Node(parent.path(), name))
			}
		}
	}
}

impl fmt::Display for HPath {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.path())
	}
}

impl fmt::Debug for HPath {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self)
	}
}
