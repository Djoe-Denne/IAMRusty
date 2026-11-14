# Offline diagnostic constraint planner

`ops/scripts/clippy-constraint-plan.py` implements the offline consumer in
[`clippy-constraint-planner.md`](../contracts/clippy-constraint-planner.md).
It reads local artifacts and current source files only. It does not import the
producer, contact MCP or other services, invoke subprocesses, run Cargo, call a
model, edit Rust, dispatch corrections, or change ADR 0501.

## Run

```powershell
python ops/scripts/clippy-constraint-plan.py `
  --inventory .tmp/campaign/inventory.json `
  --snapshot .tmp/campaign/snapshot.json `
  --evidence .tmp/campaign/evidence.json `
  --observations .tmp/campaign/observations.json `
  --constraints .tmp/campaign/constraints.json `
  --output .tmp/campaign/plan.json `
  --report .tmp/campaign/plan.md
```

Omit `--constraints` when no independently reviewed hard constraints exist.
Omit `--observations` when no file-observation artifact exists. The work graph
still includes every diagnostic file and any normalized symbol relations.
Omit `--report` for JSON only. Parent directories must already exist.
The JSON is authoritative; Markdown renders counts, components, concrete symbol
and diagnostic examples, constraint rationales, blockers, coverage and source
validity. Exit status is 0 for a validated report (including a blocked report),
or 2 for an invalid artifact or publication error.

All capture references are root-relative local paths. The snapshot's canonical
absolute `workspace` is the root used to validate sources, not the shell's
current directory. The inventory must be a snapshot input with role `inventory`.
Bind every capture, authorization, closure and policy-context artifact through
`snapshot.inputs`, including the policy file used by a reviewed constraint.
Bind all mapped source files and every hard-constraint participant through
`snapshot.files`, including consumers with no diagnostics.

The snapshot binds the inventory and captures, while evidence and reviewed
constraints bind the inventory and snapshot hashes. Do not include evidence
itself in a snapshot that it hashes: that would create a circular binding.
Use the normalized local schemas in the contract, not guessed MCP wire payloads.
Producer metadata is preserved as provenance, not treated as authorization.

### Replay the captured bounded campaign

With the parent's saved local artifacts present, run this single offline CLI:

```powershell
python -B ops/scripts/clippy-constraint-plan.py `
  --inventory .tmp/clippy-campaign-20261114/inventory/overlap-script.json `
  --snapshot .tmp/clippy-campaign-20261114/partition-bounded-snapshot-001.json `
  --evidence .tmp/clippy-campaign-20261114/partition-bounded-evidence-001.json `
  --observations .tmp/clippy-campaign-20261114/partition-bounded-observations-001.json `
  --output .tmp/clippy-campaign-20261114/partition-work-plan-local.json `
  --report .tmp/clippy-campaign-20261114/partition-work-plan-local.md
