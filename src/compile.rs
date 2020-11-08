use crate::FilePath;
use crate::query::{Context, Scope, Span};

pub fn compile(path: FilePath) {
	let ctx = Context::new(path);
	let mut scope = Scope::root(&ctx, None);
	let scope = &mut scope.span(Span::internal());

	let path = crate::node::Path(vec![crate::node::Identifier("Loader".to_owned())]);
	let symbols = crate::parse::symbols(scope, path);
	println!("{:#?}", symbols);
	println!("{:#?}", ctx);
}
