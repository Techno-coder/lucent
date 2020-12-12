use crate::FilePath;
use crate::query::{Context, Scope, Span};

pub fn compile(path: FilePath) -> std::io::Result<()> {
	use crate::node::*;
	use std::sync::Arc;

	let ctx = Context::new(path);
	let mut scope = Scope::root(&ctx, None);
	let queries = &mut scope.span(Span::internal());

	let path = Arc::new(Path::Root);
	let path = path.push(Identifier("Main".into()));
	let path = path.push(Identifier("fibonacci".into()));

	let path = FLocal(FPath(path, 0));
	let _ = crate::inference::function(queries, &path).unwrap();

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
	errors.try_for_each(|error| term::emit(writer, configuration,
		&scope.ctx.files, &error.lift(scope)))
}