```

This does not recollect or refresh MCP evidence. It needs the referenced raw
captures and the same current source hashes; drift remains explicit or fails
qualification. Output names differ from the input artifacts and the author's
`partition-work-plan-B-001.json/md` verification outputs.

## Exploratory work graph and file observations

`work_graph` is always emitted alongside the original conservative component,
blocker and dispatch fields. It is a machine-readable ownership proposal, not a
second execution engine:

- `nodes` retains every diagnostic file, including unknown mappings, and
  source-verified observed consumers without diagnostics. Join `diagnostic_keys`
  to the top-level `diagnostics` array to inspect rule, namespace, original
  Clippy provenance and Sonar issue keys. A node's diagnostic-record count is
  the length of that array, not a semantic defect count.
- `edges` separates qualified `hard_must_coedit` constraints from
  `candidate_dependency` observations. Hard sets use deterministic spanning
  stars while preserving the original complete constraint sets. Candidates
  reference normalized relations or hashed observation JSON pointers.
- `proposed_groups` reuses minimal components relative to qualified hard
  constraints. Cross-group candidate edges become `boundary_edge_ids` and
  explicit review tasks. Local blockers, unresolved observations and global
  coverage caveats produce `unresolved_ids`; readiness is only `candidate` or
  `review_required`, never certified independence.
- `coverage` separates current file hashes, observed normalized symbol IDs,
  exact-mapping symbol IDs, origin bindings, site-edge counts and unique file
  boundaries. It retains query pagination and language/index coverage rather
  than interpreting a complete query as global completeness.

**Arrow legend:** queried declaration/provider → observed consumer is the
**impact direction**, opposite the common `depends_on` reading. These arrows
show boundary coupling for ownership review, not a dependency execution DAG,
topological task order, or proof that every diagnostic in the provider file
touches the queried declaration. Same-file loops are observations, not boundaries.
Repeated sites on one provider/consumer pair remain distinct evidence edges;
the unique boundary count deduplicates file pairs. Missing edges mean
`not_observed`, never `independent`.

The initial bounded neighborhood may therefore have 10 candidate site edges,
including three same-file loops and seven cross-file sites over five unique
provider/consumer boundaries, while retaining four unresolved span mismatches
and zero hard unions. Tentative singleton groups are useful review/ownership
units, not permission to dispatch 131 independent correction lots.

### Observation validation and binding order

`--observations` consumes the contract's `clippy-file-observations` artifact.
Its hash is tracked as a CLI input; it references the finalized snapshot,
evidence and inventory hashes. It **must not** appear in `snapshot.inputs`.
Raw captures, source/context files and origin proofs are already snapshot-bound,
avoiding a circular hash dependency.

For the initial `reference` adapter, replay the captured `who_uses` page sequence:
validate request root/target, page offsets and counts, totals, progress and
duplicate identities. Preserve optional omitted `consumer_function`; an explicit
null, empty or non-string value is unsupported, never an invented caller.
Observations consume returned usage witnesses in per-query capture order,
including unresolved rows, once per site. Cardinality/order mismatch cannot
shift a failed row onto another row's matching file or good byte span.
Raw offsets are not repaired.

A candidate must have current endpoint hashes, a matching observed destination
and a checked raw token span. Its origin separately requires the bounded
`direct_crate_module_function_v1` proof: hashed default Cargo crate/module chain,
signature, one exact definition with verified source-token coordinates, and
hashed Serena witnesses replayed through the shared source-range utility.
The initial chain supports only
`apparatus_contracts::runtime::validate_runtime_uid`, not a general Rust resolver.
Only a literal extended local Windows drive wrapper is accepted in definition
text; UNC, device paths, traversal, ambiguous definitions, cfg/path module
indirection and unsupported overrides cannot establish an origin.

This source-bound origin-to-file candidate does not establish full HIR/Serena
consumer identity. A `verified_symbols` assertion additionally needs matching
normalized symbol sources and a normalized relation witness; a producer label
alone is insufficient. Missing/forged origin evidence and path-only/unresolved
records stay inspectable in `unresolved_observations`, not candidate edges.

Recheck the observation CLI hash, source/context hashes and nested captures
after planning and after staging outputs, before replacement. Output collision
protection covers the observation file and referenced inputs as well as sources.

## Qualification, not dependency clustering

File nodes come from diagnostic paths, verified relation endpoints and reviewed
constraint participants. Only validated `must_coedit` entries union nodes.
Transitive union produces minimal components **relative to those hard
constraints**. Calls, imports, implementations, references, reexports, same-rule
findings, common symbols and exact overlap aliases do not create hard edges.
Candidate and rejected entries remain visible without unioning files.

Each hard constraint needs:

- At least two canonical, distinct, current snapshot-bound files.
- Exact, current diagnostic mappings and correction-specific rationale.
- A snapshot-bound existing policy reference of the form
  `file#exact section or decision locator`. The existing policy skill is
  supported; registry entries must visibly be accepted, never proposed.
