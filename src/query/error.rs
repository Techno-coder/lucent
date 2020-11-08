use codespan::FileId;
use codespan_reporting::diagnostic;

use super::ESpan;

/// An error with source locations.
#[derive(Debug)]
pub struct E {
	pub error: diagnostic::Diagnostic<FileId>,
	pub labels: Vec<Label>,
}

impl E {
	fn new(kind: diagnostic::Severity) -> Self {
		let error = diagnostic::Diagnostic::new(kind);
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
}
