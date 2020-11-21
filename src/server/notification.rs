use lsp_types::*;

use crate::FilePath;

use super::MScene;

pub fn send_notification<E>(scene: MScene, parameters: E::Params)
	where E: notification::Notification {
	let event = lsp_server::Notification::new(E::METHOD.to_owned(), parameters);
	let message = lsp_server::Message::Notification(event);
	scene.connection.sender.send(message).unwrap();
}

pub fn open_text_document(scene: MScene, event: DidOpenTextDocumentParams) {
	let path = &event.text_document.uri.to_file_path().unwrap();
	let key = scene.ctx.files.create(&path, event.text_document.text.into());
	scene.watched.insert(path.clone());
	scene.invalidate(&key);
	update(scene, path);
}

pub fn change_text_document(scene: MScene, event: DidChangeTextDocumentParams) {
	assert_eq!(event.content_changes.len(), 1);
	let path = &event.text_document.uri.to_file_path().unwrap();
	let change = event.content_changes.into_iter().next().unwrap();
	let key = scene.ctx.files.update(&path, change.text.into());
	scene.invalidate(&key);
	update(scene, path);
}

pub fn close_text_document(scene: MScene, event: DidCloseTextDocumentParams) {
	let path = event.text_document.uri.to_file_path().unwrap();
	scene.watched.remove(&path);
}

fn update(scene: MScene, path: &FilePath) {
	let _ = super::diagnostics(scene, &path);
}

pub fn change_watched_files(scene: MScene, event: DidChangeWatchedFilesParams) {
	for change in event.changes {
		let path = change.uri.to_file_path().unwrap();
		if scene.watched.contains(&path) { continue; }
		let key = scene.ctx.files.remove(&path);
		scene.invalidate(&key);
	}
}

