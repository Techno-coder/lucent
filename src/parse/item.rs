use std::collections::HashMap;

use crate::node::*;

#[derive(Debug)]
pub struct ItemTable {
	pub modules: HashMap<Path, HModule>,
	pub functions: HashMap<Path, HFunction>,
	pub statics: HashMap<Path, HStatic>,
	pub libraries: HashMap<Path, HLibrary>,
	pub load_functions: HashMap<Path, HLoadFunction>,
	pub load_statics: HashMap<Path, HLoadStatic>,
}
