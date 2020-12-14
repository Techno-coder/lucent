use std::fmt;

macro_rules! targets {
    ($($name:ident $string:expr;)*) => {
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
	Host "host";
}
