# Usable Clippy/Sonar partition representation

Status: runnable bounded work graph, integrated verification complete; no correction dispatch.
Baseline: master bd5a102d8a2fcb04d0d23562910beb064adfde46.
Contract: section 8 of [the planner contract](../contracts/clippy-constraint-planner.md).
This follows the [initial partial replay](20261114-clippy-constraint-replay.md).

## Intended deliverable

A machine-readable work graph retaining every diagnostic file, source-bound
symbol mappings where demonstrated, separately qualified hard co-edit edges,
candidate dependency boundaries, tentative ownership groups and explicit
unresolved observations. Minimal groups are relative to known hard constraints,
not a guarantee that singleton groups are independent.

No fixed worker count, production correction, automatic approval or dispatch.
The tooling stays on AIForAll master pending a future separately authorized
repository extraction. Residual worktree cleanup is explicitly deferred.

## First actual source-bound result

The direct MCP producer captured clippy:C000102 in
`services/Manifesto/tests/common/migration_roundtrip.rs` as exact:
`assert_initial_schema_roundtrip`, normalized extent [5:0, 62:0), containing
inventory range [5:0, 5:95). This is a real mixed Rust file containing imports,
strings, closures and body macros; the diagnostic is on its ordinary header.

The current source SHA-256 is
`5ce1e1793e3566df062935303d1b25041c328ed6ca96614e4378fec02922b54d`.
The parent checked source and proof hashes against the actual saved MCP queries.
The independent correctness reviewer replayed this real case successfully.
Sonar alias S000316 remains unknown because its column units are not established.

This proves one positive physical-localization case, not reference closure,
local correction-policy acceptance, co-edit scope or repository-wide coverage.
The parent then ran the actual offline planner against those captures. It
retained C000102 exact, S000316 unknown, all 1,046 source records and 131 diagnostic
file components. Independence remains unproven.

Local evidence: `.tmp/clippy-campaign-20261114/partition-first-captures-001/`,
`partition-first-snapshot-001.json`, `partition-first-evidence-001.json` and
`partition-first-produce-001.log`. Offline outputs:
`partition-first-plan-parent-001.json/md` and its command log.

## Correctness gate before expansion

Independent review found three synthetic false-verification cases in the initial
range-local implementation: a bodyless nested foreign function attributed to its
outer function, a relevant inner cfg skipped, and mismatched parent/ancestor
identities accepted through unordered word matching. These require RED/GREEN
regressions and targeted recheck before broadening the actual capture.
All five negative variants and two producer false-proof witnesses were reproduced
before correction. The corrected shared utility and adapters pass the author
suites (23 scanner, 59 producer, 38 planner tests). Independent targeted recheck
passed all three findings using 28 executed discriminants, and the parent reran
the actual offline first-case gate with its exact mapping retained.

The method also explicitly recognizes the narrow non-rewriting lint-attribute
syntax in the contract. In particular, runtime.rs's #![allow(missing_docs)] does
not erase the ordinary validator origin, while relevant cfg/rewriting attributes
remain unsupported. A saved actual validator extent replays verified against the
current source; qualified origin binding is still a separate next-phase check.

## Bounded dependency neighborhood

The selected HIR target is
`apparatus_contracts::runtime::validate_runtime_uid`. Its initial 14 usages across
eight files are a bounded witness set, not an assumed set of valid graph edges.
Positive current-source declaration binding and captured endpoint hashes are
required. Span mismatches and missing identities remain unresolved; invocation
references do not become mandatory joint-edit constraints.

The graph must retain all 131 diagnostic files even when localization or
reference coverage is unknown. Candidate boundary edges are review information,
not a way to relabel blocked work safe.

## Actual bounded collection

The parent executed the extended explicit plan against direct MCP:
134 prehashed source/context files, 15 named query seeds and one HIR reference
target, with query/page/source budgets retained. All 89 normalized queries
reported success.

The producer emitted **23 exact diagnostic mappings** to **21 distinct symbols
in 11 files**. Another 1,001 mappings remain unknown and 22 ambiguous; no record
was dropped. These are source-record counts, not the strict-union defect count.

The observation artifact retains all 14 usage records:
- 10 source-verified file-candidate observations;
- four unresolved native-span mismatches (two maintenance, two admission-proof);
- seven candidate cross-file usage sites across **five distinct file boundaries**;
- three candidate sites remain within runtime.rs, not cross-file boundaries.

