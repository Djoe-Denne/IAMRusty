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

## Linux-first correction — immediate output parent

Baseline content for this correction is the integrated tooling commit
`db16aa726ca0f44876180ac45b4922110c030c09`; the author's original14 staged additions
were checked to match that commit on the owned paths before editing. No rebase,
contract transplant, commit, Cargo or runtime operation is part of this delta.

The lead's real Linux37-test run (artist/UID1000, Ubuntu24.04 WSL) returned exit1,
one failure/two errors: `new_output()` rejected the public grand-parent above the
private immediate output parent. The briefing
`20261029-k6-linux-first-run-path-invalid.md` is the source of that RED evidence;
the previous security result was static only, not a Linux PASS. Author did not
start, adopt or stop the parent's active WSL lease.

Fix: `private_path(directory=True)` checks that canonical, unlinked, outside-Git
directory's own UID/type/private modes. File validation still requires both the
read-only private file and its immediate private directory, by calling the same
directory guard. `new_output()` still exclusively creates a new output beneath
the immediate private parent. No `/tmp`, RUNNER_TEMP, authority or auth change.

Added a portable injected-stat policy regression: new output accepted below
owned0700 parent even when grand-parent is foreign/public1777; immediate public
or foreign parent is still rejected. It reproduced PATH_INVALID before the fix
(40tests, exit1,6 POSIX skips). After correction: native Windows unittest40tests,
exit0,6 POSIX skips (34 exercised). This injection is not real POSIX evidence.

The Linux regression uses only new test-owned directories: actual public
grand-parent/private parent, private-file read, and refusals of immediate public
parent, private file in public parent, linked parent, existing directory/file and
Git ancestor. Sentinel and output-flood tests now require exact PRIVACY_FAILED
and OUTPUT_LIMIT, so an earlier PATH_INVALID cannot masquerade as coverage.
These6 real POSIX/process tests await the lead's Linux replay after integration.

Impact check: canonical Serena references show private_read/new_output; exact
source matching also finds the tools-root and executable-file checks in
run-linux.py. The canonical server does not contain this unmerged worktree fix;
the targeted patch and native tests are the validation for this candidate.

## S-1 HIGH correction candidate — protected names and bound tool identities

Exact correction baseline:
`db352be1ebe01eb275fd74d8cdee6f7298b99bf1`. Before this delta the author's owned
files matched that integrated content, including the previous immediate-parent
fix; the original14 staged additions and old unstaged fix were not rebased,
reset, recommitted or mistaken for this new delta.

Read the full independent BLOCK briefing
`20261029T2140Z-security-k6-private-directory-db352be.md` before editing. The
previous40-test Linux PASS did not cover substitution by another local UID.
No current exploit, multi-UID PoC or hostile executable was claimed or run.
The parent's acquired tools and WSL lease remain untouched; executable runs and
publication stay frozen pending independent review of this HIGH correction.

RED first: added chain owner/mode and deterministic name-substitution tests against
the old guard. Native Windows Python discovery executed46 tests, exit1,8 failures,
6 POSIX skips. The bad chains and substitutions were not refused. Child execution
was mocked: fixture bytes are non-executable ELF markers, not hostile code.

Production fix:

- `name_chain()` separately checks every component from filesystem root to leaf:
  canonical/no links, directories on the prefix, trusted owner root/effective UID;
  group/other-writable directories require sticky protection. Checking child
  ownership at each next component makes the accepted sticky case name-protected.
  System0755 and correctly owned1777 ancestors remain valid; another UID's0755
  directory and shared non-sticky writers are refused. Confidentiality rules for
  the leaf and immediate parent remain intact. The policy is deliberately
  conservative, not an analysis of all users' effective group memberships.
- Output-parent identity is compared across preflight before exclusive mkdir.
- Internal PinnedTools stores archive/ELF path and ancestry identities before
  hashing, verifies them after hashing, and rechecks before version/inspect/scenario
  child creation and provenance. Dictionary path values cannot detach from their
  bound identity. Changed device/inode, owner/mode or file metadata rejects with
  TOOL_UNPINNED before child execution. Directory timestamps are deliberately not
  identity fields, so unrelated /tmp/sibling entries do not cause false rejection.
- Primary cross-UID protection is the protected chain of names; identity
  revalidation is additional fail-closed detection, not a claim of atomic exec.
  Root and the operator UID remain trusted. No service/HTTP/lease/auth/CI change.

Final author GREEN: native Windows unittest discovery50 tests, exit0,6 POSIX skips
(44 exercised). Includes both trusted owners/system0755/sticky1777 acceptance,
foreign/non-sticky rejection before mkdir or Child creation, parent replacement
before mkdir, tools replacement during the final digest and after hashes before
versions/scenario/provenance/publication, and an unrelated-entry positive witness.
Metadata ownership is injected for portable tests; renames use only fixture
directories. No chown, user creation, system chmod, server or runtime was added.
The real POSIX fixture now distinguishes trusted sticky1777 from non-sticky0777;
its replay is still the parent's responsibility after integration.

Independent review of the HIGH fix and actual Linux replay remain required.
No k6/Chromium version/inspect/browser/HTTP/selftest command executed by this author,
no campaign artefact published, no Cargo or global configuration operation, no
Docker/WSL/Kind resource created/started/stopped and no new owned runtime ID.
