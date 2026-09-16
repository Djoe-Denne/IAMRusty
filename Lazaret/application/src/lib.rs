//! Application layer for Lazaret.

mod grants;
mod identity;
mod invoke;

use rustycog::command::{CommandRegistry, CommandRegistryBuilder};

pub use grants::GrantService;
pub use identity::{EnrollCommand, IdentityService};
pub use invoke::{invoke_path, InvokeError, InvokeService};

/// Build an empty command registry (no handlers this slice).
#[must_use]
pub fn empty_command_registry() -> CommandRegistry {
    CommandRegistryBuilder::new().build()
}