- Explicit independent delegated/human review and source-bound scope closure.
- Complete, successful closure for every query referenced by the constraint, whose private, macro,
  trait, feature, test and freshness coverage is verified or explicitly
  inapplicable. All explicitly referenced queries are relevant closure witnesses;
  a complete localization query cannot repair a partial consumer query.
  Pagination completeness alone is insufficient.

Invalid hard-constraint approval, participants or required review evidence reject
the whole plan rather than being silently dropped. A structurally valid proposal
with incomplete query closure remains visible in `unqualified_hard_constraints`
and with named blockers, but is never accepted or unioned. Its declared files
remain separate unless another independently qualified hard constraint joins
them. A complete query alongside a partial 1/10 consumer capture still yields
no confirmed group for that proposal. Ordinary stale/unknown mappings also
remain visible and block affected work.

### Local review artifact convention

The contract deliberately leaves `approval.authority_ref` and local evidence
contents open. For verifiable offline qualification, this consumer accepts an
authority reference as a local `{path, sha256, locator}` object pointing to a
snapshot-bound JSON authorization record:

```json
{
  "reviewer": "delegated-reviewer",
  "proposal_author": "different-author",
  "authorized": true,
  "constraint_ids": ["reviewed-change-1"]
}
```

`approval.reviewer` must match the record, differ from `proposal_author`, and
`reviewed_at` must be UTC and not precede the snapshot. The parent/reviewer must
verify the real delegation and identities: a JSON assertion is not an identity
provider or a substitute for human approval.

A separate local JSON object in the constraint's `evidence_refs` must contain:

```json
{
  "constraint_id": "reviewed-change-1",
  "reviewer": "delegated-reviewer",
  "scope_complete": true,
  "files": ["caller.rs", "definition.rs"],
  "rationale": "The specific correction requires these joint edits.",
  "policy_ref": ".agents/skills/aiforall-sonar-policy/SKILL.md#Campagne 2026-10-08",
  "source_sha256": {
    "caller.rs": "<current snapshot SHA-256>",
    "definition.rs": "<current snapshot SHA-256>"
  }
}
```

The sorted files, rationale and policy reference must match the constraint
exactly. These artifacts are reviewer evidence, never inferred automatically
from a dependency graph. They can exist alongside query references, but cannot
replace complete query/language coverage. An absent convention-specific record
means qualification fails closed; no assumption is made about undocumented
producer internals.

## Local policy candidates

A rule name alone never classifies a finding as local. The automatic subset is
intentionally narrower than the contract's maximum permitted policy set:

- For `doc_markdown`, `missing_errors_doc` and `missing_panics_doc`, require an
  exact unique enclosing mapping, a current hash-bound source and an actual
  single literal `///` or `//!` line at the diagnostic location. Generated
  documentation, include/attribute contexts and code-fence lines do not qualify.
- Also require a snapshot-bound local JSON context reference on that mapping.
  It identifies `diagnostic_key` and the current `source_sha256`, and explicitly
  checks `preserves_api`, `preserves_behavior`,
  `preserves_documentation_meaning`, `literal_source_only`, `not_generated`,
  `no_macro_expansion` and `no_cross_file_edit` as true.
- The policy file itself must be snapshot-bound. The report records the exact
  policy reference and all checked predicates.

The context record is a source review, not an executable predicate. Literal
source structure and hashes are independently checked; semantic invariants
remain explicit review assertions. The offline tool cannot prove meaning
preservation from a comment prefix.

`uninlined_format_args`, `needless_raw_string_hashes`, `unreadable_literal`,
function-line missing-doc diagnostics and all other contexts remain
`needs_review` in this implementation. No formatting suggestion engine or
additional mechanical rule whitelist is introduced. This deliberate limit does
not prevent reviewed constraints from using existing applicable policy.

## Coordinates, freshness and uncertainty

### Physical localization proof

