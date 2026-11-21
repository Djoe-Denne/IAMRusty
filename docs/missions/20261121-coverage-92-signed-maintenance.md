# Coverage 92% — Manifesto signed maintenance

Three PostgreSQL integration cases extend the existing `runtime_writer_postgres`
target. They use the real maintenance command handler, migrated writer, and
pinned RSA verifier with an ephemeral process-only key. The typed channel
supplies test facts; no external signer, operator authentication, or cluster
attestation is exercised.

- Signed Prepare rejects missing consent, superseded intents, and proofs
  bound to another challenge. It consumes the valid proof exactly once,
  journals its fingerprint, and leaves the business component disabled.
- Open challenge issuance requires the locked route and exact service and
  revision UIDs. Open rejects a valid signature over a report different from
  the prepared report without changing the database or journal. A matching
  proof activates the component and records the first activation timestamp.
- Blocking an active consumer disables and fences it, records the first
  disable timestamp, and removes its lease without rewriting the proof
  journal. Blocking a prepared consumer invalidates its pending Open.
- Blocking a release before any consumer exists makes the barrier durable.
  Fencing then preserves the barrier. Neither an equal-revision resume nor
  a newer ordinary Prepare can clear it; a newer explicit ResumePrepare can.

## Production SQL correction

The first run exercised positive Open and BlockRelease with an existing
consumer. Both returned `RuntimeAuthorityUnavailable`. The SQL trace and
migrations show that both statements write `project_components.updated_at`,
which the migrated table does not define. The correction uses `activated_at`
and `disabled_at`, preserving the first timestamp as the domain transition
does. No new column, migration, transaction boundary, or legacy event is
introduced. The positive integration cases use the normal migrated schema.
The final instrumented Windows run (including the HTTP tests from PR #34)
passed all **16** cases. The launcher removed both owned fixture
containers and preserved the original inventory. Targeted Clippy and
Rustfmt checks passed.

## Coverage measurement

Before the SQL correction, on the pinned SonarCloud analysis
`1d289873-4126-45f5-a489-102b1670ce10`
(`83695bf9`), **460** previously uncovered new lines in
`services/Manifesto/infra/src/apparatus_runtime/runtime_control/maintenance.rs`
receive positive counts in the passing native LLVM LCOV export. These are
line hits, not branch or successful-cutover proof. The fixed-denominator
projection for this change alone is **89.56%** (26,254 / 29,314). Its
coordinate set and PR #34's 246 measured lines are disjoint: together they
project **90.40%** (26,500 / 29,314), with 469 further lines to reach 92%.
The hosted results still require CI and a SonarCloud scan.

After the SQL correction, one combined native run matches **793** previously
uncovered new lines at unchanged coordinates, excluding the two edited SQL
lines. That projects **90.70%** on the same pinned denominator (26,587 /
29,314), with **382** additional lines needed for 92%. This projection is
not a hosted result.
