# k6 mission1 — Linux functional foundation

Contract: [V1 Accepted](../../../docs/contracts/k6-functional-linux.md),
[ADR0504](../../../docs/adr/0504-validation-fonctionnelle-linux-k6.md).
Only one public `GET /iam/health`, exact `200` / `OK`, and a **separate** Chromium
prerequisite. This is not load testing, readiness, OAuth, authz or a deployment.
No Rust service, SDK gitlink or existing CI workflow is modified.

## Current admission and missing evidence

**Partial evidence, not a qualified mission.** Published source783fa80, observed
in run40982576736/source6b58108, completed129Linux tests without skips on22.04
and24.04. Pinned k6v2.3.0 and Chrome for Testing156.0.8078.73 versions, ELF hashes
and both scenario inspections were executed successfully on both platforms.
Direct22 reported SIGABRT before cleanup, with privacy and child cleanup PASS;
the cause and sender remain unknown. Direct24 still reported no usable sandbox
and privacy FAIL. These observations do not qualify Chromium or HTTP.

Live HTTP, live selftests, the required browser prerequisite and parent-runtime
cleanup remain INCONCLUSIVE. The format-classification delta described below has
only local fixture evidence until separately checked; the129Linux results belong
to783fa80, not to this new code. Fake-command tests prove launcher behavior only,
never an application or native browser outcome. Initial acquisition history is
retained in IMPLEMENTATION and the mission ledger.

`live_target()` calls the existing `LocalClusterTargetAnchor.check()` without
runner/root/privacy overrides and then **refuses HTTP**, with
`TARGET_BINDING_UNQUALIFIED`. This intentional incomplete integration preserves
contract lines103–125: a cluster ancre and a JSON observation reference do not
prove the origin mapping. No config field, environment variable or CLI switch
lifts it. Even a syntactically valid, historically anchored lease is insufficient.

The parent must supply/qualify the existing guarded POSIX observation path tying
the current lease to the existing Linux exposure, Envoy routing, Service/endpoints
and IAM workload. If that needs a new lease producer/transfer, tunnel or deployer,
an architecte decision and scope arbitration are required. Do **not** replace
this gate with `return`, a boolean approval or a self-attested manifest. HTTP and
the live selftest stay INCONCLUSIVE/nonzero until that integration is reviewed.
Tools have been observed; Chromium qualification remains separate from HTTP and
is still unproven.

The ratified addendum at `3f2e8fbe625b47fbc4093e673dc2804bcf9a3023`
also identifies a **source-level public-route incompatibility**: Envoy's `/iam/`
route retains fail-closed ext_authz, whose no-Bearer path denies access; the
local-full renderer does not add a health exemption. This is the lead's reviewed
static finding, **not an observed live403**. No default call to18080 is
enabled. Activation requires an existing authorized public exposure and a ratified
architecte amendment. Do not add a token, exemption, tunnel, free URL or403 oracle
in place of the unchanged200/OK contract.

## Pins and explicit acquisition

Nominal platform: **Ubuntu24.04, Linux x86_64, non-root**, including WSL only after
the parent's lifecycle/preflight/lease. Another platform is not silently accepted.
The CI label `ubuntu-24.04` is not an immutable runner image: the result records
the observed kernel/architecture and runner image version if provided.

`config/tools.lock.json` fixes:

- k6 `2.3.0`, official GitHub release archive, official checksum text and SHA256
  of both archive and extracted ELF. The archive hash matches the ratified contract.
- **Google Chrome for Testing `156.0.8078.73`**, Linux64, Chromium revision1706539.
  This is the pinned Chromium-engine distribution, not a claim
  that a preinstalled system browser or historical Chrome149 was verified.
  The exact official Google archive was acquired over authenticated HTTPS on
  2026-10-29 and its SHA256 and ELF SHA256 calculated without execution.
  Google publishes versioned downloads but no SHA256 in that metadata: these
  initial candidate hashes were reviewed before the controlled runs cited above.
  Rehashing each download is not a new trust pin. The executable versions and
  matching ELF hashes were observed in those runs, without qualifying the browser.

The lock has acquisition URLs and checksum provenance; published run projections
only contain versions/hashes, not arbitrary URLs. There are no remote JS imports,
npm packages, xk6 extensions, Cloud integrations or implicit downloads/installs.
Python3 standard library is sufficient for tooling. The existing target authority
additionally requires its existing PyYAML prerequisite; supply it explicitly before
HTTP integration, never install it automatically inside a run.

