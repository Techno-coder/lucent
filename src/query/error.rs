use codespan_reporting::diagnostic;

use crate::source::File;

use super::{EScope, ESpan, QScope, QueryError, Span};

pub type Diagnostic = diagnostic::Diagnostic<File>;
type DiagnosticLabel = diagnostic::Label<File>;

/// An error with source locations.
#[derive(Debug, Clone)]
pub struct E<S = ESpan> {
	pub error: Diagnostic,
	pub labels: Vec<Label<S>>,
}

impl<S> E<S> {
	fn new(kind: diagnostic::Severity) -> Self {
		let error = Diagnostic::new(kind);
		Self { error, labels: vec![] }
	}

	pub fn error() -> Self {
		Self::new(diagnostic::Severity::Error)
	}

	pub fn message(mut self, message: impl ToString) -> Self {
		self.error.message = message.to_string();
		self
	}

	pub fn label(mut self, label: Label<S>) -> Self {
		self.labels.push(label);
		self
	}

	pub fn note(mut self, note: impl ToString) -> Self {
		self.error.notes.push(note.to_string());
		self
	}
}

impl<S> E<S> {
	/// Adds this error to the query.
	pub fn emit(self, scope: &mut impl EScope<S>) {
		scope.emit(self);
	}

	/// Adds this error to the query.
	/// Returns `QueryError` for convenience.
	pub fn to(self, scope: &mut impl EScope<S>) -> QueryError {
		self.emit(scope);
		QueryError::Failure
	}

	/// Adds this error to the query.
	/// Returns `crate::Result` for convenience.
	pub fn result<T>(self, scope: &mut impl EScope<S>) -> crate::Result<T> {
		Err(self.to(scope))
	}
}

impl E {
	pub fn lift(mut self, scope: QScope) -> Diagnostic {
		self.error.labels.extend(self.labels.into_iter()
			.flat_map(|label| label.lift(scope)));
		self.error
	}
}

#[derive(Debug, Clone)]
pub struct Label<S> {
	pub style: diagnostic::LabelStyle,
	pub message: String,
	pub span: S,
}

impl<S> Label<S> {
	pub fn new(style: diagnostic::LabelStyle, span: S) -> Self {
		Self { style, message: String::new(), span }
	}

	pub fn message(mut self, message: impl ToString) -> Self {
		self.message = message.to_string();
		self
	}
}

impl Label<ESpan> {
	pub fn lift(self, scope: QScope) -> Option<DiagnosticLabel> {
		let Span(span) = self.span.lift(scope);
		let (file, span) = span.map(|(file, (start, end))| (file, start..end))?;
		let label = DiagnosticLabel::new(self.style, file, span);
		Some(label.with_message(self.message))
	}
}