Mapping acceptance follows the shared `clippy-localization-proof` reference
convention in the contract's **Physical localization is not reference closure**
clarification (canonical revision `3f34add0`). No additional mapping fields are
introduced: reference one dedicated, snapshot-input-bound JSON proof through
`mapping.evidence_refs` with `locator: "/"`, alongside the query IDs. A policy
context or coordinate-only record without the proof discriminator is not enough.

Validate all required shared proof fields, the source/symbol identity, the
declared inventory coordinate unit, discovery scope, exactly one remaining
candidate, nonempty successful source-symbol query references and the exact
boolean predicate summaries. This implementation initially supports decoded
`find_symbol` queries (including namespaced tool names ending in `_find_symbol`
or `.find_symbol`), not an overview alone. Pagination may remain `unknown` and
external private/macro/trait/feature/test/reference coverage may be unknown:
those facts do not invalidate a separately replayed physical mapping.

Boolean summaries are not accepted as source proof by themselves. The
`source_crosschecked_simple_function_v1` replay independently scans the entire
current file, not just supplied seeds, using this deliberately small subset:

- Only top-level simple functions, whitespace and literal leading `///`
  documentation; optional `pub`, `pub(crate)` and `async` function prefixes.
- Simple parameter text, optional simple return type, and brace-delimited bodies
  with ordinary identifier/numeric expression tokens and blocks.
- No attributes, macros, generated-source markers, string/character tokens,
  non-documentation comments, closures, nested local declarations, or other
  top-level item syntax. Unsupported code anywhere in the file fails this
  bounded method rather than being silently omitted from discovery.
- Check the selected function's declaration name, complete body endpoints and
  normalized symbol kind (`function`, `Function`, `fn` or LSP function kind 12).
  Exact token extents and whole-line extents are supported; an immediately
  preceding contiguous literal documentation prefix may be included.

This legacy method remains unchanged, including its whole-file limitations.
The additional `source_crosschecked_function_range_v1` method instead calls
A's shared `ops/scripts/clippy_source_ranges.py` utility. B does not maintain a
second expanded grammar or trust a producer-supplied witness list: it decodes
the referenced hashed raw MCP responses, rejects conflicting text/structured
payloads and verifies the advertised source/parent/body witnesses before replay.
Source decoding uses raw bytes and UTF-8 without newline normalization.

The shared range method supports mixed files and ordinary top-level/impl
declarations, comments/literals and balanced opaque invocation macros under the
contract's bounds. An ordinary header outside a body macro may map exactly;
a diagnostic inside that token tree does not. Nested foreign declarations,
relevant cfg, ambiguous ancestry and forged parent identities still fail
attribution. These are shared executable checks, not all-true boolean claims.
No mapping proof grants local correction policy, reference closure or hard
co-edit approval.

The overlap inventory already uses one-based lines and **zero-based** columns.
Both Clippy and Sonar mappings use the proof's explicit
`inventory_column_unit`, not inferred native units. Normalized symbol and scope
ranges separately declare `utf8_byte`, `unicode_scalar` or `utf16`; conversion
checks source boundaries and rejects split UTF-8 sequences or UTF-16 surrogate
pairs. Both verified symbol and discovery scope must contain the whole
diagnostic. Empty, missing or multi-primary locations cannot establish a unique
enclosing symbol.

Producer schema version 1 remains compatible. Older evidence without a shared
localization proof is retained, but its claimed exact mappings downgrade to
unknown; regenerate the proof with the updated producer. Local-policy context
and localization-proof artifacts remain distinct. Neither creates consumer
closure or relaxes reviewed hard-constraint approval.

The planner checks source hashes before and after its own read. A dirty-source
mismatch blocks its findings and invalidates any approved constraint using that
source. It preserves the capture timestamp and configuration, but neither runs
Git nor independently verifies the commit's checkout identity, historic
collection before/after checks, index completeness, Sonar revision equivalence,
or authorizations' truth. There is no arbitrary freshness TTL. Review evidence
age from `captured_at`; matching HEAD format is not proof of freshness.

