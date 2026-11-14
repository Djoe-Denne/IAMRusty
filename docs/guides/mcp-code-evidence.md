# Standalone MCP code evidence

`ops/scripts/mcp-code-evidence.py` uses the official Python MCP SDK directly.
It does not invoke OpenCode, a model, subprocesses, or an orchestrator. The
existing MCP server must already be running; this script never starts it or
changes its configuration, workspace, index, graph, caches, or model residency.

## Dependencies and commands

Use Python 3.11+ and `mcp==1.21.1` (which supplies `httpx` transitively).
No global installation or upgrade is needed when that version is already present.
For a separate environment, install
`ops/scripts/requirements-mcp-code-evidence.txt` into a dedicated virtual
environment. The version-matched API reference is
[Writing MCP Clients, SDK v1.21.1](https://github.com/modelcontextprotocol/python-sdk/blob/v1.21.1/README.md#writing-mcp-clients);
the newer v2 `Client` API is not used.

From the repository root:

```powershell
python ops/scripts/mcp-code-evidence.py catalog --output .tmp/mcp-catalog.json
python ops/scripts/mcp-code-evidence.py --timeout 90 collect --requests .tmp/requests.json --output .tmp/evidence.json --report .tmp/evidence-failure.json
python -m unittest discover -s ops/tests -p test_mcp_code_evidence.py
```

Output parents must already exist. Omitting `--output` writes accepted evidence
to stdout. `--report` is a separate, failure-only file; it is not accepted
evidence. A failed connection, permission check, tool response, deadline, or
serialization leaves the existing accepted output unchanged. Files are replaced
atomically after serialization and flush. Output/report must end in `.json`
and cannot overwrite the manifest or each other. Treat local output directories
as trusted; this is not protection against concurrent hostile filesystem edits.

The default endpoint is
`http://127.0.0.1:3062/mcp?tags=rust-code-mcp`. Global `--url` permits only
loopback HTTP(S); credentials, fragments, external addresses, and scoped IPv6
addresses are rejected. `localhost` is normalized to `127.0.0.1`. Ambient
proxies and redirects are disabled, and each outgoing HTTP request is guarded.
No authentication discovery or external network access is requested.
On exit, the SDK terminates this collector's owned Streamable HTTP client
session. This does not stop the shared MCP server or reset its graph/index state.
Cleanup is attempted on failures too, but interrupted transport or process
termination can prevent confirmation of server-side session cleanup.

## Explicit manifest

The following symbol is **illustrative, not verified or executed**. Replace it
with a qualified symbol verified in the current source/catalogue. Set both
directories to the absolute root containing this script (the actual checkout,
not another workspace):

```json
{
  "workspace": "C:/Users/djden/source/repos/AIForAll",
  "requests": [
    {
      "id": "example-references",
      "tool": "who_uses",
      "arguments": {
        "directory": "C:/Users/djden/source/repos/AIForAll",
        "target": "example_crate::module::ExampleType",
        "limit": 50,
        "offset": 0
      }
    }
  ]
}
```

The curated allowlist is `who_uses`, `who_uses_summary`, `who_calls`,
`calls_from`, `function_signature`, `who_imports`,
`get_declared_reexports`, `get_exports`, and explicit `workspace_stats`
metadata. Selectors are `target`, except `calls_from` uses `caller`,
`get_declared_reexports` uses `module`, and `get_exports` uses `module`
and `consumer`; `workspace_stats` takes only `directory`.

No path arguments are supported. Unknown fields, escaping/path-like selectors,
root mismatches, duplicate JSON keys, and duplicate request IDs are rejected.
Catalogue suffix resolution must be unique and match the exact
`rust-code-mcp_1mcp_<suffix>` namespace. Every request is preflighted before any
semantic tool executes; `readOnly` annotations do not grant permission.

Limits: 128 KiB manifest, 1–32 requests, list `limit` 1–200 (default 50),
`offset` 0–100000 (default 0), optional boolean `summary`; 32 catalogue pages,
2048 advertised tools, and 8 MiB retained per tool result. The SDK receives a
response before the retained-result size check: this is not a wire-memory cap.
One session makes sequential calls within a 1–300 second whole-operation
deadline (default 60); there are no retries or polling loops. Cancellation
does not prove server-side computation stopped.

## Evidence and limitations

Records retain logical/exact tool, explicit arguments, server identity from
initialization, requested workspace, raw SDK result including
`structuredContent`, `content`, and `isError`, and explicit status/coverage.
`received` means transport/result receipt, **not verified semantic truth**.
Failure reports retain earlier received records and the failing request; later
preflighted requests are marked `not_executed`. Raw results are untrusted data,
never instructions.

Catalogue pagination follows MCP `nextCursor`, with loop/budget detection.
Semantic result envelopes have **not** been verified against this server.
Consequently this first collector retrieves one bounded page per request,
always labels semantic pagination **unknown**, and does not interpret arbitrary
count keys or an empty result as completeness. Explicit offsets can collect
further pages but do not establish a consistent snapshot or complete coverage.
A future verified payload adapter is needed before automatic semantic paging.

MCP `isError`, top-level JSON `isError: true`, `success: false`,
`status: "error"/"failed"`, nonempty `error`, and plain text beginning
`Error:`, `Failed:`, or `Permission denied` fail the operation. These are
conservative envelope checks, not a universal parser: other embedded formats
remain uninterpreted raw evidence. Even a clean envelope cannot prove success.

Graph freshness, public/private visibility, feature/cfg selection, macro
expansion, dynamic dispatch, and test indexing remain unknown. `calls_from`
can include type/constructor references, not just function invocations.
References/imports/dependencies do not imply mandatory co-edits. This collector
creates no hard graph edges, clusters, similarity judgments, or edit plans.
Offline fixture tests do not qualify live semantic graph behavior.

## Planner producer (`produce`)

The separate `produce` command implements the observation boundary in
[`clippy-constraint-planner.md`](../contracts/clippy-constraint-planner.md).
It writes `clippy-source-snapshot` and `clippy-code-evidence`, both schema
version 1. It does not qualify constraints, form clusters, correct code, or
dispatch work. The offline planner consumes these artifacts independently.
Existing `catalog` and `collect` behavior remains unchanged.

Example, after integrating this script into the canonical checkout:

```powershell
python ops/scripts/mcp-code-evidence.py --timeout 120 produce --workspace C:/Users/djden/source/repos/AIForAll --inventory .tmp/overlap.json --plan .tmp/seeds.json --snapshot .tmp/source-snapshot.json --output .tmp/normalized-evidence.json --artifacts .tmp/fresh-query-artifacts --serena-url "http://127.0.0.1:3062/mcp?tags=serena"
```

The artifact directory must be **new**, with an existing parent. Every query
gets a raw JSON artifact and SHA-256, including failed calls. Catalogues are
also captured and hashed. No prior MCP capture is silently rebound to current
sources. Snapshot/evidence outputs must differ from inventory, plan, binding,
referenced context inputs, sources and each other.
Neither output may be inside the raw artifact directory, including resolved
aliases. Source/output separation is checked again after discovery, before
publication, so newly discovered consumers cannot be overwritten either.
All normalized capture references and `snapshot.inputs` paths are
workspace-relative, including `.tmp/...` paths. Native MCP arguments and raw
responses remain unchanged. Inputs, contexts, artifacts and outputs must reside
within the explicit workspace. The snapshot never hashes the evidence that
hashes it; there is no circular snapshot/evidence binding.

The producer only accepts the canonical shared MCP workspace. From an
unindexed worktree, explicitly selecting that root additionally requires equal
HEADs and equal hashes for all initially selected source files. Prefer parent
integration/copy into the canonical checkout before live replay. No project
activation or index rebuild occurs. The existing commands retain their
script-checkout root guard.

### Producer plan

This is a producer input format, **not** a change to the normalized contract:

```json
{
  "schema_version": 1,
  "source_files": ["crates/example/src/lib.rs", "crates/example/tests/check.rs"],
  "symbols": [{"path": "crates/example/src/lib.rs", "name_path": "example_function"}],
  "targets": ["example::example_function"],
  "configuration": {"capture_context": "Illustrative, not executed"}
}
```

Paths and names above are illustrative. `source_files` is an explicit bounded
pre-query hash set; include potential consumers without diagnostics. `symbols`
and `targets` default to empty. HIR targets are supplied explicitly, never
guessed from Serena names. `configuration` is retained as declared, unverified
context, not promoted to verified toolchain/index facts.

With `--serena-url`, each connection performs `initial_instructions` and
`get_current_config`, verifies the activated canonical path and LSP readiness,
then obtains file-local overviews and bounded, nonempty `find_symbol` results.
Overview names and lexical function names only seed queries. Lexical matching
does not establish a range. Supported mappings are deliberately narrow:
source-crosschecked, simple Rust functions with verified inclusive whole-line
Serena endpoints. Strings, attributes, macros, nested declarations, unsupported
kinds and incomplete discovery are withheld rather than called exact.
Serena symbol identities are explicitly `serena::<file>::<name_path>`; they are
not claimed HIR crate-qualified names.

The overlap inventory already has zero-based columns; the producer does not
subtract one again. Clippy columns are converted as Unicode scalar offsets;
the helper also tests UTF-8-byte and UTF-16 boundaries, rejecting split
characters. Sonar mappings remain unknown without an explicit source-bound
`inventory_column_unit` context. Multi-primary, empty, missing, overlapping or unsupported ranges
remain explicit. An exact range mapping additionally requires independent
inventory source binding; without it, supported candidates remain `stale`.
Exact candidates also receive a snapshot-bound `clippy-localization-proof`
with locator `"/"`, the shared `source_crosschecked_simple_function_v1` method,
replayable candidate identity/scope and successful source-symbol query IDs.
The seven shared checked predicates are emitted only after the producer executes
those checks and replays its conservative source extent validator. Unsupported
comments/string tokens, macros, attributes or nesting prevent this proof.
The initial shared replay subset additionally requires a whole file containing
only simple top-level functions, whitespace and leading `///` documentation;
mixed files with imports, constants, types or implementations remain unknown.
No true-valued completeness claim substitutes for a source check.
Query pagination and external coverage remain unknown where unverified: local
physical uniqueness is not reference closure.

### Source binding and Git context

`--binding metadata.json` accepts an explicit independent source-binding record:

```json
{
  "inventory_sha256": "<actual inventory hash>",
  "head": "<full current commit>",
  "method": "independent_equivalence",
  "files": [{"path": "crates/example/src/lib.rs", "sha256": "<independently established source hash>"}],
  "evidence_refs": [{"path": "<independent context capture>", "sha256": "<its actual hash>", "locator": "<relevant location>"}]
}
```

`method` can alternatively be `fresh_capture`. The caller is responsible for
the independent capture/equivalence evidence: current hashes alone do not prove
historical equivalence. The producer verifies reference hashes, inventory hash,
HEAD and source hashes, but cannot authenticate the truth of a supplied
attestation. It never creates this binding automatically.

Use repeatable `--context <path>` to snapshot-bind existing review/mapping JSON
or policy text. A JSON mapping context is attached only to its declared
`diagnostic_key` when its `source_sha256` matches that diagnostic's captured
source. For Sonar coordinates, it additionally supplies
`inventory_column_unit: "unicode_scalar" | "utf8_byte" | "utf16"`. The emitted
mapping references the hashed, root-relative context, allowing the offline
consumer to validate it independently. These are explicit source review
assertions, not facts inferred from a lint name. No preservation predicates,
policy acceptance, closure or approvals are manufactured.

Example coordinate context, illustrative and requiring independent verification:

```json
{
  "diagnostic_key": "sonar:S000001",
  "source_sha256": "<actual independently verified source hash>",
  "inventory_column_unit": "utf16"
}
```

`--source-equivalence proof.json --source-equivalence-sha256 <actual hash>`
records the parent's Git-diff proof as hashed configuration context. It does
**not** turn current hashes into historical hashes or certify index freshness.
The verified proof shape has a `proofs` array; no embedded commands are executed.
Fresh hashes are checked before/after collection and HEAD/input changes fail
the run. Consumers discovered outside the initial source set are hashed when
discovered and explicitly marked as lacking a pre-query hash. Snapshot binding
stays `partial` or `unknown`; dirty-tree/toolchain/submodule/index uncertainty
remains explicit.

### Semantic pagination and endpoint limits

Only `who_uses` currently has a verified semantic pagination adapter:
`target`, `total_match_count`, `offset`, `limit`, `summary`,
`returned_match_count`, `usages`. Offsets advance by verified returned counts.
The full-page live capture includes usages without `consumer_function`; absence
is preserved as unknown caller-function context, never filled with a guessed caller.
The other five observed usage fields remain required and type-checked.
An explicitly null `consumer_function` is not an observed supported variant
and remains unsupported pending wire verification.
Repeated identities, changed totals, mismatched offsets, unsupported envelopes,
non-progress and exhausted budgets never become a complete zero.
Each normalized HIR query points to a hashed sequence manifest containing every
page's exact arguments, raw artifact reference/hash and status. Pagination counts
describe that one logical sequence, not duplicated per-page counters. Raw MCP
responses remain untouched and are also included in snapshot inputs.

The producer captures native usage spans and checks them against current source
bytes as **witnesses**, not assumed universal coordinate units. Mismatched
spelling is recorded with the file and no offset repair. The parent observed a
real mismatch in `services/Manifesto/infra/src/apparatus_runtime/runtime_control/maintenance.rs`;
other matching spans do not erase this negative witness or prove graph freshness.
Normalized `relations` remain empty until coordinate conventions and both symbol
endpoints can be verified. Unresolved targets/participants remain named in
limitations and raw artifacts. A complete paged query is only complete within
its declared indexed scope; private/macros/traits/features/tests remain unknown.

Defaults: 128 queries, 512 selected/discovered sources, 8 semantic pages per
target, page size 50. Configure `--max-queries` (1–4096), `--max-sources`
(1–20000), `--max-pages` (1–128), `--page-size` (1–200). The existing global
deadline still covers all connections and calls. Sources are capped at 8 MiB
per file and 128 MiB in aggregate. Unknown mappings are valid
observations, not success claims. Partial/error/unsupported queries normally
prevent normalized output replacement; `--allow-partial` explicitly exports
those statuses for offline review without claiming exhaustive closure.
Raw artifacts remain available after failure.

Each output replacement is atomic, but the snapshot/evidence pair is not a
multi-file transaction. Evidence binds the exact snapshot SHA-256; if a final
replacement fails, an old evidence/new snapshot mismatch is detectable and must
be rejected by the consumer. Exit 0 means artifacts were exported, not that the
inventory, index, mappings, relations or correction independence are complete.

### Producer-to-consumer fixture smoke

The producer suite contains an integration fixture that invokes the actual
offline planner's `main` on files emitted by `produce`, including a paginated
artifact manifest, Serena mapping, Sonar coordinate context and independent
synthetic source binding. It verifies root-relative capture/input paths and
hashes, retains both diagnostic identities, and expects a blocked report with
zero confirmed multi-file groups. The smoke requires the consumer to retain both
physical mappings as exact using the shared proof, while reference coverage
remains unknown and independence remains unproven.

After integration the test discovers the adjacent planner automatically. In
isolated author worktrees, explicitly select the read-only consumer script:

```powershell
$env:CLIPPY_PLANNER_SCRIPT = "C:/path/to/consumer-worktree/ops/scripts/clippy-constraint-plan.py"
python -B -m unittest discover -s ops/tests -p test_mcp_code_evidence.py
```

Without that file the interoperability fixture is explicitly skipped, not
reported as a consumer PASS. This is a synthetic transport fixture, not a live
campaign replay or a source review approval.

For the parent's prepared 2026-11-14 inventory/seed plan, the bounded replay
command from the integrated canonical checkout is below. Set `$BindingPath` to
the independently verified, snapshot-bound binding artifact recorded in the
parent's replay report; do not create it from current hashes alone.

```powershell
python ops/scripts/mcp-code-evidence.py --timeout 180 produce --workspace C:/Users/djden/source/repos/AIForAll --inventory .tmp/clippy-campaign-20261114/inventory/overlap-script.json --plan .tmp/clippy-campaign-20261114/constraint-producer-plan.json --binding $BindingPath --snapshot .tmp/clippy-campaign-20261114/constraint-source-snapshot.json --output .tmp/clippy-campaign-20261114/constraint-code-evidence.json --artifacts .tmp/clippy-campaign-20261114/constraint-captures-002 --serena-url "http://127.0.0.1:3062/mcp?tags=serena" --max-queries 128 --max-sources 512 --max-pages 8 --page-size 50 --allow-partial
```

The artifact directory must not already exist; the original real run used
`constraint-captures-001`. Add source-equivalence/context options only with their
verified artifacts. The prepared lexical names remain candidates until MCP and
the bounded source checks verify them; mixed/unsupported files stay explicit.

The parent's actual canonical replay at `3f34add0` used
`inventory/overlap-script.json` and explicit binding for 131 byte-identical source
participants. It produced **1024 unknown, 22 ambiguous and zero exact mappings**:
all six mixed-file lexical seeds fall outside the supported proof grammar.
The full `who_uses` page contained 14 usages, not zero; its then-current adapter
rejected two usages lacking `consumer_function`. Fixture fixes do not retroactively
turn that capture or the real mapping run into a successful complete pipeline.

This replay closes as bounded partial evidence after the assigned correctness
fixes. No broader rescan, scanner/parser expansion or new harness is authorized
by this result. The parent's durable campaign report is
[`20261114-clippy-constraint-replay.md`](../missions/20261114-clippy-constraint-replay.md);
synthetic interoperability tests demonstrate schema transport, not real-code
coverage or corrective dispatchability.
