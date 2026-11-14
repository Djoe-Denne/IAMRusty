# Standalone diagnostic constraint planner — implementation contract

Status: implementation contract for the authorized tooling/report lot, 2026-11-14.
Baseline supplied by the parent: `966a16b26fe5dd96a8dd191a962989de02c18c3b`.
This contract does not approve corrective changes or dispatch.

## 1. Decision and scope

Extend the existing direct MCP evidence collector as the **producer** of
normalized, source-bound observations. Add a separate **offline consumer** that
qualifies constraints and computes file components. Keep observation, policy
qualification and graph union separate. Neither an MCP relation nor an author's
proposed correction is an approved must-coedit constraint.

No new ADR is required: this is reversible local analysis tooling implementing
the existing campaign policy, not a new service, production API, storage boundary
or correction precedent. No ADR status or business architecture changes.

Sources:

- `AGENTS.md`, local MCP contract;
- `.agents/skills/aiforall-mcp-workflow/SKILL.md`, especially evidence coverage;
- `.agents/skills/aiforall-sonar-policy/SKILL.md`, existing conditional precedents;
- `docs/guides/clippy-decisions/README.md`, approval and applicability rules;
- `docs/guides/clippy-campaigns.md`, snapshot validity and file-exclusive lots;
- `ops/scripts/clippy-sonar-overlap.py`, existing schema version 1;
- `ops/scripts/mcp-code-evidence.py`, existing format version 1 and safeguards;
- `.cursor/handoffs/20261114-clippy-campaign-direct.md`, continuity and limits.

Existing collector behavior is deliberately weaker than the next deliverable:
`collect` currently receives one semantic page and marks pagination unknown.
Catalogue pagination is not semantic pagination. The overlap script's strict
union of 620 is not 620 proven distinct semantic defects. Historical ledger
sections withholding graph permission precede the current explicit authorization.

## 2. Exclusive implementation ownership

**A — producer adapter:** `ops/scripts/mcp-code-evidence.py`,
`ops/tests/test_mcp_code_evidence.py`, `docs/guides/mcp-code-evidence.md`.
Change the pinned requirements file only if the parent explicitly authorizes it;
the existing SDK version is sufficient by default. Own discovery, verified
payload decoding, source-bound symbol mapping, semantic pagination, provenance
and normalized evidence export. Preserve existing commands and raw evidence.

**B — offline planner:** new `ops/scripts/clippy-constraint-plan.py`,
`ops/tests/test_clippy_constraint_plan.py`,
`ops/tests/fixtures/clippy_constraint_plan/` and
`docs/guides/clippy-constraint-plan.md`.
Own strict normalized-input validation, reviewed constraints, bounded local
policy, components and JSON/Markdown report generation. No MCP/network imports
or dependence on producer internals. Fixtures implement this contract without
waiting for live queries.

Both authors can work independently from this boundary. The parent alone owns
live capture coordination, integration replay, the campaign report and ledger.
Live consumer integration waits until A's producer output and tests are ready.
Neither author edits the overlap tool, this contract, foreign `AGENTS.md`,
Obsidian manifest, Rust sources, submodules or another author's files.
No correction agents, Cargo/runtime, staging, commits or index maintenance.

## 3. Artifact boundary and CLI

B's CLI accepts `--inventory`, `--evidence`, `--snapshot`, optional
`--constraints`, required `--output`, optional `--report`. All are local files;
outputs must differ from every input and are replaced atomically only after
validation. JSON is the machine authority; Markdown is a concise rendering.
No executable policy, arbitrary expressions or commands in inputs.

Inventory input is the existing overlap output (`schema_version: 1`), unchanged.
Retain its IDs, records, exact matches, provenance, hints and limitations.
Use a diagnostic key `clippy:<id>` or `sonar:<id>` to avoid namespace collisions.
Exact overlap links are alias metadata, not graph edges. Preserve uncertain
aliases separately; never silently merge the seven existing unmerged hints.
Strict union counts and semantic defect counts must never share a label.

