//! Communication infrastructure adapters

pub mod email;
pub mod sms;
pub mod notification;
pub mod service;

// Re-export all communication adapters
pub use email::*;
pub use sms::*;
pub use notification::*;
pub use service::*; 