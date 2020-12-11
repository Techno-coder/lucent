use lsp_server::{Connection, Message, Notification};
use lsp_types::*;
use lsp_types::notification::{self, *};
use lsp_types::request::*;
use parking_lot::RwLock;

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
	super::register_watchers(connection);

	let mut scene = Scene::new(connection, parameters.root_uri
		.map(|path| path.to_file_path().unwrap()));
	super::populate_targets(&mut scene);
	let scene = &RwLock::new(scene);

	Ok(connection.receiver.iter().for_each(|message| match message {
		Message::Request(packet) => {
			if connection.handle_shutdown(&packet).unwrap() { return; }
			RequestDispatch::new(&scene.read(), packet)
				.on::<SemanticTokensRequest, _>(super::semantic_tokens)
				.on::<GotoDefinition, _>(super::definition)
				.finish();
		}
		Message::Response(_) => (),
		Message::Notification(packet) => Dispatch::new(scene, packet)
			.on(notification::<DidOpenTextDocument>, super::open_text_document)
			.on(notification::<DidChangeTextDocument>, super::change_text_document)
			.on(notification::<DidCloseTextDocument>, super::close_text_document)
			.on(notification::<DidChangeWatchedFiles>, super::change_watched_files)
			.finish(),
	}))
}

fn notification<E>(event: Notification) -> Result<E::Params, Notification>
	where E: notification::Notification {
	event.extract(E::METHOD)
}
