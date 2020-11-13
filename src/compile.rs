use crate::FilePath;
use crate::query::{Context, Scope, Span};

pub fn compile(path: FilePath) -> std::io::Result<()> {
	use crate::node::*;
	use std::sync::Arc;

	let ctx = Context::new(path);
	let mut scope = Scope::root(&ctx, None);
	let query_scope = &mut scope.span(Span::internal());

	let path = Path::Root;
	let path = Path::Node(Arc::new(path), Identifier("Main".into()));
	let path = Path::Node(Arc::new(path), Identifier("fibonacci".into()));
	let functions = crate::parse::functions(query_scope, &path);
	println!("{:#?}", functions);
	display_diagnostics(scope)
}

fn display_diagnostics(root: Scope) -> std::io::Result<()> {
	use codespan_reporting::term;
	let configuration = &term::Config::default();
	let colors = term::termcolor::ColorChoice::Auto;
	let writer = &mut term::termcolor::StandardStream::stderr(colors);

	let mut scope = Scope::root(root.ctx, None);
	let scope = &mut scope.span(Span::internal());
	let mut errors = scope.ctx.errors(root).into_iter();
	scope.ctx.source.files(|files| errors.try_for_each(|error|
		term::emit(writer, configuration, files, &error.lift(scope))))
}
