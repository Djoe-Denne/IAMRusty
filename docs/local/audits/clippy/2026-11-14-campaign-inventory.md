# Clippy and Sonar campaign inventory

## Scope and collection

- Read-only triage; no fixes, worker dispatch, GitHub/Sonar state changes or
  suppression changes. Snapshot directory:
  `.tmp/clippy-campaign-20261114/inventory/`.
- Local HEAD: `68d0d3fb89c15bf19b8735631d9c4ae8c6914aaa`.
  Uncommitted changes: AGENTS.md documentation-language policy and the Obsidian
  manifest; both preserved. No Rust source changes during collection.
- `just clippy-ci` succeeded on native Windows, toolchain 1.99.1, workspace
  locked/all-features, production then tests. Both build-finished events were
  successful; zero invalid JSON lines and zero compiler errors.
- Sonar public REST API: project Djoe-Denne_IAMRusty, `resolved=false`, 500
  records per page, three pages, 1,312 unique issue keys matching paging.total.
  Retrieval: 2026-11-14 09:10 UTC. Latest analysis: 08:51:53 UTC, revision
  `a7e65f2c78efb2ab954701fdee18dca966ba61d5`. This predates the local tooling-only
  commit; counts are not presented as an identical-source/OS comparison.

## Categorization

Categories are a deterministic rule-based triage, not a verified assessment of
public API consumers or behavioural impact. The mapping is preserved in
`classification-map.json`; classified records, per-rule counts and areas are
preserved beside it. Category totals are mutually exclusive within each source.

- Documentation: **92 Clippy**, **283 Sonar**.
- Mechanical syntax, idioms and imports: **284 Clippy**, **514 Sonar**.
- Structural refactoring and complexity: **22 Clippy**, **68 Sonar**.
- Runtime, ownership, casts and error handling: **65 Clippy**, **159 Sonar**.
- API, types, visibility and annotations: **74 Clippy**, **286 Sonar**.
- Additional rule requiring qualification: **0 Clippy**, **2 Sonar** (`rust:S9334`).

Total: **537 unique Clippy diagnostics**; **1,312 Sonar issue keys**.
Documentation plus mechanical candidates account for 376 local diagnostics;
their actual fixes still require the existing skill/decision registry and
context checks. Rule dispersion across services is not correction impact.

Sonar contains 1,306 code smells and 6 issues typed BUG. Of the 1,312 issue keys,
1,268 come from imported Clippy rules and 44 from native Sonar rules. These are
not additional independent backlogs to sum with local Clippy.

Grouping Sonar by exactly matching rule, component, textRange (or line when
absent) and message produces **509 groups**: documentation 89, mechanical 253,
structure 28, runtime/ownership 66, API/types/annotations 71, qualification 2.
This is not a verified count of implementation changes; different rules may
still refer to one change, and identical-looking findings can need source review.

## Additional compiler warnings

Clippy also reported **492 unique rustc warnings**, tracked separately:
446 dead_code, 19 missing_docs, 14 unused_imports, 5 ambiguous_glob_reexports,
4 static_mut_refs and four other single warnings. No blanket deletion of dead
code is authorized by this inventory.

## Local Clippy findings by referenced area

- workers/apparatus-operator: 113.
- services/IAMRusty: 107.
- services/Manifesto: 85.
- services/Lazaret: 58.
- services/OAuthConnect: 53.
- crates/apparatus-contracts: 53.
- services/Hive: 21.
- workers/sentinel-sync: 17.
- services/Telegraph: 17.
- services/GitHubConnect: 10.
- runtime/monolith: 3.

These counts do not imply eleven independent lots. Shared files, signatures,
callers and package/src-tests ownership must be checked before dispatch.
Sonar additionally references five issues in the RustyCog submodule, which
remains separately scoped; no SDK changes are authorized here.

## Sonar issues typed BUG: review before dispatch

- Four `rust:S7493` blocking-I/O findings: apparatus-operator/admit.rs (one),
  Lazaret/setup/app.rs (two), Manifesto/setup/runtime_maintenance.rs (one).
- Two `rust:S9334` missing-serde-default findings in
  crates/apparatus-contracts/src/runtime.rs. These remain unqualified, not
  confirmed production defects.

Sonar positions are historical navigation hints, not current editing offsets.
No line-offset arithmetic or automated fix decision has been applied.

## Evidence and limits

- `clippy-run.json`, original per-pass JSONL, `clippy-unique.json`,
  `sonar-pages.json`, `sonar-analysis.json`, `classification-summary.json`,
  `classification-map.json` and classified per-source files are retained locally.
- Complete retrieval and accounting were checked; no code/caller impact review,
  tests of proposed fixes, Sonar reanalysis, correction or agent dispatch occurred.
- Local Windows and CI Linux differ. Exact cross-source matching was not
  attempted, and the Sonar site grouping must not be called a global deduplication.
