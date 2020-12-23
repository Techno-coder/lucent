use std::ops::Range;

use codespan_lsp::byte_span_to_range;
use codespan_reporting::diagnostic::{LabelStyle, Severity};
use lsp_types::*;
use lsp_types::notification::PublishDiagnostics;

use crate::FilePath;
use crate::node::{FLocal, HFunction, Value, VPath};
use crate::parse::TSpan;
use crate::query::{Context, Diagnostic, QScope, Span};
use crate::source::File;

use super::{RScene, Visitor};

pub fn diagnostics(scene: RScene, file: &FilePath) -> crate::Result<()> {
	let mut diagnostics = Vec::new();
	let uri = Url::from_file_path(file).unwrap();
	for (mut scope, path) in scene.modules(file) {
		let queries = &mut scope.span(Span::internal());
		let symbols = &crate::parse::symbols(queries, &path)?;
		let table = &crate::parse::item_table(queries, &path)?;

		let ctx = queries.ctx;
		let visitor = &mut Diagnostics(queries);
		super::traverse(visitor, table, symbols);
		let errors = ctx.errors(scope).into_iter();

		let scope = &mut scene.scope(ctx);
		let scope = &mut scope.span(Span::internal());
		diagnostics.extend(errors.into_iter().filter_map(|error|
			encode(ctx, error.lift(scope))));
	}

	// Sort and remove duplicate diagnostics.
	// Duplicates arise from multiple targets.
	diagnostics.sort_by_cached_key(|value|
		serde_json::to_vec(value).unwrap());
	diagnostics.dedup();

	let publish = PublishDiagnosticsParams { uri, diagnostics, version: None };
	Ok(super::send_notification::<PublishDiagnostics>(scene, publish))
}

pub fn file_location(ctx: &Context, file: File, range: Range<usize>) -> Location {
	let range = byte_span_to_range(&ctx.files, file, range).unwrap();
	let path = &ctx.files.source(file).unwrap().path;
	let path = Url::from_file_path(path).unwrap();
	Location { uri: path, range }
}

struct Diagnostics<'a, 'b, 'c>(QScope<'a, 'b, 'c>);

impl<'a, 'b, 'c> Visitor<'a, 'b, 'c> for Diagnostics<'a, 'b, 'c> {
	fn scope<'d>(&'d mut self) -> QScope<'a, 'b, 'd> {
		let Self(scope) = self;
		scope
	}

	fn function(&mut self, _: &TSpan, path: &FLocal, _: &HFunction) {
		let _ = crate::lower::function(self.scope(), &path);
	}

	fn value(&mut self, _: &TSpan, path: VPath, _: &Value) {
		let _ = crate::lower::low(self.scope(), &path);
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

