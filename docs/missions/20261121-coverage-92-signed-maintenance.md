# Coverage 92% — Manifesto signed maintenance

Two PostgreSQL integration cases extend the existing `runtime_writer_postgres`
target. They use the real maintenance command handler, migrated writer, and
pinned RSA verifier with an ephemeral process-only key. The typed channel
supplies test facts; no external signer, operator authentication, or cluster
attestation is exercised.

- Signed Prepare rejects missing consent, superseded intents, and proofs
  bound to another challenge. It consumes the valid proof exactly once,
  journals its fingerprint, and leaves the business component disabled.
- Open challenge issuance requires the locked route and exact service and
  revision UIDs. Open rejects a valid signature over a report different from
  the prepared report without changing the database or journal.
- Blocking a release before any consumer exists makes the barrier durable.
  Fencing then preserves the barrier. Neither an equal-revision resume nor
  a newer ordinary Prepare can clear it; a newer explicit ResumePrepare can.

The final instrumented native run passed all 12 cases. Both owned fixture
containers were removed; all pre-existing containers were preserved.

## Production defect exposed, not fixed here

The first run exercised positive Open and BlockRelease with an existing
consumer. Both returned `RuntimeAuthorityUnavailable`. The SQL trace and
migrations show that both statements write `project_components.updated_at`,
which the migrated table does not define. Positive Open and release closure
with existing consumers are not claimed by this test-only change. Restore
those positive contract tests alongside the production SQL correction.

## Coverage measurement

On the pinned SonarCloud analysis `1d289873-4126-45f5-a489-102b1670ce10`
(`83695bf9`), **460** previously uncovered new lines in
`services/Manifesto/infra/src/apparatus_runtime/runtime_control/maintenance.rs`
receive positive counts in the passing native LLVM LCOV export. These are
line hits, not branch or successful-cutover proof. The fixed-denominator
projection for this change alone is **89.56%** (26,254 / 29,314). Its
coordinate set and PR #34's 246 measured lines are disjoint: together they
project **90.40%** (26,500 / 29,314), with 469 further lines to reach 92%.
The hosted results still require CI and a SonarCloud scan.