Snapshot input and normalized evidence use the following **new local schemas**,
not purported MCP wire schemas. A must adapt only payloads actually verified
against the live catalogue and saved successful/negative examples. Unsupported
wire shapes become explicit unknowns, never heuristic empty results.

### Snapshot (`schema_version: 1`, `kind: clippy-source-snapshot`)

Required fields:

- `workspace`: canonical absolute repository root;
- `head`: full commit ID; `captured_at`: UTC timestamp;
- `files`: array of `{path, sha256}`, repository-relative normalized file paths;
- `inputs`: array of `{role, path, sha256}`, binding inventory and source captures;
- `configuration`: object recording toolchain, feature/target/lane selections,
  submodule revisions and known index configuration, with unknown values explicit;
- `source_binding`: `verified | partial | unknown`, and `limitations`: strings.

HEAD alone cannot establish freshness of a dirty tree. Hash all mapped and
constraint-participant files, including consumers without diagnostics; record
dirty-source information when available. Validate source hashes before and after
collection. Binding an old capture to today's hashes requires independent source
equivalence evidence; do not manufacture historical capture hashes. The offline
planner may read current sources to verify hashes but never modify them.
Sonar source revision and local Clippy source binding remain distinct.

### Evidence (`schema_version: 1`, `kind: clippy-code-evidence`)

Required top-level fields: `workspace`, `snapshot_sha256`, `inventory_sha256`,
`producer`, `mappings`, `relations`, `queries`, `limitations`.
`producer` records adapter version and catalogue/capture references with hashes.
IDs are unique within each array and references must resolve.

- `mappings`: `{diagnostic_key, status, symbols, evidence_refs, reasons}`.
  Status is `exact | ambiguous | unknown | stale | outside_root`.
  Each symbol has `{id, qualified_name, kind, path, range, source_sha256}`.
  `id` is capture-local, not a claimed stable cross-run identity.
- `range`: `{start_line, start_column, end_line, end_column, column_unit}`.
  Lines are 1-based, columns 0-based, end exclusive; `column_unit` is
  `utf8_byte | unicode_scalar | utf16`. Preserve raw native coordinates in query
  artifacts. Conversion requires a tested source-aware conversion; line-only
  overlap cannot establish exact mapping.
- `relations`: `{id, kind, from_symbol, to_symbol, evidence_refs, coverage}`.
  Kind is `reference | call | implementation | import | reexport`.
  Endpoints reference verified symbols; unresolved endpoints stay in query
  limitations rather than invented symbol nodes.
- `queries`: `{id, tool, arguments, artifact, artifact_sha256, status,
  pagination, coverage}`. Status is `ok | partial | error | unsupported`.
  `pagination` has `{status, pages, returned, total, reason}`; status is
  `complete | partial | unknown | not_applicable`, total is integer or null.
  `coverage` records `{private, macros, traits, features, tests, freshness}`;
  each value is `verified | partial | unknown | not_applicable`.
  Coverage claims require supporting query/source evidence, not defaults.
- `evidence_refs` are query IDs or local `{path, sha256, locator}` references
  to captured source/context. They are data, never tool execution instructions.

An `exact` mapping requires a unique current enclosing symbol for the whole
diagnostic range, using the most specific applicable symbol and verified source
coordinates. Ambiguous overlapping symbols, expanded macro locations,
multi-primary findings, missing ranges, private omissions and incomplete symbol
discovery stay explicit. A declaration identifier span alone is not an enclosing
body span. Do not use nearest-name, similar-message, fuzzy-line or offset mapping.
Module/crate-level findings without a unique applicable symbol remain unknown.

### Physical localization is not reference closure

An exact mapping proves a physical location in the captured current source,
not completeness of consumers, reference graphs, features or tests. Do not apply
the full co-edit closure predicate to mapping acceptance. In particular, Serena
pagination marked `unknown` because no totals are advertised does not downgrade
a source-bound mapping when relevant **local** uniqueness is independently
crosschecked. Do not rewrite that query's pagination as complete.

