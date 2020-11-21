use std::ops::Range;

use codespan_lsp::byte_span_to_range;
use codespan_reporting::diagnostic::{LabelStyle, Severity};
use lsp_types::*;
use lsp_types::notification::PublishDiagnostics;

use crate::FilePath;
use crate::query::{Context, Diagnostic, Span};
use crate::source::File;

use super::MScene;

pub fn diagnostics(scene: MScene, path: &FilePath) -> crate::Result<()> {
	let mut scope = scene.scope();
	let queries = &mut scope.span(Span::internal());
	let module = &super::file_module(queries, path)?;
	let _ = crate::parse::item_table(queries, module)?;
	let errors = scope.ctx.errors(scope).into_iter();

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

fn encode(ctx: &Context, mut diagnostic: Diagnostic) -> Option<lsp_types::Diagnostic> {
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
	let target = diagnostic.labels.iter().position(|label|
		label.style == LabelStyle::Primary).unwrap_or(0);
	let target = diagnostic.labels.remove(target);
	let range = byte_span_to_range(&ctx.files,
		target.file_id, target.range).unwrap();

	let related_information = Some(diagnostic.labels.into_iter()
		.map(|label| DiagnosticRelatedInformation {
			location: file_location(ctx, label.file_id, label.range),
			message: label.message,
		}).collect());

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

