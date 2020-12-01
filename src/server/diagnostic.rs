use std::ops::Range;

use codespan_lsp::byte_span_to_range;
use codespan_reporting::diagnostic::{LabelStyle, Severity};
use lsp_types::*;
use lsp_types::notification::PublishDiagnostics;

use crate::FilePath;
use crate::node::{HVariables, Value, VPath};
use crate::parse::TSpan;
use crate::query::{Context, Diagnostic, QScope, Span};
use crate::source::File;

use super::{MScene, Visitor};

pub fn diagnostics(scene: MScene, path: &FilePath) -> crate::Result<()> {
	let mut scope = scene.scope();
	let queries = &mut scope.span(Span::internal());
	let module = &super::file_module(queries, path)?;
	let symbols = &crate::parse::symbols(queries, module)?;
	let table = &crate::parse::item_table(queries, module)?;

	let ctx = queries.ctx;
	let diagnostics = &mut Diagnostics(queries);
	super::traverse(diagnostics, table, symbols);
	let errors = ctx.errors(scope).into_iter();

	let ctx = scene.ctx;
	scope!(scope, scene);
	let uri = Url::from_file_path(path).unwrap();
	let diagnostics = errors.into_iter()
		.filter_map(|error| encode(ctx, error.lift(scope)))
		.collect();

	let publish = PublishDiagnosticsParams { uri, diagnostics, version: None };
	Ok(super::send_notification::<PublishDiagnostics>(scene, publish))
}

pub fn file_location(ctx: &Context, file: File, range: Range<usize>) -> Location {
	let range = byte_span_to_range(&ctx.files, file, range).unwrap();
	let path = &ctx.files.source(file).unwrap().path;
	let path = Url::from_file_path(path).unwrap();
	Location { uri: path, range }
}

struct Diagnostics<'a, 'b>(QScope<'a, 'b, 'b>);

impl<'a, 'b> Visitor<'a, 'b> for Diagnostics<'a, 'b> {
	fn scope<'c>(&'c mut self) -> QScope<'a, 'b, 'c> {
		let Self(scope) = self;
		scope
	}

	fn value(&mut self, _: &TSpan, path: VPath,
			 _: &Value, _: Option<&HVariables>) {
		let _ = crate::inference::types(self.scope(), &path);
	}
}

fn encode(ctx: &Context, diagnostic: Diagnostic) -> Option<lsp_types::Diagnostic> {
	let mut message = diagnostic.message;
	for note in diagnostic.notes {
		message += "\n- ";
		message += &note;
	}

	let severity = Some(match diagnostic.severity {
		Severity::Error | Severity::Bug => DiagnosticSeverity::Error,
		Severity::Warning => DiagnosticSeverity::Warning,
		Severity::Note => DiagnosticSeverity::Information,
		Severity::Help => DiagnosticSeverity::Hint,
	});

	if diagnostic.labels.is_empty() { return None; }
	let first = diagnostic.labels.first().unwrap();
	let target = diagnostic.labels.iter().find(|label|
		label.style == LabelStyle::Primary).unwrap_or(first);
	let range = byte_span_to_range(&ctx.files,
		target.file_id, target.range.clone()).unwrap();

	let related_information = Some(diagnostic.labels.into_iter()
		.filter_map(|label| Some(DiagnosticRelatedInformation {
			location: file_location(ctx, label.file_id, label.range),
			message: (!label.message.is_empty()).then_some(label.message)?,
		})).collect());

	Some(lsp_types::Diagnostic {
		range,
		severity,
		code: None,
		source: None,
		message,
		related_information,
		tags: None,
	})
}

