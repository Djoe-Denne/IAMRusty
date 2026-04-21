//! Event-to-tuple translators.
//!
//! Each producer service contributes a `Translator` implementation that knows
//! how to turn one of its domain events (`HiveDomainEvent`,
//! `ManifestoDomainEvent`, `IamDomainEvent`) into a set of tuple writes and
//! deletes. The dispatcher picks a translator by the event envelope's
//! `event_type` prefix.

pub mod hive;
pub mod iam;
pub mod manifesto;

use anyhow::Result;

use crate::fga_client::Tuple;

/// Result of translating a single event. Write and delete sets are applied
/// atomically against OpenFGA.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TupleDelta {
    pub writes: Vec<Tuple>,
    pub deletes: Vec<Tuple>,
}

impl TupleDelta {
    pub fn is_empty(&self) -> bool {
        self.writes.is_empty() && self.deletes.is_empty()
    }

    pub fn write(mut self, t: Tuple) -> Self {
        self.writes.push(t);
        self
    }

    pub fn delete(mut self, t: Tuple) -> Self {
        self.deletes.push(t);
        self
    }
}

/// Translator for one producer service.
pub trait Translator: Send + Sync {
    /// Short identifier for logs (e.g. `"hive"`).
    fn name(&self) -> &'static str;

    /// Return `Some(TupleDelta)` if the translator claims this event, `None`
    /// if it cannot decode or does not care about the event. Unknown events
    /// are fine — most domain events are irrelevant to authorization.
    fn translate(&self, raw_event: &serde_json::Value) -> Result<Option<TupleDelta>>;
}
