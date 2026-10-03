//! Application layer for Lazaret.

mod grants;
mod identity;
mod invoke;
mod kv_lifecycle;

use rustycog::command::{CommandRegistry, CommandRegistryBuilder};

pub use grants::GrantService;
pub use identity::{EnrollCommand, IdentityService};
pub use invoke::{
    invoke_path, plugin_dns_endpoint, DigestDnsPluginLocator, EmptyPluginLocator, InvokeError,
    InvokeService, PluginEndpointLocator, StaticPluginLocator,
};
pub use kv_lifecycle::purge_binding_namespace;

/// Build an empty command registry (no handlers this slice).
#[must_use]
pub fn empty_command_registry() -> CommandRegistry {
    CommandRegistryBuilder::new().build()
}
