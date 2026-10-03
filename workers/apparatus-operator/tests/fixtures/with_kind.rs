//! Fixtures T10/T11 : base T6 + module Kind (`required-features` controller).

#[path = "mod.rs"]
mod base;

pub use base::*;

#[path = "kind/mod.rs"]
pub mod kind;
