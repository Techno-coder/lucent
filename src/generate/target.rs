use std::{fmt, mem};

use crate::node::Width;

pub struct Architecture {
	pub pointer: Width,
}

const HOST: Architecture = Architecture {
	pointer: Width::new(mem::size_of::<*const u8>()).unwrap(),
};

macro_rules! targets {
    ($($name:ident $string:expr => $target:path;)*) => {
		/// The target architecture for an item.
    	#[derive(Debug, Copy, Clone, PartialEq)]
    	pub enum Target { $($name,)* }

    	impl Target {
    		pub fn parse(string: &str) -> Option<Self> {
    			Some(match string {
    				$($string => Target::$name,)*
    				_ => return None,
    			})
    		}
    	}

		impl std::ops::Deref for Target {
			type Target = Architecture;

			fn deref(&self) -> &Self::Target {
				match self {
    				$(Target::$name => &$target,)*
				}
			}
		}

    	impl fmt::Display for Target {
    		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    			write!(f, "\"{}\"", match self {
    				$(Target::$name => $string,)*
    			})
    		}
    	}
    };
}

targets! {
	Host "host" => HOST;
}
