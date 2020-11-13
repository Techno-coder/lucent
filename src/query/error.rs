use codespan::FileId;
use codespan_reporting::diagnostic;

use super::{ESpan, MScope, QScope, QueryError, Span};

type Diagnostic = diagnostic::Diagnostic<FileId>;
type DiagnosticLabel = diagnostic::Label<FileId>;

/// An error with source locations.
#[derive(Debug, Clone)]
pub struct E {
	pub error: Diagnostic,
	pub labels: Vec<Label>,
}

impl E {
	fn new(kind: diagnostic::Severity) -> Self {
		let error = Diagnostic::new(kind);
		Self { error, labels: vec![] }
	}

	pub fn error() -> Self {
		Self::new(diagnostic::Severity::Error)
	}

	pub fn message(mut self, message: impl Into<String>) -> Self {
		self.error.message = message.into();
		self
	}

	pub fn label(mut self, label: Label) -> Self {
		self.labels.push(label);
		self
	}

	pub fn note(mut self, note: impl Into<String>) -> Self {
		self.error.notes.push(note.into());
		self
	}

	pub fn lift(mut self, scope: QScope) -> Diagnostic {
		self.error.labels.extend(self.labels.into_iter()
			.flat_map(|label| label.lift(scope)));
		self.error
	}
}

impl E {
	/// Adds this error to the query.
	pub fn emit(self, scope: MScope) {
		scope.emit(self);
	}

	/// Adds this error to the query.
	/// Returns `QueryError` for convenience.
	pub fn to(self, scope: MScope) -> QueryError {
		self.emit(scope);
		QueryError::Failure
	}

	/// Adds this error to the query.
	/// Returns `crate::Result` for convenience.
	pub fn result<T>(self, scope: MScope) -> crate::Result<T> {
		Err(self.to(scope))
	}
}

#[derive(Debug, Clone)]
pub struct Label {
	pub style: diagnostic::LabelStyle,
	pub message: String,
	pub span: ESpan,
}

impl Label {
	pub fn new(style: diagnostic::LabelStyle, span: ESpan) -> Self {
		Self { style, message: String::new(), span }
	}

	pub fn message(mut self, message: impl Into<String>) -> Self {
		self.message = message.into();
		self
	}

	pub fn lift(self, scope: QScope) -> Option<DiagnosticLabel> {
		let Span(span) = self.span.lift(scope);
		let (file, span) = span?;
		let label = DiagnosticLabel::new(self.style, file, span);
		Some(label.with_message(self.message))
	}
}
