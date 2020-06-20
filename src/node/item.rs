use std::collections::HashMap;
use std::fmt;

use crate::node::Variable;
use crate::span::S;

#[derive(Debug)]
pub enum Item {
	Symbol(Symbol),
	ModuleEnd,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Symbol {
	Module(Path),
	Variable(Path),
	Function(FunctionPath),
}

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

impl PartialEq<[&str]> for Path {
	fn eq(&self, other: &[&str]) -> bool {
		let Path(elements) = self;
		elements.len() == other.len() &&
			Iterator::zip(elements.iter(), other)
				.all(|(Identifier(left), right)| left == right)
	}
}

impl fmt::Display for Path {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let Path(path) = self;
		let (last, slice) = path.split_last().unwrap();
		slice.iter().try_for_each(|identifier| write!(f, "{}.", identifier))?;
		write!(f, "{}", last)
	}
}

impl fmt::Debug for Path {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self)
	}
}

#[derive(Debug)]
pub struct Annotation {
	pub name: S<Identifier>,
	pub value: super::Value,
}

#[derive(Debug)]
pub struct Structure {
	pub annotations: Vec<Annotation>,
	pub fields: HashMap<Identifier, S<super::Type>>,
}

#[derive(Debug)]
pub struct Static {
	pub identifier: S<Identifier>,
	pub node_type: Option<S<super::Type>>,
	pub value: Option<super::Value>,
}

pub type FunctionKind = usize;

#[derive(Debug, Default, Clone, Hash, Eq, PartialEq)]
pub struct FunctionPath(pub Path, pub FunctionKind);

#[derive(Debug)]
pub struct Function {
	pub is_root: bool,
	pub convention: Option<S<Identifier>>,
	pub annotations: Vec<super::Annotation>,
	pub parameters: Vec<S<Parameter>>,
	pub return_type: S<ReturnType>,
	pub value: super::Value,
}

#[derive(Debug, Clone)]
pub enum ReturnType {
	Register(S<Identifier>),
	Type(S<super::Type>),
}

#[derive(Debug, Clone)]
pub enum Parameter {
	Register(S<Identifier>),
	Variable(S<Variable>, S<super::Type>),
}

#[derive(Debug)]
pub struct Module {
	pub annotations: Vec<super::Annotation>,
	pub first: Option<Symbol>,
	pub last: Option<Symbol>,
}
