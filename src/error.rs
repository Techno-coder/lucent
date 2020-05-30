use codespan::FileId;
use codespan_reporting::diagnostic;

#[derive(Debug)]
pub struct Diagnostic(pub diagnostic::Diagnostic<FileId>);

impl Diagnostic {
	pub fn error() -> Self {
		Self(diagnostic::Diagnostic::error())
	}

	pub fn message(mut self, message: impl Into<String>) -> Self {
		let Self(diagnostic) = &mut self;
		diagnostic.message = message.into();
		self
	}

	pub fn label(mut self, label: diagnostic::Label<FileId>) -> Self {
		let Self(diagnostic) = &mut self;
		diagnostic.labels.push(label);
		self
	}
}
