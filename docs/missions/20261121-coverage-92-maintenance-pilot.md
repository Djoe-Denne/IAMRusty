# Coverage 92% — Manifesto maintenance pilot

## What the tests prove

Three cases extend the existing `manifesto-service` target
`runtime_writer_postgres`. They use the real maintenance command handler,
the writer adapter, and the normal migrated PostgreSQL fixture. The test
channel provides typed operator facts; the admission proof verifier still
rejects all proofs. Challenge issuance does not require a signed proof.

- `Inspect` on a missing row is read-only; a successful and replayed
  `Fence` preserves the exact CAS and the single committed journal entry.
- A fence rejected for an enabled business component or a future
  acknowledgement rolls back the guard insert and leaves no binding.
- A `Prepare` challenge has the writer's exact TTL and stored intent. A
  second challenge invalidates the previous pending intent; stale CAS,
  an early `Open` challenge, and an unexpected `Prepare` UID leave the
  committed rows unchanged.

The native Windows integration launcher passed all ten cases in both runs
(three new, seven existing). It removed both owned containers after each
run; no owned container remained running. Targeted Clippy and formatting
checks passed, with no Clippy diagnostics in the new module. This is
database and command integration, not operator authentication, Kubernetes
attestation, mTLS, or signed `Prepare`/`Open` acceptance.

## Coverage measurement

The SonarCloud baseline is analysis `3168e48e-0348-4065-83e7-8c0af944475a`
on `6e46ad0c6e5e2faca0fde5a6b7973fdc54aa46f5`. The two production files
are unchanged from that revision. Intersecting nonzero lines from the native
LLVM LCOV export with Sonar's uncovered new lines gives the same exact
coordinates in both runs:

- `maintenance.rs`: **263** additional new lines exercised.
- `runtime_maintenance_command.rs`: **8** additional new lines exercised.

At the same denominator, 25,506/29,297 becomes 25,777/29,297 (**87.99%**);
**1,177** additional new lines remain to reach 92%. This is a projection,
not a new hosted scan. The pilot does not change production code, coverage
exclusions, the Quality Gate, or how the CI merges coverage.
