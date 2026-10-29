# Implementation record

Base: `c9b537c9c84f11dbdaf0178a1d19666de9261989`, ratified contract V1,
SDK gitlink `02e5db23aac6016aab062c3078696d5f7fd464ef` (not modified).

Plan (single tooling author):

1. RED: strict summary, fixed input and confidentiality tests, before the runner.
2. GREEN: local JS modules, explicit one-case scenarios, closed Python launcher,
   process supervision and fail-closed target gate. No Rust changes.
3. Refine: adversarial subprocess/permissions/cleanup tests; explicit provisioning
   separated from execution; same launcher consumed by CI.
4. Integrate/review pins and trust boundaries before real Linux qualification.
   HTTP and negative live launches require the parent's current POSIX lease and
   verified origin binding. A missing bridge is not replaced by an attestation.

No runtime or target was supplied to the tooling author. Real HTTP, Chromium,
sandbox and campaign cleanup are not established by local unit tests.

## Actual author evidence (2026-10-29)

- Confirmed clean isolated branch `ensemble-f94542fb-b57a-4c90-8958-53322b984f7b`
  at the base above; the ratified contract exists in that committed HEAD.
- RED: native `C:/Python313/python.exe -B -m unittest discover -s
  ops/tests/k6/tests -p test_*.py` exited1 before `run-linux.py` existed.
- GREEN/refinement: the same native Windows command finally executed36 tests,
  exit0, **5 skipped** (real POSIX permission/process checks),31 exercised.
  An intermediate cross-platform test expectation used POSIX slashes on Windows;
  corrected the test to compare `str(Path(...))`, not a production guard bypass.
- Native Node `--input-type=module --check` with each of the three JS sources
  supplied on stdin: exit0 for all. Syntax only, **not** `k6 inspect` or execution.
- Python AST parsing of all6 Python sources, both public JSON configurations,
  and YAML parsing of the workflow with existing PyYAML: exit0. Workflow parsed
  permissions `contents:read` and runner `ubuntu-24.04` checked explicitly.
- Explicit in-memory HTTPS acquisition of official k6/Chrome archives, safe
  in-memory member reads (no extraction/execution on this host): k6 archive
  30,847,764 bytes / Chromium archive201,336,602 bytes; archive and ELF SHA256
  recorded in the lock. k6 archive matches the contract. Chrome candidate lock
  still needs independent review before qualification.
- Official GitHub tag-ref API confirms checkoutv4.2.2 commit
  `11bd71901bbe5b1630ceea73d27597364c9af683` and upload-artifactv4.6.2 commit
  `ea165f8d65b6e75b540449e92b4886f43607fa02`.
- `git diff --check`: exit0 before staging (new files were separately checked
  by tests/syntax). Final `git diff --check` and `git diff --cached --check`:
  exit0 with all14 owned candidate files staged; no commit/hook bypass.

## Not executed / remaining acceptance

- Exact `python3 ... unittest` under Linux not executed: this Windows session's
  `python3.exe` resolves to the WindowsApps alias, not an admitted Linux runtime.
  The native Python command above is not silently represented as the Linux gate.
- `k6 version`, Chromium `--version`, both `k6 inspect` commands, and Linux
  `validate`, `run`, `browser`, `selftest`: **NOT EXECUTED**. No verified/pinned
  Linux installation, configured campaign, parent runtime lease or target supplied.
  `k6` also was not present in this session's PATH; no PATH fallback attempted.
- The five POSIX tests need the same suite on Linux, including child cleanup,
  descendant interruption, existing foreign child preservation and overflow.
- `live_target()` deliberately ends in `TARGET_BINDING_UNQUALIFIED` after the real
  existing cluster check. It cannot yet authorize an HTTP child even if a caller
  supplies a syntactically valid config/lease. Parent/architecte must resolve the
  actual existing guarded origin binding, not remove the gate declaratively.
- CI has not been dispatched, run or claimed green. Chromium compatibility,
  dependencies, actual binary versions, effective scenario options and sandbox
  witnesses all require Linux execution after pin review.
- No Rust, Cargo, service/identity, SDK gitlink or existing CI changes. No runtime
  started, Docker image/container/volume created/restarted or lease adopted.
  Docker owned IDs: none; no registry line invented and no global teardown done.
- No commit/push/branch cleanup performed by this author. Candidate files are
  handed back for lead integration/review; mission closure remains the parent's.

## Continuity — public exposure addendum

Lead message and canonical ledger read after the first candidate: addendum
`3f2e8fbe625b47fbc4093e673dc2804bcf9a3023` is ratified. Its contract section
"Incompatibilité publique du trajet étudié" was read in the canonical checkout:
the reviewed sources preserve fail-closed Envoy ext_authz for `/iam/` and no
renderer health exemption. **No live403 is claimed.** The author worktree stays
at `c9b537c`; no contract files or service/authentication code are changed here.

The existing unconditional fail-closed origin-binding gate remains in place.
Added a CLI-level regression asserting that absent public-target proof causes
nonzero/HTTP INCONCLUSIVE before tool verification/execution or any k6 child.
The parent must identify an existing authorized public exposure and integrate a
ratified architecte amendment before activation. No token, exemption, tunnel,
arbitrary target field, changed oracle or extra runtime was introduced.

Revalidation after this delta: native Windows Python unittest discovery executed
37 tests, exit0,5 POSIX skips (32 exercised). Linux/k6/Chromium/live commands
remain unexecuted; no new runtime resource or lease was created/adopted.