The five observed consumers of the runtime.rs declaration are:
- `services/Manifesto/http/src/handlers/runtime_control.rs`;
- `services/Manifesto/infra/src/apparatus_runtime/runtime_control.rs`;
- `services/Manifesto/infra/src/apparatus_runtime/runtime_control/read.rs`;
- `services/Manifesto/infra/src/apparatus_runtime/runtime_control/registry.rs`;
- `workers/apparatus-operator/src/runtime_observation.rs`.

The HTTP handler is outside the diagnostic inventory but is a source-verified
observed consumer. Thus the expected graph has 132 file nodes: 131 diagnostic
files plus that consumer context, not the two Cargo/module proof-context files.

Observation direction is **declaration/provider to observed consumer**, an impact
direction. It is not a reverse claim that the provider depends on its consumer,
nor a task-execution DAG. Partitioning uses candidate boundary coupling; these
observations do not create mandatory joint-edit groups.

All observation snapshot/evidence/inventory hashes, current endpoint hashes and
the snapshot-bound origin proof were independently checked by the parent.
Local artifacts: `partition-bounded-captures-001/`,
`partition-bounded-snapshot-001.json`, `partition-bounded-evidence-001.json`,
`partition-bounded-observations-001.json` and `partition-bounded-produce-001.log`
under `.tmp/clippy-campaign-20261114/`.

## Verified graph and runnable command

The actual offline CLI exported `.tmp/work-partition.json` and
`.tmp/work-partition.md`. The JSON's `work_graph` contains:

- **132 nodes**, preserving all 131 diagnostic files and the observed HTTP
  consumer context;
- ten candidate site edges, including seven cross-file sites over five boundaries;
- zero qualified hard edges/unions;
- **132 tentative ownership groups**, all `review_required`;
- four unresolved observations, propagated rather than silently discarded.

These are not 132 workers or certified independent correction batches. The
representation supports reviewing boundary coupling and assigning ownership;
automatic corrective dispatch and a globally optimal partition are not provided.
The observed dependency neighborhood is one named validator, not all repository
references. Unknown/no edge means `not_observed`, not `independent`.

From the repository root, this single offline command reproduces the graph using
the saved local captures and unchanged participant source hashes:

```powershell
python -B ops/scripts/clippy-constraint-plan.py `
  --inventory .tmp/clippy-campaign-20261114/inventory/overlap-script.json `
  --snapshot .tmp/clippy-campaign-20261114/partition-bounded-snapshot-001.json `
  --evidence .tmp/clippy-campaign-20261114/partition-bounded-evidence-001.json `
  --observations .tmp/clippy-campaign-20261114/partition-bounded-observations-001.json `
  --output .tmp/work-partition.json `
  --report .tmp/work-partition.md
```

It needs no OpenCode or LLM. Fresh collection uses the producer's documented
`produce --observations` command and an already running direct MCP server;
the offline command does not recollect evidence or hide source drift.

Concrete edge: runtime.rs -> Manifesto HTTP runtime-control handler, traced through
`/observations/0` in the hashed observation artifact. Each tentative group retains
its diagnostic keys, qualified constraint IDs, boundary edge IDs and unresolved
review obligations.

## Final verification

Executed natively on the integrated tree, with no skips:

```powershell
python -B -m unittest discover -s ops/tests -p test_clippy_source_ranges.py -q
python -B -m unittest discover -s ops/tests -p test_mcp_code_evidence.py -q
python -B -m unittest discover -s ops/tests -p test_clippy_constraint_plan.py -q
```

Results: **23 + 70 + 55 = 148 passing tests**.
The actual bounded MCP producer and complete offline CLI both exited0.
Independent targeted range recheck passed 28 discriminants; independent producer
security review passed and checked actual artifact/participant hashes; fresh
consumer correctness review passed its cross-file graph scope.

No Rust application edits, corrective workers, Cargo build/test, runtime startup,
index rebuild, repository extraction or cleanup detour was performed.
The ordinary publication hook still runs the repository's native formatting gate.

Parent graph output SHA-256 before any later regeneration:
`1f436fc1404d17a3f59c3b492af7b762a6aa35a31054469204083f97844c4bcb`.
