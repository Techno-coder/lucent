//! Passes for verifying parsed items.
//! Analysis occurs before type inference so no
//! pass may use type information for verification.

pub use function::*;
pub use loops::*;

mod function;
mod loops;