After review, explicit provisioning (proposed commands, **not evidence of a run**):

```sh
umask 077
# Choose a NEW absolute private Linux directory outside any Git checkout.
PRIVATE_ROOT=/home/REPLACE/.cache/opencode/k6-REPLACE
mkdir -m 700 "$PRIVATE_ROOT"
export K6_TOOLS_DIR="$PRIVATE_ROOT/tools"
export K6_BROWSER_EXECUTABLE_PATH="$K6_TOOLS_DIR/chrome-linux64/chrome"
python3 -B ops/tests/k6/provision-linux.py --output "$K6_TOOLS_DIR"
```

Provisioning is a separate explicit network operation, never invoked by the
launcher. It checks archives before safe extraction (no links, path traversal or
overwrites), creates owner-only files/directories, and never runs downloaded code.
The launcher rechecks archive and ELF bytes before **even** `version`/`--version`;
it refuses PATH fallbacks or browser wrappers. Browser shared libraries must be
present on the approved Ubuntu runner; no automatic apt install or security
policy relaxation is attempted. Failure remains nonzero.

Path confidentiality and name stability are separate guards. Every ancestor must
belong to root or the effective UID, without symlinks; ordinary system0755
directories are accepted. Group/other-writable ancestors require sticky protection
and trusted ownership of each child entry, so root-owned `/tmp`1777 is supported
but a non-sticky shared0777 ancestor or foreign-UID owner is refused. The leaf and
its immediate container still meet their stronger private-file/directory rules.
This deliberately conservative policy does not trust another UID's directory
merely because its current mode is0755.

Tool file/ancestor identities are bound before hashing, checked again after it,
and revalidated before versions, inspect, scenario launches and provenance.
Replacement inode/owner/mode or changed file metadata fails before child execution.
Directory timestamps are not compared: unrelated `/tmp` entries may change.
The output parent's identity is also checked across preflight before exclusive
creation. The local threat model trusts root and the operator UID; this is not
protection against hostile root or a generally compromised operator account.

## Commands and private campaign inputs

The launcher is the same in local use and CI. `tools` contains the guarded
equivalents of `k6 version`, `"$K6_BROWSER_EXECUTABLE_PATH" --version` and both
`k6 inspect` commands. Do not execute unverified binaries directly first.

```sh
python3 -B -m unittest discover -s ops/tests/k6/tests -p 'test_*.py'
python3 -B ops/tests/k6/run-linux.py tools --output "$PRIVATE_ROOT/tools-check"
python3 -B ops/tests/k6/run-linux.py browser --output "$PRIVATE_ROOT/browser-check"
```

Each `--output` must be absolute, outside Git, absent, with an existing owner-only
canonical parent directory. No symlinks, reuse, output overwrite or automatic
retries. The script emits only a small closed-label JSON result, with nonzero exit
for FAIL or INCONCLUSIVE. `cleanup` concerns **launcher children only**, not closure
of the parent's cluster/runtime. HTTP and Chromium verdicts are never conflated.

### Temporary Ubuntu22.04 comparison — not qualification

The comparison addendum ratified in `b316668` admits **only** a temporary native
Ubuntu22.04 amd64 non-root diagnostic. Ubuntu24.04 remains the nominal platform;
all nominal commands still refuse22.04. This does not authorize the separately
Proposed AppArmor exception, a security-policy change or a durable downgrade.

After explicit acquisition into the same kind of fresh private root, the proposed
command is (not a record of execution):

```sh
python3 -B ops/tests/k6/run-linux.py browser --compare-ubuntu-2204 --output "$PRIVATE_ROOT/ubuntu2204-comparison"
```

This flag is accepted only on `browser` or its guarded `diagnose-chromium`
followup, and only on22.04. The browser comparison reuses the pinned
archive/ELF checks, both actual binary version checks, both scenario inspections,
then **at most one** unchanged k6/browser scenario and `Child` supervision. Failed
prerequisites stop before that scenario. No automatic direct run, retry, application
target, free OS/arguments/executable, package installation or fallback is added.

The one-line JSON separates tools, attempted browser launch, complete case,
sandbox witness, child cleanup, parent cleanup and confidentiality. Its bounded
metadata records the admitted OS label, observed architecture/kernel/runner image,
UID, versions and ELF hashes. Even a complete case with the required witness and
cleanup returns **exit2**, HTTP/Chromium **INCONCLUSIVE**, and **never calls
`publish`**. Failures remain nonzero, with privacy and cleanup failures intact;
no result/provenance file is published on this path. Raw streams and summaries
stay private. These simulated regression fixtures do not qualify a real browser.

