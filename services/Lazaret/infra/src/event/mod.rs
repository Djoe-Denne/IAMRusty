//! Queue consumers for Lazaret (KV purge on Manifesto `component_removed`).

pub mod consumer;

pub use consumer::{KvPurgeEventConsumer, KvPurgeEventHandler};
