# CLIPPY-20261121-runtime-control-reply-layout — Box the Manifesto snapshot reply

- Status: Proposed as a reusable precedent.
- Date: 2026-11-21
- Rule: `clippy::large_enum_variant`
- Findings: SonarCloud `AaHRE6WM_l0PzZjt4g83` and `AaHRE6WM_l0PzZjt4g84`.
- Approval: the user requested this correction; no approval as a general rule.
- Replaces: none.

## Context

`RuntimeControlReply` is the typed result of Manifesto's internal runtime
control command. Clippy reported its largest variant, `Snapshot`, at 416
bytes, versus 128 for the next largest. The same source warning produced
both remaining Sonar imports. The command and HTTP consumers are in this
workspace. The versioned snapshot DTO is also consumed independently and is
not part of the layout change.

## Decision

The Clippy policy and decision index had no applicable `large_enum_variant`
precedent. Use the lint's indirection suggestion for this payload alone:
`Snapshot(RuntimeBindingSnapshotV1)` becomes
`Snapshot(Box<RuntimeBindingSnapshotV1>)`. Move the successful `get_binding`
result into the box, with no clone. Other replies remain inline and pay no
allocation for this change. Keep the untagged serializer, the same DTO, and
the same route and error mappings. A suppression would leave the layout
unchanged; boxing fields of the shared DTO would affect more consumers.

## Limits and verification

This changes the Rust variant payload type for any direct constructor or
consumer. Do not apply this as written to a stable public Rust enum without
an API review, or assume the extra allocation improves latency on a
snapshot-dominated workload. Tests cover the relative enum size and the
unchanged Axum JSON response. The two native library suites passed (11
tests). Both workspace Clippy lanes completed without the warning or new
diagnostics. No live E2E test was needed for the wire serialization check.
Hosted Sonar reanalysis is tracked separately from the decision's approval.
