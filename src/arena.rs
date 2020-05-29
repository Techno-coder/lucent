use std::marker::PhantomData;

type ArenaNode<T> = Option<Box<Node<T>>>;

#[derive(Debug)]
struct Node<T>(T, ArenaNode<T>);

#[derive(Debug)]
pub struct OwnedArena<'a, T> {
	_owner: PhantomData<&'a ()>,
	root: ArenaNode<T>,
}

impl<'a, T> OwnedArena<'a, T> {
	pub fn push(&mut self, value: T) -> &'a mut T {
		self.root = Some(Box::new(Node(value, self.root.take())));
		let Node(value, _) = self.root.as_mut().unwrap().as_mut();
		unsafe { (value as *mut T).as_mut() }.unwrap()
	}
}

impl<'a, T> Default for OwnedArena<'a, T> {
	fn default() -> Self {
		OwnedArena {
			_owner: Default::default(),
			root: None,
		}
	}
}
