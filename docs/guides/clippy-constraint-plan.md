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
  --constraints .tmp/campaign/constraints.json `
  --output .tmp/campaign/plan.json `
  --report .tmp/campaign/plan.md
```

Omit `--constraints` when no independently reviewed hard constraints exist.
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

This is not a general Rust parser. A function in a file containing imports,
impls, structs or unsupported syntax may therefore remain unknown until a
shared proof-method extension is authorized. A forged all-true proof with an
omitted nested function fails replay. Query success or a single supplied seed
never substitutes for these checks.

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
There is no live-data replay in this leaf lot; the parent owns producer
integration, actual campaign reporting and representative source review.
