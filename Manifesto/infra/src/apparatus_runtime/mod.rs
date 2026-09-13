//! Réconciliation Apparatus P2 in-process (allowlist T7).
//!
//! Ports P0, claim, apply, ticker et cleanup.

use apparatus_contracts::OperationId;
use chrono::{DateTime, Utc};
use uuid::Uuid;

mod cas;
mod cleanup;
pub mod in_process;
mod tick;

pub use cas::{write_bind_failure, write_observed};
pub use in_process::InProcessApparatusRuntime;
pub use tick::{
    apply_due_once, apply_due_once_skip_observe, run_once, run_tick_loop, RuntimeApplyError,
};

/// TTL de claim (30 s).
pub const APPARATUS_LEASE_TTL: std::time::Duration = std::time::Duration::from_secs(30);

/// Intervalle du ticker (2 s, ≪ TTL).
pub const APPARATUS_TICK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

/// Plafond de tentatives ; ensuite terminal (`next_retry_at` NULL + code).
pub const APPARATUS_RETRY_MAX: i32 = 8;

/// Base du backoff (`min(base * 2^retry_count, cap)`).
pub const APPARATUS_BACKOFF_BASE: std::time::Duration = std::time::Duration::from_secs(30);

/// Plafond du backoff (5 min).
pub const APPARATUS_BACKOFF_CAP: std::time::Duration = std::time::Duration::from_secs(300);

pub(crate) const APPARATUS_ERROR_BIND_FAILED: &str = "bind_failed";
pub(crate) const APPARATUS_ERROR_TEARDOWN_FAILED: &str = "teardown_failed";

/// Incrémente le compteur et calcule `next_retry_at` (None = terminal).
pub(crate) fn next_backoff(
    current_retry_count: i32,
    now: DateTime<Utc>,
) -> (i32, Option<DateTime<Utc>>) {
    let new_count = current_retry_count.saturating_add(1);
    if new_count >= APPARATUS_RETRY_MAX {
        (APPARATUS_RETRY_MAX, None)
    } else {
        (new_count, Some(now + backoff_delay(new_count)))
    }
}

fn backoff_delay(new_retry_count: i32) -> chrono::Duration {
    let exp = u32::try_from(new_retry_count.clamp(0, 30)).unwrap_or(30);
    let factor = 1i64.checked_shl(exp).unwrap_or(i64::MAX);
    let base = i64::try_from(APPARATUS_BACKOFF_BASE.as_secs()).unwrap_or(i64::MAX);
    let cap = i64::try_from(APPARATUS_BACKOFF_CAP.as_secs()).unwrap_or(i64::MAX);
    chrono::Duration::seconds(base.saturating_mul(factor).min(cap))
}

/// Dérive un `operation_id` déterministe (`component_id` ⊕ génération ⊕ verbe).
#[must_use]
pub fn derived_operation_id(component_id: Uuid, generation: u64, verb: &str) -> OperationId {
    let mut bytes = *component_id.as_bytes();
    let gen_be = generation.to_be_bytes();
    for (i, g) in gen_be.iter().enumerate() {
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