Out-of-root/traversal diagnostic paths are preserved as unresolved provenance,
not canonicalized into nodes. Invalid paths in snapshots, verified symbols or
constraints reject the input. Repeated path separators normalize explicitly;
dot aliases, case aliases, symlinks and hard-link identity collisions are not
silently merged. Strict overlap identity remains unchanged.

An unresolved finding blocks every component sharing its file. A candidate
cross-component relation blocks both participants without merging them.
Candidate-constraint scope includes both declared files and the canonical files
of every referenced diagnostic. A mismatched participant list therefore cannot
leave its actual diagnostic file as an unblocked candidate. This expands blocker
scope, never graph unions; source mappings and local-policy facts stay intact.
Incomplete query closure is scoped through verified relation endpoints and
explicit non-rejected constraint participants that reference that query.
Arbitrary tool arguments and free-form caveat prose cannot assign affected
files. A local proof's query with unknown pagination is not automatically a
failed localization or local-policy proof.

Unbounded findings, artifact limitations and unused incomplete queries make
global independence unproven, but are not copied into every component's
blocker list. Demonstrated physical mappings, local classifications and
otherwise demonstrated file-local candidates remain visible. This distinction
is not permission to dispatch all candidates or claim global independence.
Unknown private, macro, trait, feature, test or freshness coverage never proves
absence. Hard must-coedit constraints still need their independent approval
and complete correction-specific closure.

This consumer validates normalized pagination totals and statuses; detecting
raw repeated/gapped pages remains the producer's responsibility. Complete
pagination and language/index coverage are reported separately.

## Accounting and outputs

Every `clippy:<id>` and `sonar:<id>` appears once in the provenance diagnostic
inventory. Exact alias metadata preserves both records and Sonar issue keys.
Uncertain aliases and ambiguous matches remain unmerged. The strict identity
`Clippy records + Sonar sites - exact alias pairs = strict union` is checked;
it is **not** a semantic defect count. In particular, a strict union of 620
does not establish 620 distinct semantic defects.

Files with several diagnostics retain all their blockers even if another
finding has a local policy candidate. Components can therefore be blocked
singletons or blocked hard-connected groups. `candidate` means only that no
recorded blocker prevents further review; it is not a safety certificate,
correction approval, scheduling assignment or dispatch authorization.
Zero confirmed multi-file groups is a valid result: unresolved relation and
candidate IDs name the missing joint-edit/closure proof.

Outputs must differ from each other, all CLI inputs, captured evidence and
source files, including filesystem aliases. Validation completes before
publication. Files are staged, flushed and replaced atomically individually;
JSON and Markdown are not a two-file filesystem transaction. A failure during
the second replacement can leave the new authoritative JSON with the old
Markdown; regenerate the report in that case. Temporary output files are
removed on ordinary failures. No input or source file is modified.

Package × `src/tests` mutexes and integration validation remain scheduling
metadata for the parent, not additional union edges. There is no worker-count,
pair-size or fixed dispatch target.

## Fixture validation

```powershell
$env:PYTHONDONTWRITEBYTECODE = '1'
python -m unittest discover -s ops/tests -p test_clippy_constraint_plan.py -q
```

Tests use temporary roots inside the fixture directory and remove them at
completion. They cover shared local-proof replay, exact physical mappings with
unknown external references, false local-completeness claims, scoped blockers
and preserved local facts under unbounded caveats, transitive hard unions, non-diagnostic consumers,
mixed complete/partial hard-constraint closure and mismatched candidate diagnostic participants,
independent local candidates, non-union dependencies, missing/self approval,
stale hashes, root/alias rejection, unknown coverage, Unicode units, overlapping
blockers, provenance accounting, determinism and atomic publication failures.
The integrated 2026-11-14 campaign also ran the real offline CLI on saved direct
MCP captures. It retained all 131 diagnostic files plus one observed consumer,
23 exact mappings and five candidate cross-file boundaries. See the
[bounded work-graph report](../missions/20261114-clippy-partition-graph.md) for
executed commands, verification and coverage limits.
