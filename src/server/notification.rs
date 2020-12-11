use std::ffi::OsStr;
use std::io::ErrorKind;
use std::sync::Arc;

use lsp_server::{Connection, Message};
use lsp_types::*;
use lsp_types::notification::{DidChangeWatchedFiles, Notification, ShowMessage};
use lsp_types::request::{RegisterCapability, Request};
use parking_lot::{RwLockUpgradableReadGuard, RwLockWriteGuard};

use crate::FilePath;
use crate::query::{Context, ScopeHandle, Span};

use super::{LScene, RScene, Scene};

const TARGETS_ABSENT: &str = "Language assistance may be limited.";
const TARGETS_FILE: &str = "targets.lucent";

pub fn send_notification<E>(scene: RScene, parameters: E::Params)
	where E: notification::Notification {
	let event = lsp_server::Notification::new(E::METHOD.to_owned(), parameters);
	let message = lsp_server::Message::Notification(event);
	scene.connection.sender.send(message).unwrap();
}

pub fn open_text_document(scene: LScene, event: DidOpenTextDocumentParams) {
	let path = &event.text_document.uri.to_file_path().unwrap();
	if path.extension() != Some(OsStr::new("lc")) { return; }
	let text: Arc<str> = event.text_document.text.into();
	let scene = &mut cancel(scene);

	scene.targets.iter().for_each(|ctx| ctx
		.invalidate(&ctx.files.create(path, text.clone())));
	if linked(scene, path) { scene.unlinked.remove(path); } else {
		scene.unlinked.insert(path.clone(), Context::new(path.clone()));
		let absent = format!("File: {}, is absent from module tree.", path.display());
		let option = "Use this file in a linked module or add its relative path";
		let message = format!("{} {} to: {}", absent, option, TARGETS_FILE);
		let show = ShowMessageParams { typ: MessageType::Warning, message };
		super::send_notification::<ShowMessage>(&scene, show);
	}

	scene.watched.insert(path.clone());
	scene.unlinked.values().for_each(|ctx| ctx
		.invalidate(&ctx.files.create(path, text.clone())));
	let _ = super::diagnostics(scene, &path);
}

pub fn change_text_document(scene: LScene, event: DidChangeTextDocumentParams) {
	assert_eq!(event.content_changes.len(), 1);
	let path = &event.text_document.uri.to_file_path().unwrap();
	let change = event.content_changes.into_iter().next().unwrap();
	let text: Arc<str> = change.text.into();
	let scene = &mut cancel(scene);

	scene.targets.iter().for_each(|ctx| ctx
		.invalidate(&ctx.files.update(path, text.clone())));
	if linked(scene, path) { scene.unlinked.remove(path); }
	scene.unlinked.values().for_each(|ctx| ctx
		.invalidate(&ctx.files.update(path, text.clone())));
	let _ = super::diagnostics(scene, &path);
}

pub fn close_text_document(scene: LScene, event: DidCloseTextDocumentParams) {
	let path = event.text_document.uri.to_file_path().unwrap();
	let scene = &mut cancel(scene);
	scene.unlinked.remove(&path);
	scene.watched.remove(&path);
}

fn linked(scene: RScene, path: &FilePath) -> bool {
	scene.targets.iter().map(|ctx|
		super::file_modules(&mut scene.scope(ctx)
			.span(Span::internal()), path)).flatten()
		.any(|paths| !paths.is_empty())
}

fn cancel<'a, 'b>(scene: LScene<'a, 'b>) -> RwLockWriteGuard<'b, Scene<'a>> {
	let scene = scene.upgradable_read();
	scene.handle.cancel();

	let mut scene = RwLockUpgradableReadGuard::upgrade(scene);
	scene.handle = ScopeHandle::default();
	scene
}

pub fn register_watchers(connection: &Connection) {
	let watchers = vec![
		FileSystemWatcher { glob_pattern: "**/*.lc".to_owned(), kind: None },
		FileSystemWatcher { glob_pattern: "**/*.lucent".to_owned(), kind: None },
	];

	let options = DidChangeWatchedFilesRegistrationOptions { watchers };
	let register_options = Some(serde_json::to_value(options).unwrap());

	let id = "lucent".to_owned();
	let method = DidChangeWatchedFiles::METHOD.to_owned();
	let registrations = vec![Registration { id, method, register_options }];
	let params = serde_json::to_value(RegistrationParams { registrations }).unwrap();

	let method = RegisterCapability::METHOD.to_owned();
	let request = lsp_server::Request { id: 0.into(), method, params };
	connection.sender.send(Message::Request(request)).unwrap();
}

pub fn change_watched_files(scene: LScene, event: DidChangeWatchedFilesParams) {
	let scene = &mut cancel(scene);
	for change in event.changes {
		let path = &change.uri.to_file_path().unwrap();
		let targets = scene.workspace.as_ref()
			.map(|path| path.join(TARGETS_FILE));
		if Some(path) == targets.as_ref() {
			populate_targets(scene);
		}

		if scene.watched.contains(path) { continue; }
		scene.targets.iter().for_each(|ctx| ctx
			.invalidate(&ctx.files.remove(path)));
		scene.unlinked.values().for_each(|ctx| ctx
			.invalidate(&ctx.files.remove(path)));
	}
}

pub fn populate_targets(scene: &mut Scene) {
	let error = |reason: &str| {
		let message = format!("{} {}", reason, TARGETS_ABSENT);
		let show = ShowMessageParams { typ: MessageType::Warning, message };
		super::send_notification::<ShowMessage>(&scene, show);
	};

	let (base, path) = match &scene.workspace {
		None => return error("No workspace root found."),
		Some(base) => (base, base.join(TARGETS_FILE)),
	};

	let error_file = |reason: &str|
		error(&format!("File: {}, {}.", TARGETS_FILE, reason));
	match std::fs::read_to_string(&path) {
		Err(kind) if kind.kind() == ErrorKind::NotFound =>
			error_file("is absent from workspace root"),
		Err(_) => error_file("could not be read"),
		Ok(string) => {
			let lines = string.trim().lines();
			let paths = lines.map(|file| base.join(file));
			let targets: Vec<_> = paths.map(Context::new).collect();
			match targets.is_empty() {
				false => scene.targets = targets,
				true => error_file("is empty"),
			}
		}
	}
}
