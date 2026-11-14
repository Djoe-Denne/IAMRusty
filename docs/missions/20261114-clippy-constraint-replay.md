# Clippy/Sonar constraint replay — partial result

Status: partial implementation and real replay; correction dispatch is not approved.
Implementation contract: [standalone planner](../contracts/clippy-constraint-planner.md).

## Hosting decision

The user chose to keep the tooling committed on AIForAll `master` for now and
continue work there. Extraction into a separate project is intended later,
not performed or implicitly authorized as part of this replay. Temporary
author worktrees are not the durable copies; the published scripts, guides,
fixtures and contract remain in this repository.

## Result on the actual inventory

- 537 unique Clippy diagnostic records.
- 1,312 Sonar issue keys grouped into 509 exact sites.
- 426 exact cross-source aliases; strict union: **620**.
- Seven uncertain alias hints remain unmerged.
- 136 diagnostic paths resolve to **131 existing physical files**.
- All 131 participant files are byte-identical to both recorded diagnostic
  revisions, including files addressed through the historical RustyCog gitlink.

The union is an identity calculation, not a count of distinct semantic defects.
The planner retains all **1,046 source records** (537 + 509), preserving both
members of an exact alias instead of discarding provenance.

The bounded first real producer/consumer replay reports:

- **0 exact physical mappings demonstrated**.
- 1,024 unknown and 22 ambiguous mappings.
- **131 unresolved file components**.
- **0 confirmed multi-file co-edit groups**.
- Global independence **unproven**.

**These are not 131 independent correction batches.** No approved hard co-edit
constraints were supplied, and no normalized relations were demonstrated.
Zero confirmed groups therefore does not establish absence of cross-file work.

## What was actually exercised

Both integrated Python CLIs ran on the canonical Windows checkout. The producer
used direct MCP SDK connections, not OpenCode APIs or an LLM. It captured and
hashed sources and raw responses, exported normalized partial evidence, and the
offline planner accepted the snapshot/evidence bindings and retained accounting.

The source-binding check independently retrieved actual Git blob bytes at
Clippy revision `68d0d3fb89c15bf19b8735631d9c4ae8c6914aaa` and Sonar revision
`a7e65f2c78efb2ab954701fdee18dca966ba61d5`, then compared current bytes. It did
not manufacture historical hashes from current files.

Only six lexical query seeds were selected, not every symbol in 131 files:
IAMRusty oauth_transactions `fmt`, Manifesto runtime registry
`register_instance`, Manifesto `main`, migration-roundtrip
`assert_initial_schema_roundtrip`, OAuthConnect support `provider`, and
apparatus-operator observation `observe_instances_once`.
These are query candidates, not verified mappings.

## Why useful grouping is still blocked

The shared localization proof currently accepts whole files containing only
simple top-level functions and documentation. Real selected files also contain
imports, types, implementations, attributes, macros or strings. Successful
Serena lookup alone does not satisfy the independent local-uniqueness replay.
The present method therefore fails the real-code utility acceptance criterion.

Sonar coordinate units are not established by this campaign. HIR and Serena
identities are not automatically interchangeable. A complete page sequence
does not prove private, macro, trait, feature or test coverage.

A separate direct witness for `validate_runtime_uid` found 14 indexed HIR usages
versus 16 Serena references across eight files; the scopes and kinds differ.
One HIR position in Manifesto runtime maintenance does not match the current
identifier. No offset repair or graph rebuild was performed.

The first full-page producer run also exposed a legitimate HIR usage shape not
covered by the initial decoder: two rows omit `consumer_function`. The corrected
adapter preserves that omission without inventing a caller. The final bounded
replay exports all **14 of 14 indexed usages**, with source freshness still
partial, no normalized relation, and the maintenance mismatch still visible.

## Verification and review

After corrective review deltas, the canonical integrated suites passed
**56 producer tests and 38 planner tests**, including synthetic actual-CLI
producer-to-consumer interoperability, with no skipped tests. That fixture proves
protocol compatibility on its synthetic subset, not usable localization across
this repository.

Independent correctness review reproduced two blockers: mixed complete/partial
closure evidence could falsely qualify a union, and candidate constraints could
leave referenced diagnostic files outside their blocker scope.
Independent security review found no demonstrated security bypass, but identified
an output/capture collision correctness defect. RED/GREEN regressions and targeted
independent rechecks resolved all three findings. Hard unions now require every
referenced closure query to qualify first; blocker scope includes referenced
diagnostic paths; output/snapshot destinations cannot replace generated captures,
and discovered source aliases are rechecked before publication.

Executed on the final integrated tree:

```powershell
python -B -m unittest discover -s ops/tests -p test_mcp_code_evidence.py -q
python -B -m unittest discover -s ops/tests -p test_clippy_constraint_plan.py -q
```

Both real runs used `--timeout 180 produce`,
the same six seeds, explicit historical binding, bounded query/page/source
budgets, and `--allow-partial`, followed by the offline planner. The second
replay did not broaden localization syntax or query scope.

No application Rust correction, correction-agent dispatch, Cargo build/test,
Docker/Kind/WSL startup, index rebuild or shared-project activation occurred.

## Reproducible local evidence

Local runtime artifacts remain under `.tmp/clippy-campaign-20261114/`, excluded
from durable source commits:

- `independent-participant-comparison.json` and `independent-binding.json`
- `constraint-producer-plan.json` and `constraint-captures-001/`
- `constraint-source-snapshot.json` and `constraint-code-evidence.json`
- `constraint-plan-001.json`, `constraint-plan-001.md` and command logs
- Corrected replay: `constraint-captures-002/`,
  `constraint-source-snapshot-002.json`, `constraint-code-evidence-002.json`,
  `constraint-plan-002.json/md`, `produce-real-002.log`, `plan-real-002.log`

Initial inventory SHA-256:
`f25eabc07d28c4a9fb6de29db430867b28447f5d29bab65f3b947bb1cef10839`.
First planner output SHA-256:
`0a8c1ef4d57ded0b0ccfd8fe74c8c550bf4d8fe901420b8edff15254ef80fef6`.
These identify the partial run, not a future corrected replay.

Corrected replay snapshot SHA-256:
`1fb316756f1140984e56740bd6af32a7cf639682c59cdd2f912062ca0a3b1598`.
Corrected evidence SHA-256:
`0fcc66f8f77b4a8939e25ee0edd37606dd4c3e0a57653db1cae2bc055f3d1175`.
Corrected planner output SHA-256:
`fafc8e7850b6261f5e0f105ee2068434248dc3f1d515cce44b250051c6088dab`.
The final snapshot also hashes one newly discovered consumer, for 132 captured
source files; the inventory still concerns 131 diagnostic files.

## Next boundary

The bounded correctness fixes are complete; this is still a partial state.
A useful follow-up must first demonstrate one actual diagnostic with a declared
range-local source method, shared positive/negative replay fixtures, and one
source-bound live acceptance run. That is an explicit method extension, not
permission to weaken exactness, silently add a Rust parser/dependency, certify
singleton independence, or dispatch corrections.
