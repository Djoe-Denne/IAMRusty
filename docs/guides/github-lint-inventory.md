# Read-only GitHub lint issue inventory

Run from the repository root with Python 3.9+ (standard library only):

```powershell
python ops/scripts/github-lint-inventory.py --repo Djoe-Denne/IAMRusty
python ops/scripts/github-lint-inventory.py --input .tmp/clippy-campaign-20261114/github-issues-pages.json --output .tmp/github-lint-inventory.json
python -B -m unittest discover -s ops/tests -p test_github_lint_inventory.py -v
```

Choose exactly one of `--repo owner/repo` and `--input path`. JSON goes to stdout
unless `--output` is supplied. The live mode requires an installed, authenticated
GitHub CLI and executes only this subprocess argument vector, without a shell:

```text
gh api --method GET repos/{owner}/{repo}/issues --paginate --slurp -f state=open -f per_page=100
```

There is no automatic labeling, closing, assignment, issue creation, correction,
or orchestration. Offline mode never invokes `gh`. Fetch failures, invalid JSON,
unsupported shapes and malformed issue records return a nonzero status, not a
zero-issue report. Server stderr and raw bodies are not copied to the report.

## Input, evidence and completeness

Input is either a REST flat issue array or the array of pages produced by
`gh --paginate --slurp`. Every record requires a positive integer `number`, string
`title` and `html_url`, `state` equal to `open` or `closed`, and an array of labels
(strings or objects with string `name`). `body` can be absent, null or a string.
Additional REST fields are ignored; this is not a GraphQL/search/export importer.

Pull requests are excluded first, then closed issues. All remaining issues,
including non-lint and unknown diagnostics, stay in the inventory. Numbers are
deduplicated across retrieved pages. Duplicate open observations contribute their
union of labels, rule IDs and path evidence, with one-based page provenance.
Conflicting titles/URLs set `display_metadata_conflict`; the lexicographically
smallest title/URL pair is displayed. Nothing resolves concurrent state changes.

Source metadata records mode, input path or repository/command, page count and a
SHA-256 of decoded input text encoded as UTF-8. JSON keys, issues and extracted
sets are sorted; identical input and source metadata produce identical output.
Input byte ordering can change the source digest; page reordering also changes
provenance. There is no nondeterministic generation timestamp.

Counts distinguish retrieved records, excluded PRs/closed issues, open records,
duplicates, unique open issues and source families. Completeness means **all
records within retrieved pages were processed**, not that a repository snapshot
was proven. Successful live pagination is recorded as `gh_paginate_completed`.
Offline exports cannot prove pagination completeness. API permissions, concurrent
changes and collection time still limit both modes; a stale snapshot is not a
current inventory. PR and closed exclusion counts are mutually exclusive.

## Mechanical categories, not fix decisions

- `source_family`: `clippy`, `sonar`, `mixed` or `other`. Explicit IDs are
  `clippy::<identifier>` and `<language>:S<digits>` (including `rust:S2208` and
  `secrets:S6706`), found in titles/bodies or matching an entire label. IDs are
  syntax hints, not verified diagnostics or mappings to fixes. Unknown future
  IDs are retained.
- Additional text anchors are whole words `clippy`, `sonar`, `sonarqube` and
  `sonarcloud`, case-insensitively. Relevant labels match those exact names or
  their `lint:`-prefixed forms. `unclippy`, `sonarization` and `not-clippy` labels
  do not match. An exact anchor can still occur in ordinary prose; this is a
  mechanical classification, not an assertion that an issue is a valid finding.
- `paths`: explicit forward-slash relative file tokens under `services`,
  `workers`, `crates`, `runtime`, `config`, `docs`, `ops`, `templates`,
  `permission-fixtures`, `rustycog`, `.github` and `.cargo`, plus a small fixed
  list of root files in the script. `./` and trailing line references are allowed.
  Absolute paths, URLs and traversal are not accepted. Directory-only references,
  paths containing spaces, Windows backslashes and unrecognized roots are not
  inferred. Paths are not checked for existence or read from disk.
- `areas`: `services/name`, `workers/name`, `crates/name` and `runtime/name`,
  derived only from file references under those prefixes.
- `dispersion`: `single_area`, `multiple_areas` or `unknown` describes referenced
  areas only. Missing paths/rules have explicit `unknown` status, not a guessed
  location or diagnostic. Files outside these area roots do not invent an area.
- `impact` is **always `needs_review`**. A single area, single file, title saying
  "local", or lint ID cannot prove a local correction. Public signatures,
  callers, traits, behavior and component boundaries require actual source review.

Evidence includes only matched IDs/anchors/paths, their source field and page;
it never includes body snippets. Titles, URLs, labels and extracted hints remain
untrusted and can themselves contain sensitive data: this is not a general secret
redactor. Keep exports private as appropriate. Never execute issue bodies, treat
paths as commands, or regard any identifier/path as permission to edit.

Before an actual correction, consult
[`aiforall-sonar-policy`](../../.agents/skills/aiforall-sonar-policy/SKILL.md),
especially "Consult, apply and preserve decisions" (the existing French heading
is "Consulter, appliquer et conserver les décisions"), and the
[decision registry](clippy-decisions/README.md). Apply precedents only in their
documented context; proposed decisions are not shared policy. The classifier
does not invent a rule-to-fix mapping or replace source/caller analysis.

The parent collection on 2026-11-14 observed one empty page for
`Djoe-Denne/IAMRusty`: **zero open GitHub issues**, not fabricated lint findings.
That historical observation is separate from Clippy/Sonar diagnostics and from
any later live query.
