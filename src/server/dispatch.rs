use lsp_server::{Message, Request, Response};
use lsp_types::request;

use super::{LScene, RScene};

pub struct RequestDispatch<'a, 'b>(RScene<'a, 'b>, Option<Request>);

impl<'a, 'b> RequestDispatch<'a, 'b> {
	pub fn new(scene: RScene<'a, 'b>, value: Request) -> Self {
		Self(scene, Some(value))
	}

	#[must_use]
	pub fn on<R: request::Request, F>(self, function: F) -> Self where
		F: FnOnce(RScene<'a, '_>, R::Params) -> crate::Result<R::Result> {
		match self {
			RequestDispatch(_, None) => self,
			RequestDispatch(scene, Some(request)) => {
				let parameters = request.extract(R::METHOD);
				match parameters {
					Err(value) => Self(scene, Some(value)),
					Ok((id, parameters)) => {
						let result = function(scene, parameters).ok()
							.map(|value| serde_json::to_value(&value).unwrap());
						let response = Response { id, result, error: None };
						let response = Message::Response(response);
						let _ = scene.connection.sender.send(response);
						Self(scene, None)
					}
				}
			}
		}
	}

	pub fn finish(self) {}
}

pub struct Dispatch<'a, 'b, T>(LScene<'a, 'b>, Option<T>);

impl<'a, 'b, T> Dispatch<'a, 'b, T> {
	pub fn new(scene: LScene<'a, 'b>, value: T) -> Self {
		Self(scene, Some(value))
	}

	#[must_use]
	pub fn on<E, P, F>(self, extract: E, function: F) -> Self
		where E: FnOnce(T) -> Result<P, T>,
			  F: FnOnce(LScene<'a, '_>, P) {
		match self {
			Dispatch(_, None) => self,
			Dispatch(scene, Some(value)) => {
				let parameters = extract(value);
				match parameters {
					Err(value) =>
						Self(scene, Some(value)),
					Ok(parameters) => {
						function(scene, parameters);
						Self(scene, None)
					}
				}
			}
		}
	}

	pub fn finish(self) {}
}