Local discovery incompleteness is different: checking one selected seed and
finding one containing symbol does not exclude a missing nested or competing
symbol. Missing local alternatives, unsupported syntax or unverified coordinate
semantics keep localization unknown/ambiguous even if the selected symbol exists.
Unknown external references do not erase a demonstrated physical mapping.

#### Shared local-proof reference convention

Use the existing `mapping.evidence_refs` object form
`{path, sha256, locator}`; no new top-level evidence fields are required.
For a dedicated proof JSON file, `locator` is `"/"` (the document root).
Snapshot-bind that file as an input, verify its hash, and retain the query IDs
as ordinary string references alongside it. A coordinate/policy context file
without the following discriminator is not a localization proof.

The proof document has these exact required fields:

- `schema_version`: `1`;
- `kind`: `"clippy-localization-proof"`;
- `diagnostic_key`: the mapping's namespaced diagnostic key;
- `source_path`, `source_sha256`: the same canonical path and hash as the symbol;
- `symbol_id`: the one normalized symbol selected by this mapping;
- `inventory_column_unit`: `unicode_scalar | utf8_byte | utf16`, verified for
  this diagnostic's original inventory coordinates;
- `local_discovery`: an object with `method`, `scope_range`,
  `candidate_symbol_ids`, `query_ids`, `checked_predicates`, `limitations`.

`scope_range` uses the normalized range schema and contains the whole diagnostic
range. `candidate_symbol_ids` lists all remaining most-specific enclosing
candidates in that scope, not merely the supplied seeds; exact acceptance
requires exactly `[symbol_id]`. `query_ids` is a nonempty list of successful,
decoded source-symbol queries, also present in the mapping's evidence references.
An overview alone is insufficient. `limitations` is a string array; any unresolved
limitation affecting local uniqueness prevents an exact result.

`method` is `"source_crosschecked_simple_function_v1"` for the initial bounded
implementation. This means current-source deterministic checks establish the
relevant declaration/body extent and exclude nested/competing local declarations,
with the selected extent independently verified by MCP. It does not mean a regex
search over supplied seeds establishes a complete Rust symbol inventory.
Unsupported macro/generated expansion, ambiguous attributes, comments/string
tokens, nesting or syntax that the implementation cannot soundly distinguish
must fail this method rather than be silently ignored. The implementation may
support a smaller explicit syntax subset; its fixtures must define that subset.
Any additional proof method requires an explicit shared contract extension.

`checked_predicates` contains exactly these boolean fields, all true for an
accepted exact proof:

- `source_binding_verified`;
- `source_hash_stable_before_after`;
- `diagnostic_coordinates_verified`;
- `symbol_endpoints_source_verified`;
- `diagnostic_fully_enclosed`;
- `relevant_local_candidates_exhausted`;
- `nested_or_competing_symbols_excluded`.

These fields are summaries of reproducible checks, not author attestations.
A must emit them only after executing the bounded checks, preserving source and
MCP query witnesses. B validates bindings, query references, supported method,
coordinate conversion/containment and candidate identity, and independently
replays the bounded source-local checks rather than trusting true booleans.
Fixtures must reject a proof whose booleans claim completeness but whose source
contains an unsupported or omitted competing declaration.

This proof confers no semantic approval, local-fix policy classification,
consumer closure or must-coedit authority. Existing local-policy context
predicates and reviewed-constraint requirements remain separate and unchanged.

## 4. Producer requirements

Use the existing pinned direct SDK and loopback, no-proxy/no-redirect protections.
Retain root validation, input/output separation, finite deadlines, budgets and
owned client-session cleanup. No OpenCode API or LLM is required.

