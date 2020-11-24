use std::lazy::SyncLazy;

use codespan_lsp::byte_index_to_position as position;
use lsp_types::*;
use tree_sitter_highlight::{Highlight, Highlighter, HighlightEvent};
use tree_sitter_highlight::HighlightConfiguration as Configuration;

use crate::node::{HPath, HType, HValue, HVariables, Identifier, Path, Variable};
use crate::parse::TSpan;
use crate::query::{ISpan, QScope, S, Span};

use super::{MScene, ReferenceVisitor};

macro_rules! root { () => { env!("CARGO_MANIFEST_DIR") }; }
macro_rules! path { () => { "/tree-sitter-lucent/queries/highlights.scm" }; }
static HIGHLIGHTS: &str = include_str!(concat!(root!(), path!()));

macro_rules! tokens {
    ($($name:ident $target:expr,)+) => {
    	#[allow(dead_code)]
    	#[repr(u32)] enum Token { $($name,)+ }
    	static TOKENS: SyncLazy<Vec<String>> =
			SyncLazy::new(|| vec![$($target.to_owned(),)+]);
    };
}

tokens![
	Keyword "keyword",
	Operator "operator",
	Punctuation "punctuation",
	String "string",
	Number "number",
	Attribute "attribute",
	Property "property",
	Variable "variable",
	Constant "constant",
	Comment "comment",
	Function "function",
	Global "global",
	Module "module",
	Type "type",
];

pub fn semantic_tokens_options() -> SemanticTokensOptions {
	let token_types = TOKENS.iter().map(|token| SemanticTokenType::new(token)).collect();
	let legend = SemanticTokensLegend { token_types, token_modifiers: vec![] };
	let document_provider = Some(SemanticTokensDocumentProvider::Bool(true));
	SemanticTokensOptions { legend, document_provider, ..Default::default() }
}

pub fn semantic_tokens(scene: MScene, request: SemanticTokensParams)
					   -> crate::Result<Option<SemanticTokensResult>> {
	let language = crate::parse::language();
	let mut highlight = Configuration::new(language,
		HIGHLIGHTS, "", "").unwrap();
	highlight.configure(&TOKENS);

	scope!(scope, scene);
	let path = request.text_document.uri.to_file_path().unwrap();
	let file = crate::source::file(scope, &path)?;
	let source = scope.ctx.files.source(file).unwrap();
	let highlights = &mut Highlighter::new();
	let highlights = highlights.highlight(&highlight,
		source.text.as_bytes(), None, |_| None).unwrap();

	// Add base highlighting tokens.
	let mut tokens = Vec::new();
	let mut token: Option<u32> = None;
	for event in highlights {
		match event.unwrap() {
			HighlightEvent::Source { start, end } => if let Some(token) = token {
				let position = position(&scope.ctx.files, file, start).unwrap();
				tokens.push((position, (end - start) as u32, token));
			}
			// Highlight spans may not overlap so
			// highlights do not need to be scoped.
			HighlightEvent::HighlightEnd => token = None,
			HighlightEvent::HighlightStart(index) => {
				let Highlight(index) = index;
				token = Some(index as u32)
			}
		}
	}

	// Add path highlighting tokens.
	let module = &super::file_module(scope, &path)?;
	let symbols = &crate::parse::symbols(scope, module)?;
	let table = &crate::parse::item_table(scope, module)?;
	let visitor = &mut Tokens { scope, tokens: &mut tokens };
	super::traverse(visitor, table, symbols);

	// Differentiate token positions.
	tokens.sort();
	let mut data = Vec::new();
	let mut last_position = Position::new(0, 0);
	for (position, length, token_type) in tokens {
		let delta_line = (position.line - last_position.line) as u32;
		let delta_start = match delta_line == 0 {
			true => position.character - last_position.character,
			false => position.character,
		} as u32;

		last_position = position;
		let token_modifiers_bitset = 0;
		data.push(SemanticToken {
			delta_line,
			delta_start,
			length,
			token_type,
			token_modifiers_bitset,
		});
	}

	let result_id = None;
	let tokens = SemanticTokens { result_id, data };
	Ok(Some(tokens.into()))
}

struct Tokens<'a> {
	scope: QScope<'a, 'a, 'a>,
	tokens: &'a mut Vec<(Position, u32, u32)>,
}

impl<'a> Tokens<'a> {
	fn token(&mut self, base: &TSpan, span: &ISpan, token: Token) {
		if let Span(Some((file, (start, end)))) = TSpan::lift(base, *span) {
			let position = position(&self.scope.ctx.files, file, start).unwrap();
			self.tokens.push((position, (end - start) as u32, token as u32));
		}
	}

	fn item(&mut self, base: &TSpan, path: &HPath, token: Token) {
		if let HPath::Node(module, name) = path {
			self.token(base, &name.span, token);
			self.module(base, module);
		}
	}
}

impl<'a> ReferenceVisitor<'a> for Tokens<'a> {
	fn scope<'b>(&'b mut self) -> QScope<'a, 'a, 'b> { self.scope }

	fn kind(&mut self, base: &TSpan, kind: &S<HType>) {
		match kind.node {
			HType::Pointer(_) => return,
			HType::Structure(_) | HType::Function(_) => return,
			HType::Array(_, _) | HType::Slice(_) => return,
			_ => self.token(base, &kind.span, Token::Type),
		}
	}

	fn variable(&mut self, base: &TSpan, _: &HValue,
				_: Option<&HVariables>, _: &Variable, span: &ISpan) {
		self.token(base, span, Token::Variable);
	}

	fn field(&mut self, base: &TSpan, structure: &Path,
			 name: &Identifier, span: &ISpan) {
		let data = crate::parse::structure(self.scope, structure);
		if data.unwrap().fields.contains_key(name) {
			self.token(base, span, Token::Property);
		}
	}

	fn function(&mut self, base: &TSpan, path: &HPath, _: usize) {
		self.item(base, path, Token::Function);
	}

	fn structure(&mut self, base: &TSpan, path: &HPath) {
		self.item(base, path, Token::Type);
	}

	fn statics(&mut self, base: &TSpan, path: &HPath) {
		self.item(base, path, Token::Global);
	}

	fn library(&mut self, base: &TSpan, path: &HPath) {
		self.item(base, path, Token::Module);
	}

	fn module(&mut self, base: &TSpan, path: &HPath) {
		self.item(base, path, Token::Module);
	}

	fn path(&mut self, base: &TSpan, path: &HPath) {
		if let HPath::Node(module, name) = path {
			let mut tokens: Vec<Token> = Vec::new();
			let symbols = crate::parse::symbols(self.scope, &module.path()).unwrap();
			tokens.extend(symbols.statics.contains_key(&name.node).then(|| Token::Global));
			tokens.extend(symbols.structures.contains_key(&name.node).then(|| Token::Type));
			tokens.extend(symbols.functions.contains_key(&name.node).then(|| Token::Function));
			tokens.extend(symbols.libraries.contains_key(&name.node).then(|| Token::Module));
			tokens.extend(symbols.modules.contains_key(&name.node).then(|| Token::Module));

			match tokens.len() == 1 {
				true => self.item(base, path, tokens.remove(0)),
				false => self.module(base, module),
			}
		}
	}
}
