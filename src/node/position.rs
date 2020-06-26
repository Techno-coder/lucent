use crate::context::Context;
use crate::node::{Item, Path};

use super::Symbol;

#[derive(Debug, Default, Clone)]
pub struct Position {
	pub parent: Option<Path>,
	pub previous: Option<Symbol>,
}

pub fn positions(context: &Context) {
	let mut stack = Vec::new();
	stack.push(Position::default());
	let items = context.items.read();
	items.iter().for_each(|item| match item {
		Item::Symbol(symbol) => {
			if let Some(module) = &stack.last().unwrap().parent {
				let module = &mut context.modules.get_mut(module).unwrap();
				if module.first.is_none() { module.first = Some(symbol.clone()); }
				module.last = Some(symbol.clone());
			}

			match symbol {
				Symbol::Function(_) | Symbol::Variable(_) => {
					let position = stack.last().unwrap().clone();
					context.positions.write().insert(symbol.clone(), position);
					stack.last_mut().unwrap().previous = Some(symbol.clone());
				}
				Symbol::Module(path) => {
					let parent = Some(path.clone());
					stack.push(Position { parent, previous: None });
				}
			}
		}
		Item::ModuleEnd => {
			let path = Symbol::Module(stack.pop().unwrap().parent.unwrap());
			context.positions.write().insert(path.clone(), stack.last().unwrap().clone());
			stack.last_mut().unwrap().previous = Some(path);
		}
	})
}

