# k6 E2E scenario registration (Proposed)

STORY-020 extends the existing [Linux k6 contract](k6-three-scenario-ubuntu-ci.md).
The Windows k6 adapters are retired. Automatic post-merge E2E and the ordinary
PR gate retain their independent policies.

## One declaration

Add a standard k6 script under `ops/tests/k6/scenarios/` and add one object to
`scenarios/catalog.json`. The version-1 document has `version` and an ordered
`cases` array. Each case has `id`, `script` (relative to `scenarios/`), `phase`,
`lifecycle`, `timeout_seconds`, `requests`, and a `checks` object keyed by check
name (`functional` or `cleanup`). A `standard` case uses phase `HTTP`, the
prepared API target and the existing bounded k6 executor. `requests` is the
exact total of k6 HTTP requests, including normal setup and teardown.

Selection is the full ordered catalogue. Filenames and IDs are bounded,
relative, and non-escaping; missing files, symlinks, duplicate IDs or JSON
members, unknown fields, and empty or invalid check sets fail closed. There is
no path glob, arbitrary command, or hook. Explicit selection avoids running
helpers, diagnostic stimuli, or private-input pilots merely because they are
JavaScript files.

The `san` triplet stays contiguous. It and the `oauth` browser case retain their
explicit lifecycles. The SAN case must restore the good certificate and perform
the positive restored stimulus even when the bad phase fails. The OAuth case
retains its isolated browser and independent causal correlation. These
specialized setup and cleanup operations are not declared as runnable commands.

## Standard scenario interface

`lib/case-contract.js` exports `scenarioContract(id)`. Use its `thresholds` in
standard k6 options and call `start()`, `check(value, predicates)`, and `finish()`
in the scenario. Export `handleSummary` using `contract.summary(data)`. Use
one VU and one iteration, and keep the `group` tag. Repeat or omitted named
checks fail even when all observed checks pass. This contract records assertion
execution counts independently of k6's aggregate success rate. The summary
retains the version-1 shape.

Use the same script with `k6 run` on an already prepared, authorized, isolated
target. Set `K6_RUN_ID`, `K6_ORIGIN`, `K6_CAMPAIGN_FILE` and `K6_SUMMARY_PATH`
from that target, with its documented TLS trust configured. Use a fresh,
private summary destination. A direct run does not start the lab or establish
the full campaign's independent isolation and correlation evidence.

Use k6 `setup()` and `teardown()` for API-created fixture data. Track successful
and ambiguous effects using the run's known identities. If setup fails after a
write, compensate within setup before rethrowing; k6 does not call teardown
after an abnormal setup. If compensation cannot be ensured (including process
termination), the existing external owner must dispose of the entire isolated
run environment. Never treat a shared, persistent target as disposable.

## Evidence and publishing

Python's `ci/evidence.py::load_catalog(path)` validates the same file and
returns an ordered mapping from case ID to its record. `CASES` is derived,
not a second authored list. k6 owns assertion expressions; the receiver
checks that a fresh, bounded summary is from the selected run and reports the
declared counts. No nonzero child exit can be admitted. The independent
image, endpoint, parent correlation, browser and cleanup proofs remain in
force. Private scenario attempts and outputs are scoped below `private/cases/`
so case IDs do not share names with harness resources.

The public campaign `selection.json` is an evidence receipt, not a second
configuration. Its fields are `run_id`, `catalog_sha256`, `cases` (ordered
IDs), `harness_sha`, and `target_sha`. Accepted summaries are published under
the fixed `public/cases/` directory; a scenario never writes directly there.
Only the validated results and explicit existing allowlist are published, not
private summaries, k6 streams, or fixture inputs.

The public projection distinguishes `functional`, `incomplete`,
`infrastructure`, and `cleanup` failures without changing the existing
`passed`/`failed`/`incomplete` status contract. An observed failed check is
different from one that was not executed. All selected cases remain in the
denominator even if a failure prevents later execution.

## Hosted PR qualification

Use `/k6-e2e` as a repository writer on an internal PR targeting `master`.
Resolve the current open PR's merge, head and base before checking out its
harness. The newest ordinary CI run/attempt for that exact PR revision must
have all three saved non-instrumented build artifacts. Verify their GitHub
identity and archive digests, and the manifests, ELF metadata and payload
hashes, before executing the same catalogue. The current PR merge must still
match when the workflow finishes. See the [hosted runbook](../../ops/tests/k6/ci/README.md#run-the-complete-campaign-on-a-pr).

No fallback to a previous attempt or `master`, no application compilation,
and no new deployment framework. The hosted result does not attest WSL
browser or OAuth behavior, replace Rust CI, or publish the `master` badge.
