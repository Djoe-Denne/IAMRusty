# Offline Clippy/Sonar overlap inventory

Run from the repository root with Python 3 (standard library only):

```powershell
python -B ops/scripts/clippy-sonar-overlap.py `
  --clippy .tmp/clippy-campaign-20261114/inventory/clippy-production.jsonl `
  --clippy .tmp/clippy-campaign-20261114/inventory/clippy-tests.jsonl `
  --sonar .tmp/clippy-campaign-20261114/inventory/sonar-pages.json `
  --output .tmp/clippy-campaign-20261114/inventory/overlap-script.json
```

Repeat `--clippy` for each raw Cargo JSONL lane. Supply `--sonar` as a saved
Sonar API response object or an array of paginated response objects. Omit
`--output` for JSON on stdout. The tool performs no network calls, subprocess
execution, source edits, issue mutations or Cargo invocations. Output paths
must not alias inputs. An existing output is replaced atomically only after
successful validation; its parent directory must already exist.

## Completeness and identity

Each supplied Cargo capture must end with exactly one successful
`build-finished` row. Invalid JSON, malformed required shapes, failed builds
and missing terminal rows fail with exit status 2 and a stderr explanation.
Only `compiler-message` diagnostics whose code starts with `clippy::` and whose
level is `warning` or `error` enter the inventory. Other compiler messages and
non-compiler rows are counted separately in `source_stats`; they are not findings.

Sonar captures require coherent positive page indexes/page sizes, consistent
totals, all page indexes from 1 through the final page, and exactly
`paging.total` unique issue keys. Non-final pages must be full, and the final
page must contain at least the remaining count (identical repeated keys may
fill otherwise unused page capacity). Empty inventory requires a complete page
with total zero. Repeated identical keys count once; any conflicting metadata
for the same key fails. Duplicate page indexes and contradictory legacy
`p`/`ps`/`total` aliases fail.

Clippy deduplication uses rule, exact message and the sorted complete primary
span set. All input-row, package, level and target provenance is retained.
Sonar deduplication uses normalized rule, component, exact message and full
range (or line/message when no range exists), retaining every issue key.
No-primary Clippy diagnostics with identical rule/message form one unresolved
group; absent location information cannot establish distinct physical sites.
Deduplication applies only within the supplied captures, not across revisions.

## Strict matching and accounting

A cross-source match requires exactly one primary Clippy span, a complete Sonar
range, equal normalized rule, equal exact message, equal normalized file path
and all four equal range coordinates. Cargo columns are converted from 1-based
to Sonar's 0-based offsets; line numbers stay unchanged. Paths use forward
slashes, with case and dot segments preserved. Dot segments are not resolved:
compiler include paths and symlinks need not identify the same captured site.
Only a
declared Sonar `project:` component prefix is stripped. Absolute roots are not
guessed, and source files are never opened.

Only `external_clippy:x` maps to `clippy::x`. Native `rust:S...` rules are not
automatically mapped to Clippy synonyms. No line-offset arithmetic, numeric
message normalization, substring or fuzzy matching is used. Multiple primary
spans, absent spans/ranges and ambiguous exact candidate groups remain unmerged,
with explicit reasons. Matching is one-to-one after source deduplication.

The JSON contains source unique counts, raw unique Sonar keys, exact
intersection, matched counts, unmatched counts and:

```text
strict_union = clippy_unique + sonar_unique_sites - exact_intersection
```

Coverage percentages report intersection divided by Clippy unique, Sonar unique
and strict union, respectively; empty denominators yield null. Stable IDs link
matches and unmatched inventories to Sonar keys and Clippy provenance. Ordering
is deterministic for the same input paths and content; IDs are inventory-local,
not persistent identities across runs.

`unmerged_hints` separately reports pairs whose two sites remain unmatched:

- Same rule/file/start line/exact message, but different full ranges or multiple
  primary spans (including an absent Sonar range when a line is available).
- Same rule/file/full range but different exact messages, for example
  platform-dependent future-size byte counts.

Hints never contribute to the intersection. **Unmatched does not prove semantic
source-exclusivity.** Strict union can retain the same underlying issue twice
because of text/range differences, OS, source revision or coverage. The script
does not verify Git revisions, toolchain/features, submodule scope or capture
freshness; record these separately before using inventory counts for decisions.

## Validation and policy references

```powershell
python -B -m unittest discover -s ops/tests -p test_clippy_sonar_overlap.py -v
```

Fixtures cover strict range conversion, provenance deduplication, pagination,
malformed/incomplete captures, missing/multiple locations, ambiguity, changed
messages/ranges, native-rule separation, stable output and output preservation
on failure. No application runtime or external dependency is needed.

This inventory follows the
[Clippy campaign guide](clippy-campaigns.md) and the **Campaigns: Clippy first,
Sonar as a complement** section of
[aiforall-sonar-policy](../../.agents/skills/aiforall-sonar-policy/SKILL.md):
preserve lane provenance, use source positions only for their capture and never
convert failed/partial analyses to zero. The
[decision registry](clippy-decisions/README.md) remains the correction-policy
reference; this tool neither fixes diagnostics nor creates new accepted
correction precedents.
