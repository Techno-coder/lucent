use std::collections::HashMap;
use std::sync::Arc;

use crate::node::{GlobalAnnotations, HGlobalAnnotation, Identifier, Path, VStore};
use crate::query::{E, QScope};

use super::{Node, PSource, Scene, TreeNode, TSpan};

pub fn global_annotations(scope: QScope) -> crate::Result<Arc<GlobalAnnotations>> {
	scope.ctx.globals.inherit(scope, (), |scope| {
		let source = crate::source::source(scope, &scope.ctx.root)?;
		let tree = super::parser().parse(source.text.as_bytes(), None).unwrap();
		let root = TreeNode::new(tree.root_node(), PSource::new(&source));
		let table = &crate::parse::item_table(scope, &Path::Root)?;

		let inclusions = &table.inclusions;
		let mut annotations = HashMap::new();
		for node in root.children() {
			let mut values = VStore::default();
			if node.kind() != "global_annotation" { continue; }
			let scene = &mut Scene { inclusions, values: &mut values };

			let identifier = node.field(scope, "name")?;
			let name = Identifier(node.text().into());
			let span = identifier.span();

			let value = node.field(scope, "value")?;
			let value = TSpan::scope(span, |span|
				super::valued(scope, scene, span, value));
			let annotation = HGlobalAnnotation { values, value, span };

			match annotations.get(&name) {
				None => annotations.insert(name, annotation).unwrap_none(),
				Some(annotation) => E::error().message("duplicate annotation")
					.label(span.label()).label(annotation.span.other()).emit(scope),
			}
		}

		Ok(Arc::new(annotations))
	})
}