The dedicated workflow preserves the canonical24.04 job, including its failure,
and adds only a nonpublishing22.04 comparison job with the same checkout SHA,
read-only permission, acquisition and private-root rules. A push may incidentally
run24.04 again; the comparison reference remains run40973768642, not a claimed
single-variable A/B. No dispatch, failure masking or artifact upload is added.

After the observed22 refusal (run40976916763), the lead selected the already
Accepted optional **separate** direct22 diagnostic. Only a current incomplete
`SANDBOX_UNPROVEN` refusal with tools PASS, full checked streams/summary and child
cleanup PASS creates `private/comparison-result.json` (bounded, exclusive,0400).
This is the existing closed result used privately by the parent, not a published
artifact, runtime-ownership assertion or replacement target lease.

Proposed followup after that admissible refusal, in the **same private campaign
root/job/host/pins**, never a command already executed by this documentation:

```sh
python3 -B ops/tests/k6/run-linux.py diagnose-chromium --compare-ubuntu-2204 --browser-result "$PRIVATE_ROOT/ubuntu2204-comparison/private/comparison-result.json" --output "$PRIVATE_ROOT/ubuntu2204-direct-check"
```

Before any version/browser child, the direct path checks the private file with
`private_read`/`strict_json`, exact closed fields, incomplete counters, same host
metadata/UID/ELF hashes/versions, and timestamps (duration and age each at most120s).
The output must be a fresh sibling campaign directory. An exclusive private
`direct-attempted.json` marker consumes the permission once, even if later checks
fail; no retry/removal is provided. Missing, truncated, stale, hostile, complete,
privacy-failed or cleanup-failed results do not admit a direct child. All normal
direct sandbox/privacy/identity/cleanup checks remain unchanged; native exit0 is
still INCONCLUSIVE/nonzero/nonpublished. CI retains the initial job failure.

