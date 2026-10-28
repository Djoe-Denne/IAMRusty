//! Knative evidence fixture — phase 1 reservation (G-2/G-3/G-5/G-6/G-8).
//!
//! Deliberately no mock Kubernetes client and no startup/cleanup implementation.
//! B5-dependent scenarios are NON MEASURABLE until the exact lock is ratified.
//! A future consumer must opt in explicitly; importing this file starts nothing.
//!
//! Phase 2 reuses `with_kind`, `kind`, `canary` and `evidence` rather than creating
//! another cluster harness. Before any runtime action it must record the parent
//! lease, full resource IDs/UIDs and the approved lock, then inspect transitive
//! replacements. Names, labels and digests alone never establish ownership.
//!
//! Required observations (not assertions of implementation readiness):
//! - G-2: same-window positive canary, cold/hot hops, authenticated peer and
//!   delegated target, denied direct paths, real runsc and sidecars absent at zero.
//! - G-3/G-8: exact UID/resourceVersion preconditions; a same-name foreign object
//!   survives a stale apply/retirement attempt; no adoption or child deletion.
//! - G-5: effective settings/images match the ratified lock; absolute deadlines,
//!   aggregate Revision budgets and ambiguous sends measured, not mocked.
//! - G-6: external binding-scoped KV survives a new Pod UID; rotation at zero
//!   creates no Pod and secret-canary scans cover control-plane evidence.
//!
//! This skeleton provides no PASS, confinement claim, pin, or runtime permission.
