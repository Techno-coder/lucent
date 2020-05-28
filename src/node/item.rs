use std::collections::HashMap;
use std::fmt;

use crate::span::S;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Identifier(pub String);

impl fmt::Display for Identifier {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let Identifier(identifier) = self;
		write!(f, "{}", identifier)
	}
}

#[derive(Default, Clone, Hash, Eq, PartialEq)]
pub struct Path(pub Vec<Identifier>);

impl Path {
	#[must_use]
	pub fn push(&self, identifier: Identifier) -> Path {
		let Path(mut path) = self.clone();
		path.push(identifier);
		Self(path)
	}
}

impl fmt::Debug for Path {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let Path(path) = self;
		let (last, slice) = path.split_last().unwrap();
		slice.iter().try_for_each(|identifier| write!(f, "{}.", identifier))?;
		write!(f, "{}", last)
	}
}

#[derive(Debug)]
pub struct Annotation {
	pub name: S<Identifier>,
	pub value: S<super::Expression>,
}

#[derive(Debug)]
pub struct Module {
	pub annotations: Vec<Annotation>,
	pub items: Vec<Path>,
}

#[derive(Debug)]
pub struct Structure {
	pub annotations: Vec<Annotation>,
	pub fields: HashMap<Identifier, S<super::Type>>,
}
