use lsp_server::{Connection, Message, Notification};
use lsp_types::*;
use lsp_types::notification::{self, *};
use lsp_types::request::*;

use crate::query::Context;

use super::{Dispatch, RequestDispatch, Scene};

pub fn server() -> crate::GenericResult {
	let (connection, threads) = Connection::stdio();
	let mut capabilities = ServerCapabilities::default();

	// TODO: incremental text edits
	let kind = lsp_types::TextDocumentSyncKind::Full;
	let kind = lsp_types::TextDocumentSyncCapability::Kind(kind);
	capabilities.text_document_sync = Some(kind);

	let semantic = super::semantic_tokens_options().into();
	capabilities.semantic_tokens_provider = Some(semantic);
	capabilities.definition_provider = Some(true);

	let capabilities = serde_json::to_value(&capabilities).unwrap();
	let parameters = connection.initialize(capabilities)?;
	serve(&connection, parameters)?;
	Ok(threads.join()?)
}

fn serve(connection: &Connection, parameters: serde_json::Value) -> crate::GenericResult {
	let parameters: InitializeParams = serde_json::from_value(parameters).unwrap();

	// TODO: start single file mode if root path is invalid
	let workspace = parameters.root_uri.unwrap().to_file_path().unwrap();

	// TODO: dynamically load root file
	let root = workspace.join("Main.lc");

	let ctx = &Context::new(root);
	let scene = &mut Scene::new(ctx, connection);
	Ok(for message in &connection.receiver {
		match message {
			Message::Request(packet) => {
				if connection.handle_shutdown(&packet)? { return Ok(()); }
				RequestDispatch::new(scene, packet)
					.on::<GotoDefinition, _>(super::definition)
					.on::<SemanticTokensRequest, _>(super::semantic_tokens)
					.finish();
			}
			Message::Response(_response) => (),
			Message::Notification(packet) => Dispatch::new(scene, packet)
				.on(notification::<DidOpenTextDocument>, super::open_text_document)
				.on(notification::<DidChangeTextDocument>, super::change_text_document)
				.on(notification::<DidCloseTextDocument>, super::close_text_document)
				.on(notification::<DidChangeWatchedFiles>, super::change_watched_files)
				.finish(),
		}
	})
}

fn notification<E>(event: Notification) -> Result<E::Params, Notification>
	where E: notification::Notification {
	event.extract(E::METHOD)
}