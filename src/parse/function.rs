use tree_sitter::Node;

use crate::context::Context;
use crate::node::{Parameter, Path, Type};
use crate::span::S;

use super::{Source, Symbols};

pub fn function(context: &Context, symbols: &mut Symbols,
				source: &Source, path: &Path, node: Node) {
	let identifier = super::identifier(source, node);
	unimplemented!()
}

fn parameters(symbols: &mut Symbols, source: &Source, node: Node) -> Vec<S<Parameter>> {
	let cursor = &mut node.walk();
	node.children_by_field_name("parameter", cursor)
		.map(|node| S::create(match node.kind() {
			"register" => Parameter::Register(super::identifier(source, node)),
			"parameter" => {
				let identifier = super::field_identifier(source, node);
				let node_type = node_type(symbols, source, node);
				Parameter::Variable(identifier, node_type)
			}
			other => panic!("invalid parameter type: {}", other),
		}, node.byte_range(), source.file)).collect()
}

fn node_type(_symbols: &mut Symbols, source: &Source, node: Node) -> S<Type> {
	unimplemented!()
}