Extend only curated read-only tools for verified symbol discovery and relations.
Prefer verified HIR discovery where sufficient. A Serena adapter is optional:
if needed, use a separate explicit curated adapter and verify its root/config
after initial instructions; never `activate_project`. No arbitrary passthrough,
rebuild, cache clear, server reconfiguration or hidden maintenance.

Catalogue argument names and result wrappers must be confirmed, not guessed from
this contract. Parent-provided live catalogue/payload artifacts are the wire
authority. Keep fixture examples of each supported envelope, including text JSON,
structured content, errors and conflicting dual payloads.

### Verified adapter entry points (parent live inspection)

The parent verified the shared root and Serena LSP readiness (2.1.0) through
`initial_instructions` and `get_current_config`; this is root/readiness evidence,
not proof of source freshness or exhaustive symbols. Captures:

- `.tmp/clippy-campaign-20261114/serena-catalog.json`;
- `.tmp/clippy-campaign-20261114/mcp-catalog-smoke.json`;
- `.tmp/clippy-campaign-20261114/hir-who_uses-sample.json`;
- `.tmp/clippy-campaign-20261114/hir-module_tree-sample.json`.

Serena `get_symbols_overview` at depth zero returns kind-to-name groups without
locations. Discover names there, then use bounded nonempty `find_symbol` queries
on the specific file. An empty `name_path_pattern` is invalid, not a wildcard.
The verified query for `validate_runtime_uid` in
`crates/apparatus-contracts/src/runtime.rs`, with `include_body=false`, returns
`name_path`, `kind`, `relative_path` and
`body_location: {start_line: 268, end_line: 277}`; these lines are zero-based.
The sample alone does not establish whether the end line is inclusive.
Confirm endpoint semantics against the current source and a boundary fixture
before conversion to the normalized end-exclusive range.

Missing wire columns can be converted to whole-line boundaries only when the
verified body range and current source establish those boundaries. Never invent
identifier columns. Require diagnostic containment in the verified body extent
and disambiguate nested symbols; otherwise retain ambiguous/unknown status.
A source-lexical candidate may seed a targeted MCP query, but lexical discovery
alone cannot verify its range. `module_tree` provides qualified names/tree, not
positions; do not use it alone for diagnostic mapping.

The verified HIR `who_uses` envelope contains `target`, `total_match_count`,
`offset`, `limit`, `summary`, `returned_match_count` and `usages`. The sample for
`apparatus_contracts::runtime::validate_runtime_uid` reports total 14, offset 0,
limit 2 and returned 2. Usage records contain `file`, `start`, `end`, `category`,
`consumer_module`, `consumer_function`. Advance offsets by verified returned
counts and validate progress/total consistency. Do not assume that other HIR
tools share this envelope or that `start`/`end` units are already established.
References with unverified endpoint units remain raw query evidence until their
coordinates are verified.

Some returned file strings contain doubled backslashes even after JSON decoding.
Normalize repeated path separators within a validated root and preserve the raw
spelling. Do not collapse compiler dot aliases or treat this normalization as
permission to resolve `..`, UNC/device prefixes or out-of-root paths. Canonical
file aliases remain explicit and cannot change strict inventory counts.

Semantic pagination must follow the verified tool-specific envelope, not the
catalogue cursor. Record requested offsets/cursors, returned counts and totals;
detect repeated pages, duplicate identities, gaps, non-progress, inconsistent
totals, truncation and budget/deadline exhaustion. A complete zero is complete
only for that query's declared indexed scope, not all Rust consumers.
Partial evidence remains usable as partial observations but never as exhaustive
closure. Transport success or `isError=false` alone does not certify decoding.
Preserve raw pages and failure evidence; no exception-to-zero conversion.

## 5. Constraint qualification

Optional reviewed input has `schema_version: 1`, `kind: clippy-coedit-constraints`,
`workspace`, `snapshot_sha256`, `inventory_sha256`, and `constraints`.
Each constraint contains:

`{id, diagnostic_keys, files, classification, rationale, evidence_refs,
policy_ref, approval, scope_complete}`.

