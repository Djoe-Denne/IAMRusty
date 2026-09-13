//! Réconciliation Apparatus P2 in-process (allowlist T7).
//!
//! Ports P0, claim, apply, ticker et cleanup.

use apparatus_contracts::OperationId;
use uuid::Uuid;

mod cas;
mod cleanup;
pub mod in_process;
mod tick;

pub use cas::write_observed;
pub use in_process::InProcessApparatusRuntime;
pub use tick::{
    apply_due_once, apply_due_once_skip_observe, run_once, run_tick_loop, RuntimeApplyError,
};

/// TTL de claim (30 s).
pub const APPARATUS_LEASE_TTL: std::time::Duration = std::time::Duration::from_secs(30);

/// Intervalle du ticker (2 s, ≪ TTL).
pub const APPARATUS_TICK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

/// Dérive un `operation_id` déterministe (`component_id` ⊕ génération ⊕ verbe).
#[must_use]
pub fn derived_operation_id(component_id: Uuid, generation: u64, verb: &str) -> OperationId {
    let mut bytes = *component_id.as_bytes();
    let gen = generation.to_be_bytes();
    for (i, g) in gen.iter().enumerate() {
        bytes[i] ^= *g;
        bytes[8 + i] ^= g.wrapping_mul(0x5d);
    }
    for (i, b) in verb.as_bytes().iter().enumerate() {
        let idx = i % 16;
        bytes[idx] ^= *b;
        bytes[(idx + 7) % 16] ^= b.wrapping_add(i as u8);
    }
    OperationId::from_bytes(bytes)
}