The additive `k6_exit` label uses the exact
[k6v2.3.0 exit-code table](https://raw.githubusercontent.com/grafana/k6/v2.3.0/errext/exitcodes/codes.go),
not generic NONZERO: notably99=`THRESHOLDS_HAVE_FAILED`,104=`INVALID_CONFIG`,
107=`SCRIPT_EXCEPTION`,108=`SCRIPT_ABORTED`. Absence, zero, signal and unknown
codes have distinct closed labels; no arbitrary number/text is projected. This
table is **never** applied to Chromium's native exit. The22 failure despite the
observed AppArmor restriction0 neither establishes a cause nor qualifies24.04;
the differing setup-image and `ImageVersion` observations in report27 remain
distinct. No host policy, browser arguments or pinned binaries are changed.

Research is reused from mission report23 and the Accepted comparison addendum:
[pinned Chromium156 sandbox guidance](https://chromium.googlesource.com/chromium/src/+/156.0.8078.73/docs/security/apparmor-userns-restrictions.md),
[official Ubuntu22.04 runner retirement announcement](https://github.com/actions/runner-images/issues/14254).
Ubuntu22.04 is deprecated and retires2027-04-17. A different result can demonstrate
environment dependence, **not AppArmor causality or Ubuntu24.04 qualification**.

`config/campaign.example.json` is documentation, **not** a usable lease. Copy and
populate it in an existing private Linux directory outside Git, then make the
new owned config read-only (`0400`); parent directories must be `0700`. Never
chmod/adopt a foreign file or copy a Windows lease. The schema is closed:

- `version:1`, `profile:local-full`, `cluster:aiforall-local-full`,
  `context:kind-aiforall-local-full` are fixed.
- `task_id`: current parent's32-hex task identity, matching the contemporary lease.
- `target_id`: opaque32-hex correlation ID, not an authority assertion.
- `origin`: exactly `http://127.0.0.1:18080`, no path/credentials/query/fragment.
- `lease`: absolute canonical private existing POSIX leasev2 path.
- `observations`: absolute canonical private reference to the parent's current
  observations. **It is not consumed as authorization by this candidate.**

Duplicate/unknown JSON fields, nonfinite JSON and missing fields fail closed.
The launcher fixes the method/route; it accepts no arbitrary URL, script or k6 args.
Proposed parent commands (currently HTTP integration is blocked as above):

```sh
export K6_CONFIG=/home/REPLACE/.cache/opencode/private-campaign/campaign.json
export K6_RUN_DIR="$PRIVATE_ROOT/http-run"
export K6_BROWSER_RUN_DIR="$PRIVATE_ROOT/browser-run"
export K6_SELFTEST_RUN_DIR="$PRIVATE_ROOT/selftest-run"
python3 -B ops/tests/k6/run-linux.py validate --config "$K6_CONFIG"
python3 -B ops/tests/k6/run-linux.py run --config "$K6_CONFIG" --output "$K6_RUN_DIR"
python3 -B ops/tests/k6/run-linux.py browser --output "$K6_BROWSER_RUN_DIR"
python3 -B ops/tests/k6/run-linux.py selftest --config "$K6_CONFIG" --output "$K6_SELFTEST_RUN_DIR"
```

`validate` is **offline syntax/schema/path/coherence only**, without importing the
live authority, tools, Docker, kubectl, HTTP or installation. `inspect` does not
execute iterations. Neither establishes a live target. `run`/`selftest` call the
target gate before executing any k6 binary; HTTP phase children have fresh pre/post
checks, including on native failure/interruption. There is no nominal pre-probe.

## Verdict, confidentiality and ownership

The HTTP scenario explicitly declares `per-vu-iterations`, one VU, one iteration,
15s max duration,5s request timeout and `redirects:0`. A redirection fails instead
of issuing a second request. Checks and case-counter thresholds are functional,
not performance SLOs. PASS additionally requires k6 exit0, strict current-run
summary, exactly one expected/started/completed case, one iteration/request,
both exact check IDs passing once with no failures, post-guard and child cleanup.
Missing, stale, truncated or zero-case evidence never becomes PASS.

`selftest` is a separate aggregate, never HTTP PASS. Its fixed private child
drivers must return actual nonzero exits for missing input, foreign target refusal
before a k6 child, absent case, deliberately false check, and interruption/missing
summary. The false-check oracle requires evidence that the real endpoint returned
200/OK while the deliberate body check failed. There is no HTTP mock server.
Without the real target, the aggregate fails INCONCLUSIVE rather than pretending
that local parser tests exercised live negatives.

`browser` creates a new context/page, visits only `about:blank`, evaluates6*7,
and closes in finally. The parent observes actual owned renderer processes via
`/proc`: real UID/start identity, seccomp mode2 with additional filter over the
parent, NoNewPrivs, and distinct user/PID namespaces. A bounded projection of
these witnesses accompanies a successful result. A blank page alone is not
sandbox proof. Forbidden sandbox/TLS flags fail; no AppArmor/userns/seccomp change,
root execution, UI, HAR, screenshots, CDP debug or application navigation is added.
If Ubuntu/WSL/CI cannot demonstrate this sandbox, Chromium remains unqualified.

On a browser refusal the one-line failure result may additionally contain
`browser_native_diagnostic`, a **closed message-category label** derived only from
the already bounded/private stdout/stderr. Known pinned Chromium messages can
report no usable sandbox, namespace permission denial, rejected/missing SUID
helper or root startup refusal. These are reported text categories, not proof
that a particular AppArmor/sysctl policy caused the failure. Unknown messages
remain `NATIVE_CAUSE_UNCLASSIFIED`; conflicting categories are not resolved by
guessing. `K6_ZERO_EXIT_NO_SANDBOX_WITNESS` means only that k6 exited0 while the
required external witness was absent, never Chromium PASS.

Sentinels, invalid UTF-8 or oversized inputs withhold the category. No matched
text, path, environment, URL or native suffix is projected. Existing privacy
checks remain unchanged: even official documentation URLs in Chromium errors
can still cause PRIVACY_FAILED; the optional closed category does not waive that
failure, publish any artefact or make a run successful.

Research for these categories used the pinned
[Chromium156.0.8078.73 zygote startup source](https://chromium.googlesource.com/chromium/src/+/156.0.8078.73/content/browser/zygote_host/zygote_host_impl_linux.cc),
[SUID helper checks](https://chromium.googlesource.com/chromium/src/+/156.0.8078.73/sandbox/linux/suid/client/setuid_sandbox_host.cc),
[namespace error message](https://chromium.googlesource.com/chromium/src/+/156.0.8078.73/sandbox/linux/suid/sandbox.c),
and [official Ubuntu specification](https://discourse.ubuntu.com/t/spec-unprivileged-user-namespace-restrictions-via-apparmor-in-ubuntu-23-10/37626).
The Ubuntu mechanism is context, not evidence of the hosted runner's active policy.
Importantly, [k6v2.3.0 browser_process.go](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/common/browser_process.go#L206-L278)
retains `:ERROR:` lines in its startup parser, not every Chromium `FATAL` line;
it can return only a generic startup-ended error. A discarded message cannot be
recovered by this diagnostic. No debug, new capture, policy change, wrapper,
installation or sandbox bypass is introduced; the precise CI cause may remain
unproven under these bounds.

### Explicit parent Chromium diagnostic (Accepted addendum 2026-10-30)

This mode is **not** a second qualification runner or a fallback in `browser`.
It addresses the possibility that k6's parser discarded a native startup error;
it does not assume that AppArmor/userns caused any observed k6 failure.
Proposed parent-only command, **not executed by the author**:

```sh
python3 -B ops/tests/k6/run-linux.py diagnose-chromium --output "$PRIVATE_ROOT/chromium-direct-check"
```

There is no config, URL, executable or extra-argument option. The same archive/ELF
pins, path identities, non-root Linux platform, version probes, closed environment
and `Child` supervisor are reused. Each invocation creates one new private profile
and launches at most one direct browser after the version probes. Its fixed argv:

```text
--headless --dump-dom --timeout=5000
--disable-background-networking --disable-component-update
--no-first-run --no-default-browser-check --disable-sync --metrics-recording-only
--user-data-dir=<new private profile> about:blank
```

`--dump-dom` supplies a small, noninteractive exit mechanism for the blank page,
not a functional assertion or an exported DOM artifact. The supervisor imposes
its own15s deadline. No debug/CDP endpoint/port, wrapper, sandbox/TLS weakening,
package, policy change or application navigation is added. A Chromium stdout/stderr
stream is private and bounded64KiB; no stream or native failure file is uploaded.

The closed `direct_observation` version1 identifies `source:CHROMIUM_DIRECT`, native
exit category (`ABSENT`, `ZERO`, `NONZERO`, `SIGNAL`), lexical native category and
termination (`EXITED`, `NOT_OBSERVED`, `CHILD_TIMEOUT`, `INTERRUPTED`). A signal can
come from the supervisor's graceful cleanup; it is not automatically a crash.
The k6 zero-exit label/table is not used for a native Chromium exit. Native text,
paths, URLs and suffixes are never projected. A known lexical label can survive
a privacy refusal (e.g. a FATAL containing a documentation URL); the refusal stays
`PRIVACY_FAILED`/nonzero, without publication. Sentinel, invalid UTF-8 and over-limit
streams cannot produce such a category. Unknown remains unknown.

The additional **direct-only** `direct_stop` version1 is a separate closed object
with exactly `version`, `path`, `signal`; it does not extend `direct_observation`
or the exact private k6 refusal schema. `Child` records its path at the event,
before cleanup: `NATIVE_OBSERVED_BEFORE_CLEANUP`, `GUARD_REJECTED`, `TIMEOUT`,
`INTERRUPTED`, `OUTPUT_LIMIT`, `ERROR`, or `NOT_OBSERVED`. Later privacy/cleanup
priority never rewrites this internal fact. In particular, a native status
recorded before cleanup cannot be attributed to a TERM sent later by that cleanup.
This explains the matching direct22 tuple in run40979730197; it does not determine
that run's exact signal or sender retrospectively.

`signal` is a canonical name from the fixed Linux x86 signals1–31 whitelist,
or `ABSENT`, `NO_SIGNAL`, `UNKNOWN_SIGNAL`; raw numbers, aliases, realtime-signal
numbers and host/Windows enum interpretations are not exported. Source:
[Linuxv6.8 x86 UAPI signal.h](https://kernel.googlesource.com/pub/scm/linux/kernel/git/torvalds/linux/+/refs/tags/v6.8/arch/x86/include/uapi/asm/signal.h).
The status provides **no sender or cause**. The whole facet is withheld on privacy
or overflow failure, and malformed facets are withheld without changing the old
diagnostic or cleanup verdict. Its absence is not evidence of a native event's
absence. No additional process read, signal, deadline, probe or permission is used;
the control path, k6 receipt/gate and CI stay unchanged. A native zero or a useful
stop facet remains nonqualifying/nonzero/nonpublished.

The separate **direct-only** `direct_text` version1 contains only stdout/stderr
presence (`EMPTY`/`PRESENT`) and one family: `CHROMIUM_FATAL_REPORTED`,
`ASSERTION_FAILURE_REPORTED`, `OTHER_TEXT`, `NO_TEXT`, or `MULTIPLE`. Presence
describes captured bytes, not an assertion about uncaptured output. Whitespace is
present text; only two empty buffers yield NO_TEXT. MULTIPLE means distinct
recognized families, not duplicate messages or unrecognized noise.

Recognition starts only after both entire captured buffers pass the unchanged
privacy/UTF-8/size checks. It uses full LF-terminated lines independently in each
stream: Chromium's numeric log items plus `FATAL:file:line] body`, or glibc's
file/line/optional program/function plus the exact assertion framing. Source:
[Chromium156 logging.cc](https://chromium.googlesource.com/chromium/src/+/156.0.8078.73/base/logging.cc),
[glibc2.35 assert/assert.c](https://raw.githubusercontent.com/bminor/glibc/glibc-2.35/assert/assert.c).
Arbitrary prefixes, incomplete lines, split-stream fragments, isolated words and
ERROR-only messages stay OTHER_TEXT. These deliberately narrow formats do not
prove the emitting library version or that a reported message caused the signal.

No text, site, assertion, line, PID, path, suffix or hash is exported. Privacy,
invalid UTF-8 or overflow withholds the entire facet. The old facets, Child,
cleanup/error priority, private k6 receipt/gate and CI remain unchanged. Native0
and useful classifications remain nonqualifying/nonzero/nonpublished. This code
authorizes no new run or recognizer series: OTHER_TEXT/NO_TEXT, or an insufficient
FATAL family, leaves the next step to the separate causal-anchor arbitration;
no anchor implementation or additional collection is included here.

Stream overflow is a sticky fact recorded by `Child` at the read boundary before
truncation, independently of its final error code. Even if cleanup failure takes
priority, the direct path neither classifies nor projects that captured prefix,
and confidentiality cannot become PASS from truncated evidence. The cleanup
failure, privacy checks, byte limits and graceful-stop rules remain unchanged.

**Every diagnostic invocation exits nonzero. HTTP and Chromium stay INCONCLUSIVE,
even for native exit0 and an observed sandbox witness.** Native/sandbox, privacy,
timeout, interruption and cleanup failures retain their diagnostic code. No
result/provenance or summary file is published. The one-line closed JSON reports
cleanup/confidentiality separately. A reaped handle without any contemporaneous
PID/UID/start observation cannot attest identity-based cleanup.

Both `browser` (`browser_source:K6`) and this diagnostic
(`browser_source:CHROMIUM_DIRECT`) project the same optional `browser_host` fields.
Only the following exact files are opened read-only, once each, bounded to64bytes
for the first four and16KiB for the fifth:

- `/proc/sys/kernel/unprivileged_userns_clone`
- `/proc/sys/kernel/apparmor_restrict_unprivileged_userns`
- `/proc/sys/user/max_user_namespaces`
- `/sys/module/apparmor/parameters/enabled`
- `/proc/self/status` (only Seccomp and NoNewPrivs projected)

Values are closed labels: `ZERO`/`ONE` (plus `POSITIVE` for the namespace limit),
`ENABLED`/`DISABLED`, or seccomp `ZERO`/`STRICT`/`FILTER`. `ABSENT`, `DENIED`,
`UNREADABLE` are distinct from zero. Invalid/duplicate/oversized values are not
copied. There is no additional process scan, command, profile or host-log read.
Configured restrictions are observations, not proof of causation.

The dedicated CI has an **explicit separate step**, only after a failed browser
step with successful tool qualification. It runs once in that same job/UID,
with unchanged pins, environment and private root; both fresh run directories
are siblings of the private `tools-check` provenance. That root and the parent's
process journals provide private correlation; this is not a comparison across
hosts/pins. Argv differs deliberately: k6/browser has its own defaults, internal
transport and context; direct uses only the fixed flags above and a separate
profile. No equivalence of launch behavior is claimed. The original browser
failure is preserved, the diagnostic also exits nonzero, and the success-only
allowlisted artifact upload remains skipped. No retry/dispatch/continue-on-error.

Argument research before implementation (2026-10-30): the
[official Headless guide](https://developer.chrome.com/docs/chromium/headless)
was recouped with pinned156.0.8078.73
[command handling](https://raw.githubusercontent.com/chromium/chromium/156.0.8078.73/components/headless/command_handler/headless_command_handler.cc),
[headless mode](https://raw.githubusercontent.com/chromium/chromium/156.0.8078.73/chrome/browser/headless/headless_mode_util.cc),
[Chrome switch definitions](https://raw.githubusercontent.com/chromium/chromium/156.0.8078.73/chrome/common/chrome_switches.h),
[metrics switches](https://raw.githubusercontent.com/chromium/chromium/156.0.8078.73/components/metrics/metrics_switches.h)
and [sync flag handling](https://raw.githubusercontent.com/chromium/chromium/156.0.8078.73/components/sync/base/command_line_switches.cc).
The old `.cc` location for Chrome switch definitions returned404; the versioned
`.h` contains the inline definitions. Remote debugging, screenshots/PDF, arbitrary
targets, unbounded headless waiting and host-security workarounds from generic
examples were not adopted. Existing pin/source audits and70 historical Linux
tests are not new evidence for this delta.

### Final bounded observation (failure only)

The existing one-line browser failure result may also contain `browser_observation`.
It is validated by an exact key/type/label allowlist both when constructed and
when received from an exception. Malformed projections are withheld. It contains:

- `native_exit`: `ABSENT`, `ZERO`, `NONZERO` or `SIGNAL`, from the supervised k6
  child's observed returncode, not Chromium's exit status. The last value already
  observed by cleanup's poll is retained on interruption; `SIGNAL` may therefore
  reflect the supervisor's graceful cleanup, not a browser crash diagnosis.
- `summary`: `ABSENT`, `INVALID` or `VALID`. `VALID` means bounded/private JSON
  with the current opaque run ID, Chromium phase/case, closed schema and valid
  types, **not a successful functional summary**. It permits partial case counters
  and incomplete/failed checks for diagnosis only. The strict PASS oracle is
  unchanged. Stale/wrong-case/malformed/oversized summaries project no counts.
- `counters`: exactly `expected`, `started`, `completed`, `iterations`, `requests`,
  each classified as `ABSENT`, `ZERO`, `ONE` or `OTHER`. Missing/null counters stay
  `ABSENT`; no values, raw metrics or run-ID content are copied into this object.
- `renderer_observed`: a boolean recording a renderer actually read with matching
  owned PID/UID/start identity. False means **not observed**, never proven absent.
- `namespace_reads`: `user` and `pid` each contain a bounded sorted set of labels
  `READABLE`, `DENIED`, `DISAPPEARED`, `OTHER`, or solely `NOT_OBSERVED`.
  These aggregate read outcomes across already-observed owned renderers/times;
  they do not identify one renderer or combine readings into a sandbox witness.
  `DISAPPEARED` classifies a missing link at that read, not process absence.

Only the supervisor's existing inventory/identity reads are consumed. Its optional
read order and short-circuit after error are preserved: if user-namespace reading
fails, the PID-namespace read stays `NOT_OBSERVED`, rather than adding a retry or
foreign-process read. Optional errors retain base identities for cleanup. No new
OS command, scan, privileged access, artifact or proof method is introduced.
The existing per-renderer sandbox witness and its safety ordering remain unchanged.
All native/sandbox/interruption/privacy/cleanup failures remain non-PASS; summary
privacy failures also prohibit publication, even if the original error was about
sandbox evidence. No failure artifact is written.

Source checks for this delta: [k6v2.3.0 browser options, lines84–100](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/common/browser_options.go#L84-L100)
reads the supplied environment lookup for the executable path; filtering JS
`__ENV` is not a reason to add `--env K6_BROWSER_EXECUTABLE_PATH` as a fix.
[Chromium156.0.8078.73 dumpable handling, lines67–80](https://github.com/chromium/chromium/blob/156.0.8078.73/sandbox/policy/linux/sandbox_debug_handling_linux.cc#L67-L80)
shows non-dumpable setup, a possible observability constraint, not the demonstrated
CI failure cause. [Python3.13 subprocess returncode](https://github.com/python/cpython/blob/v3.13.0/Doc/library/subprocess.rst#L936-L946)
documents absent returncodes and negative POSIX signal exits. These pinned sources
and the existing k6 startup parser above were read before the diagnostic choices;
no binary, environment/pin change or runtime experiment was used by the author.

Child environments are constructed from scratch (no ambient K6, proxy, loader,
Cloud, secrets or user config), with reporting explicitly disabled. Runtime API
listen address is empty, user config is an explicit empty file, and system env
injection into JS is off. Only opaque run ID, fixed origin, summary path and the
private selftest mode enter JS. URL tags are disabled and request labels are stable.

Stdout/stderr are privately bounded to64KiB each, never streamed/published.
Overflow stops owned processes and fails. Sentinels and conservative secret/URL
shape checks cover both streams and every retained public artifact on success and
negative paths. Native exceptions/rejected config values are never interpolated
into diagnostics. This is bounded evidence, not a universal secret-redaction claim.
Any failed confidentiality check prevents publication and PASS.

Only `result.json` and `provenance.json` are publishable, maximum32KiB each and
256KiB total. All projected bytes are checked before any publication. The private
subdirectory (summary, process identity records and selftest data), archives and
captured streams are **not** artifacts; never upload the whole run directory.
There are no raw metrics or environment/command-line dumps in artifacts.

The supervisor is a Linux subreaper with a dedicated child session and a baseline
inventory. It tracks PID+UID+start time/descendance, sends only graceful TERM to
observed owned identities, joins and checks survivors, including orphaned children.
No process-name ownership, forced kill, Docker mutation, deletion, prune or reset.
Unstoppable survivors are a cleanup failure with private IDs for the parent.
The parent retains responsibility for runtime leases, contemporary Docker registry
and before/after inventories, including early failures. No Docker resources are
created by this tooling; do not invent registry entries. Never stop unrelated
WSL/Docker/Kind resources to make this suite appear clean.

## Minimal CI, sources and validation record

`.github/workflows/k6-functional-smoke.yml` runs **prerequisites only** on
`ubuntu-24.04`: local tests, explicit pinned acquisition, guarded versions/inspect,
then real sandboxed Chromium. Checkoutv4.2.2 and upload-artifactv4.6.2 are pinned
to full commit SHAs, checked through the official GitHub refs API on2026-10-29.
Permissions are only `contents:read`, no persisted checkout credentials, application
secrets/target, deployer, tunnel, dispatch, Cloud or `continue-on-error`.
Only four explicit projection files can upload after success. A green prerequisite
workflow is **not** an HTTP/E2E/mission1 PASS. No HTTP job/skip invents such proof.

Versioned official sources checked before implementation:

- [k6v2.3 per-VU executor](https://grafana.com/docs/k6/v2.3.x/using-k6/scenarios/executors/per-vu-iterations/index.md),
  [thresholds](https://grafana.com/docs/k6/v2.3.x/using-k6/thresholds/index.md),
  [custom summary](https://grafana.com/docs/k6/v2.3.x/results-output/end-of-test/custom-summary/index.md):
  retain explicit functional cardinality and a small legacy-schema projection,
  explicitly fix `new-machine-readable-summary=false`. Checks alone are insufficient.
- [v2.3.0 runtime flags](https://github.com/grafana/k6/blob/v2.3.0/internal/cmd/runtime_options.go),
  [root flags](https://github.com/grafana/k6/blob/v2.3.0/internal/cmd/root.go),
  [inspect](https://github.com/grafana/k6/blob/v2.3.0/internal/cmd/inspect.go):
  fixed config/environment, disabled runtime API, no assumed global reporting flag
  on inspect. Browser and summary details were cross-checked against the tag.
- [Browser optionsv2.3](https://grafana.com/docs/k6/v2.3.x/using-k6-browser/options/index.md)
  and [v2.3.0 Chromium launch](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/chromium/browser_type.go):
  retain native sandbox and isolate options. Reject illustrative `master-with-browser`,
  `no-sandbox` and certificate/proxy bypasses; official examples are not a security policy.
- [Official k6 checksum release](https://github.com/grafana/k6/releases/tag/v2.3.0),
  [official CfT metadata](https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json):
  metadata selects the exact public candidate, not a moving runtime dependency.
- The lead's documented review of Grafana's
  [Actions guide](https://grafana.com/blog/performance-testing-with-grafana-k6-and-github-actions/)
  and [suite organisation guide](https://grafana.com/blog/organizing-your-grafana-k6-performance-testing-suite-best-practices-to-get-started/)
  is reused: start small, local modules, explicit inputs. Reject floating action
  tags/default versions, Cloud/PR comments, load thresholds and error wrappers that
  publish response URLs/traces. No extra framework is justified for one GET.

Actual author checks and remaining gates are recorded in `IMPLEMENTATION.md`.
The parent's ledger is `.cursor/handoffs/20261029-k6-01-socle.md` (lead-only writes).
Historical17IT/Chrome149 evidence retains its original scope, never k6 qualification.