- `classification`: `must_coedit | candidate | rejected`;
- `files`: canonical repository-relative paths, at least two distinct files
  for `must_coedit`; include actual required consumers without diagnostics;
- `policy_ref`: file plus decision ID or exact legacy section/rule;
- `approval`: null, or `{reviewer, authority_ref, reviewed_at}`;
- `scope_complete`: boolean, supported by evidence, not a confidence score.

Reviewed hard constraints require explicit delegated/human approval independent
of the proposal author, a rationale explaining why **this correction** needs
joint edits, verified participants, current snapshot binding and evidence of
scope closure. Never mark an author proposal accepted automatically. Approval
does not repair stale or missing evidence. Rejected and candidate entries remain
visible but do not union files. Unknown files, unsupported policy references,
unbound evidence or incomplete scope block dispatchability.

Dependency/call/reference/import/implementation edges are candidates by default.
Similar code, the same lint/rule, common package or touching the same symbol are
not hard edges. A proposed public signature change needs correction-specific
caller/implementation qualification; `pub` does not mean local and `private`
does not imply complete references. Do not equate call counts with strict
function invocations.

Bounded automated classification may label a **local candidate**, not create
cross-file hard constraints, for these existing policy contexts only:

- `doc_markdown`, `missing_errors_doc`, `missing_panics_doc`: verified literal
  source documentation only; preserve API, behavior and documentation meaning;
- `uninlined_format_args`, `needless_raw_string_hashes`, `unreadable_literal`:
  verified single-source suggestion/context, no signature, visibility, macro
  expansion, generated source, exported format contract or cross-file edit.

Every automatic decision records its exact policy reference and checked
predicates; incomplete predicate evidence means `needs_review`. No general
"mechanical" rule-name whitelist. Other legacy precedents may inform a reviewed
constraint but are not implicitly automated by this contract. A local fix
classification is not proof that its file's other diagnostics are local.

## 6. Graph and report semantics

Nodes are canonical files with diagnostics/provenance attached. Maintain a
lossless raw-path-to-canonical mapping; reject out-of-root traversal and detect
case/symlink aliases rather than guessing identities. Do not change the overlap
tool's conservative path identity or its strict counts.

Union only qualified hard constraints, as undirected required-file sets.
Compute deterministic connected components, with sorted files, diagnostics and
constraint IDs. Components are minimal **relative to the accepted hard
constraints**, not globally proven optimal correction batches. A singleton with
no edges is not automatically safe.

Maintain candidates and unresolved work separately. Propagate a blocker to every
component it may affect; a bounded unresolved cross-component relation blocks its
participants without merging them. If affected scope cannot be bounded, report
that global independence is unproven. Do not claim all components dispatchable.
Do not turn every artifact limitation or incomplete query into a blocker on every
component. Determine relevance from validated mapping/query references, verified
relation endpoints and explicit constraint participants, not free-form prose or
an arbitrary tool argument. A file-scoped local-discovery failure affects that
file's mappings; incomplete reference closure affects the correction/constraints
that depend on it. A genuinely unbounded uncertainty denies global independence,
but must not erase known mappings, accepted file-local evidence or otherwise
demonstrated candidate classifications. Retain unused incomplete queries and
artifact caveats in report completeness without claiming their irrelevance or
automatically treating them as a failed proof for each unrelated component.
Neither scoped blocking nor local mapping proof relaxes approved must-coedit
scope closure or permits lint-name-only local classification.
Separate graph components from scheduling: package × src/tests mutex and
validation/integration dependencies remain scheduling metadata, not hard unions.

Report at least:

- strict input counts, exact aliases, uncertain aliases and mapping status counts;
- candidate local singleton groups and fully qualified multi-file components,
  with concrete representative evidence, rationale and outside-diagnostic files;
- unresolved/blocked groups, diagnostics and reasons; one unknown finding must
  not disappear because another finding in the same file is locally classified;
