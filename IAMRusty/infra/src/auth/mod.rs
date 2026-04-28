//! `OAuth2` client implementations and password services

mod github;
mod gitlab;
mod password;
mod password_adapter;
mod password_reset_adapter;

pub use github::*;
pub use gitlab::*;
pub use password::*;
pub use password_adapter::*;
pub use password_reset_adapter::*;
