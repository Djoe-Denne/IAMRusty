# Constraint tooling: proportionate verification

## Contract and acceptance scope

Implementation contract: docs/contracts/clippy-constraint-planner.md,
integrated at 59ef8b62 before isolated authors started.
The deliverable is standalone read-only tooling and a real-code planning report.
No production correction, automatic approval, agent dispatch or exhaustive
reference guarantee is part of acceptance.

## Author proofs

- Producer: native Python fixtures for transport guards, strict decoders,
  pagination progress/counts/errors/budgets, source-coordinate mapping, stale
  hashes, snapshot binding and explicit incomplete/unknown coverage.
- Planner: native Python fixtures for contract/schema/ref validation,
  approved/co-edit versus dependency relations, minimal connected components,
  outside-diagnostic consumers, blocker propagation, alias accounting and
  input/output separation.
- Both authors run only their targeted tests. No Cargo/runtime/remote writes.

## Parent integration proofs

1. Inspect and integrate each isolated delta without foreign-file changes.
2. Replay both fixture suites on the integrated tree and check deterministic
   outputs/accounting.
3. Use existing captures only with independently recorded source equivalence;
   record actual current file hashes before and after collection.
4. Run bounded direct MCP queries on current diagnostic seeds. Verify actual
   coordinate adapters and raw payload pages, not just successful transport.
5. Feed normalized producer output into the offline planner. Review a local
   candidate, an actual cross-file relation and an unresolved/stale example.
6. Report confirmed versus proposed/unknown groups honestly. Zero approved
   multi-file groups is allowed only with named candidates/missing proof, not
   as a claim that the repository has no transverse work.

## Real negative witness already observed

Two who_uses pages for apparatus_contracts::runtime::validate_runtime_uid have
consistent totals and pagination. Three current-source slices contain the
reported identifier; a maintenance.rs slice does not. Mark stale-or-coordinate
uncertainty rather than shifting offsets or rebuilding the shared graph.
Evidence: .tmp/clippy-campaign-20261114/hir-who-uses-two-pages.json.

## Independent review

After integration, use a fresh bounded correctness review for graph/accounting/
dispatchability semantics and security review for newly exposed MCP/root/output
boundaries if those changed materially. Persist briefings before corrections.
No automatic four-role review pipeline.

## Closeout

Normal hooks, scoped diff review, durable English report, commit/push and
remote verification. Inventory both owned worktrees; preserve their useful
contents before removal and retain foreign AGENTS.md/Obsidian changes.
No Docker/Kind/WSL resources are required.

## Observed outcome

Final integrated suites: producer 56 PASS, planner 38 PASS, no skips.
Two actual bounded producer-to-planner replays retained all 1,046 records;
corrected HIR pagination returned 14/14 indexed usages with partial freshness.
The original negative span witness remains visible.
Independent targeted rechecks resolved C-1/C-2 and the output/capture collision.

Utility acceptance is NOT met: no exact mapping on the real selected mixed Rust
files, no normalized relation, no approved co-edit constraint and no demonstrably
independent local correction group. All 131 diagnostic-file components remain
unresolved. No broad parser/scanner/harness extension is authorized by this
partial replay. Report and next-step boundary are persisted in
docs/missions/20261114-clippy-constraint-replay.md and the implementation contract.