- query/page completeness separately from language/index/feature/test coverage;
- snapshot validity per source, evidence age and before/after source verification;
- a bounded `dispatchability`: `candidate | blocked`, never a safety certificate;
- accounting identities showing every input diagnostic retained exactly once in
  the provenance inventory, even where aliases or absent-file findings exist.

Produce a meaningful real-code report after replay, not only synthetic fixtures
or an unqualified list of singletons. If evidence cannot confirm a multi-file
constraint, show zero **confirmed** groups plus named unresolved candidates and
the exact missing proof; do not fabricate unions to improve the report.
No worker count, forced pairs, fixed ten-agent target or corrective dispatch.

## 7. Validation and integration gate

A runs fixture-only `test_mcp_code_evidence.py`; B runs fixture-only
`test_clippy_constraint_plan.py`. No Cargo or runtime is required.

Required negative fixtures: stale hashes/dirty source mismatch; outside-root
paths and aliases; line/column convention mismatch; ambiguous/private/macro
mapping; unknown trait/feature/test coverage; malformed/error payload; repeated,
gapped, truncated or inconsistent semantic pages; unsupported envelope; reviewer
missing or self-approval; incomplete approved constraint; candidate call edges
not unioned; same-rule findings not unioned; required non-diagnostic consumer;
transitive hard union; unknown endpoint/blocker propagation; stable ordering;
strict-union accounting versus uncertain aliases; output/input collision.

Parent integration replays A on bounded current diagnostic seeds and then B on
the actual inventory and captured evidence. Independently verify representative
local, multi-file and unresolved cases against current sources. Report exact
commands/results and skipped validations. A successful offline plan is not a
Clippy fix, an exhaustive reference proof or authorization to dispatch.

No migration, runtime rollout or new human business decision is needed. A genuine
new correction precedent remains Proposed in the normal registry and outside
this tooling lot's automatic authority. The parent should reference this file
from the campaign ledger/guide; only this contract is owned by the architecture
leaf. No commit or publication is performed by this leaf.

### Real-replay stop condition and next-step boundary

The first bounded real replay reported by the parent retained 1,024 unknown and
22 ambiguous mappings, zero exact mappings, and no confirmed relations or
approved co-edit constraints. Those mapping-record counts are not a replacement
for strict-union or semantic-defect counts. Inspection of the producer's
`simple_file_functions` and guide confirms that its initial method accepts only
whole files made of simple functions and documentation, rejecting mixed imports,
types and implementations. All six seeds in
`.tmp/clippy-campaign-20261114/constraint-producer-plan.json` target real mixed
Rust files. This is a material utility limitation, not missing permission to
reclassify unsupported syntax as exact.

Close this replay with an explicit partial report rather than iterate through
unapproved parser changes or certify singleton independence. Zero confirmed
multi-file components means no demonstrated hard unions in this capture, not
proof that all files are independent. Distinguish absent relation observations
from a demonstrated absence of consumers.

Targeted Serena symbol/child ranges plus source-aware lexical verification could
support a later **range-local** method without requiring a full Rust parser or a
new dependency. This would be an explicit extension, not a relaxation of
`source_crosschecked_simple_function_v1`. Overview/child results alone do not
prove completeness; lexical masking must account for supported comments, quoted
and raw strings, character/lifetime distinctions, balanced delimiters and nested
declarations, and reject unsupported macros/cfg/generated cases. Plain regex
replacement of strings/comments is not an adequate substitute.

Do not build that scanner or a new harness automatically in this replay. If a
follow-up is authorized, first select one actual diagnostic and one existing
seed, demonstrate that a bounded range-local method can cover its syntax, and
amend the shared method contract before either author implements it. Require
matching A/B positive and negative fixtures in the existing unittest suites and
one real source-bound acceptance replay; stop if the chosen case exceeds the
declared subset. No complete Rust grammar, new dependency, reference-closure
guarantee or broad rescan is implicitly authorized.
