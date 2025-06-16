//! Data Transfer Objects
//!
//! This module contains DTOs for communication with the HTTP layer
//! and other external interfaces.

pub mod auth;
mod user;

pub use auth::*;
pub use user::*;
