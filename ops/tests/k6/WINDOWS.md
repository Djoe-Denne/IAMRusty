# Native Windows adapter — implementation candidate, not qualification

Contract: [Accepted Windows mission1](../../../docs/contracts/k6-windows-functional.md).
Windows PowerShell **5.1**, native AMD64, non-elevated. No Rust, service, identity,
OAuth, HTTP server, WSL, Kind, Docker, VM, package installer or CI dispatcher.
The three existing JavaScript files are copied byte-for-byte into each fresh run.
The Linux entrypoints, ChildPOSIX and private collection driver are not imported.

## Two explicit qualification boundaries

**No Chromium PASS or successful complete tools phase is currently possible.**
Pinned Chromium156 `ChromeMainDelegate::BasicStartupComplete` invokes
`HandleVersionSwitches` only under `BUILDFLAG(IS_POSIX)`. The pinned Windows
`chrome_exe_main_win.cc` does not provide an equivalent `--version` handler.
Consequently this adapter **does not execute `chrome.exe --version`**: assuming
that it exits before UI would violate the Windows contract. File VersionInfo,
archive provenance, a successful k6 case, or an agent opinion cannot replace an
executed version proof. After the admitted k6 version/two inspections, the explicit
`CHROMIUM_VERSION_UNAVAILABLE` gate returns nonzero/INCONCLUSIVE. Browser launch
also stays behind this gate. No fallback to Chrome149, headless-shell, a new JS
scenario, exposed CDP, unsafe flags or altered asset is silently adopted. The lead
must route a reviewed compatible executable-version strategy before enabling it.
The browser branch is deliberately unqualified and not operationally admitted by
the presence of this source.

**HTTP is closed.** `validate`, `run`, `selftest` check private input/JSON and fixed
origin, then return `TARGET_BINDING_UNQUALIFIED` without launching a tool or making
a GET. The only existing authority is the Linux anchor whose origin binding is
itself unqualified. No Windows lease producer, JSON approval boolean, URL or CLI
switch is a replacement authority. Live false-check/absent-case/interruption
selftests are **not implemented as executable HTTP paths** while that admission
is absent. Target integration must be resolved through the existing contract, not
by a fake endpoint, new deployer, tunnel, token or health exemption.

These are acceptance failures, not reasons to weaken guards. WSL and Linux remain
INCONCLUSIVE; Linux CI is separately suspended by the lead, not qualified here.

## Lock interface for the lead

The runner only reads `config/tools.windows.lock.json`; its absence is a refusal,
not an instruction to acquire/install anything. This implementation does not
create that file or supply fabricated production hashes.

Closed JSON schema (unknown/duplicate/case-alias members rejected):

- Top level: `version` integer1, `platform` string `windows-amd64`, `k6`, `chromium`.
- `k6`: `version` string `2.3.0`, `url` exact official v2.3.0 Windows-amd64 ZIP URL,
  `checksum_url` exact official v2.3.0 checksums URL, `archive` string
  `k6-v2.3.0-windows-amd64.zip`, `archive_sha256` fixed
  `112276d495e5741c968e2bc09ea6196099c1275bd6db9ee0875d173c7148ce43`,
  `executable` string `k6-v2.3.0-windows-amd64\k6.exe`, `executable_sha256`.
- `chromium`: `version` string `156.0.8078.73`, `revision` **string** `1706539`,
  `url` exact official versioned win64 ZIP URL, `metadata_url` exact versioned
 156.0.8078.73.json URL, `archive` string `chrome-win64.zip`, `archive_sha256`,
  `executable` string `chrome-win64\chrome.exe`, `executable_sha256`.
- All hashes are lowercase64 hexadecimal, nonzero. Archive paths are resolved
  from the root derived from the corresponding explicit executable path/suffix.
  The k6 and Chromium roots can differ. No free archive path, PATH lookup, URL,
  executable, command or argument override is accepted by the campaign CLI.

Both archive and executable are rehashed before every launch. DOS/PE signature,
AMD64 machine and PE32+ are checked before execution. File handles deny writing
and deletion; every ancestor is held without delete sharing. Reparse/UNC/ADS/
noncanonical names and unsafe ACL/owners are refused, not repaired. Private
directories require user/SYSTEM/Administrators-only data access. The ancestor
model also trusts TrustedInstaller and system administrators; it does not defend
against a compromised operator or administrator. It deliberately rejects shared
sandbox-writable temp ancestors on this host. Pick a genuinely private existing
parent; do not change ACLs on an existing drive/directory to make it pass.

## Parent/process boundaries

`windows-native.cs` is a local PowerShell Add-Type helper, not another executable
supervisor or a generic command CLI. It creates one unnamed empty Job Object,
assigns it atomically with `PROC_THREAD_ATTRIBUTE_JOB_LIST` at `CreateProcessW`,
and keeps the root suspended until its PID/creation/SID/image are observed. The
handle inheritance list contains just stdin/stdout/stderr. No breakaway flag,
KILL_ON_JOB_CLOSE, TerminateJobObject, TerminateProcess, taskkill, elevation or
debug privilege is used. Normal completion is preferred; timeout/interruption
requests CTRL_BREAK for **only the newly created console group**, then joins for
a bounded grace period. Lack of a shared console/cooperation can prevent cleanup;
it never authorizes force. Closing the job does not kill anything.

Job accounting covers the whole descendant tree even when a child exits between
observations. An unobservable short-lived child, inaccessible handle/token, missed
identity, nonempty job, or non-signalled retained handle prevents cleanup PASS.
No process is adopted by its name, PID alone, old journal or image. Private live
and ending identity journals are written around each operation; the final public
projection contains none of their SID/PID/path details. Partial private data is
retained if process cleanup cannot be proven, for parent recovery, not published
or force-deleted. Abrupt host termination is not a teardown guarantee.

The unqualified browser witness code reads only owned, image-matched processes.
It parses their actual Windows command lines, requires headless/new run-owned
profile/normal ephemeral internal k6 CDP (`--remote-debugging-port=0`), and refuses
external debugging addresses and sandbox/TLS bypass flags. It adds no debug port
or transport. A renderer must actually have a restricted primary token, untrusted
integrity SID, restricting NULL SID and no enabled privileges. That conservative
USER_LOCKDOWN witness is distinct from a Job Object or a missing command flag.
Unavailable/different policy remains INCONCLUSIVE. Chromium nesting, token access,
live timing and cleanup are **not proven by Python fixtures**.

## Results and private data

Each new absolute output path must not exist and must be outside Git. A fresh
run ID, short live lease, source hashes, pinned hashes and observed k6 version
are correlated in the Windows envelope. The JS summary schema is unchanged.
Strict cardinalities are1 expected/started/completed/iteration, two exact check
names each passes1/fails0, HTTP requests1 or browser requests0; absent/null,
boolean/string counts, stale run IDs, unknown/duplicate fields, failed exit,
truncated/absent summaries and vacant cases cannot pass.

The environment is built afresh, not inherited: no proxy, K6 output/cloud/debug,
loader, remote browser or user profile override. Usage reporting is disabled.
Raw stdout/stderr are bounded to64KiB each in private parent memory, decoded
strictly and checked for sentinels/credential patterns before any projection.
Overflow is sticky; missing EOF/invalid encoding is not a clean empty stream.
Native streams and exceptions are never emitted. Only fixed labels, run ID,
versions/hashes are publishable on stdout. A privacy failure withholds success;
cleanup remains a separate result. No raw-metric or arbitrary artifact exporter
is provided. Successful owned directory removal is bounded and refuses reparse
entries before traversal. No other output, user profile or foreign process is
removed.

## Proposed commands, not execution evidence

The parent supplies explicit approved Windows paths and a fresh private output
for each operation. First integrate/review the lock and resolve the two boundaries
above. The tool guards are not a permission to execute direct binary commands
without archive/executable verification. In particular, the agreed direct Chrome
`--version` command is **not safe to run based on POSIX semantics**.

```powershell
python.exe -B -m unittest discover -s ops/tests/k6/tests -p 'test_*windows*.py'
powershell.exe -NoProfile -NonInteractive -File ops/tests/k6/run-windows.ps1 -Mode tools -Output $env:K6_TOOLS_CHECK_DIR
powershell.exe -NoProfile -NonInteractive -File ops/tests/k6/run-windows.ps1 -Mode browser -Output $env:K6_BROWSER_RUN_DIR
powershell.exe -NoProfile -NonInteractive -File ops/tests/k6/run-windows.ps1 -Mode validate -Config $env:K6_CONFIG
powershell.exe -NoProfile -NonInteractive -File ops/tests/k6/run-windows.ps1 -Mode run -Config $env:K6_CONFIG -Output $env:K6_RUN_DIR
powershell.exe -NoProfile -NonInteractive -File ops/tests/k6/run-windows.ps1 -Mode selftest -Config $env:K6_CONFIG -Output $env:K6_SELFTEST_RUN_DIR
```

Focused tests use small known Python children only, never k6/Chrome/HTTP. The test
root is freshly created under the operator's Local directory with a private DACL;
existing ACLs are not mutated. Per-run private `.jsonl` fixture identity inventories
are retained alongside that root, while fixture data/directories are removed.
Each fixture has a bounded natural lifetime; even a cleanup-negative fixture is
joined by retained identity before its handles are disposed. Test scripts never
force-kill. The parent owns real campaigns, CI suspension, provenance acquisition,
integration, publication and final inventory/retirement.

## Official sources consulted before the choices

- [k6v2.3.0 browser_type.go](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/chromium/browser_type.go)
  (explicit executable and default ephemeral internal CDP, not no-sandbox).
- [k6v2.3.0 inspect.go](https://github.com/grafana/k6/blob/v2.3.0/internal/cmd/inspect.go)
  (unextended inspection emits runner options, no application request).
- [k6v2.3.0 thresholds.go](https://github.com/grafana/k6/blob/v2.3.0/metrics/thresholds.go)
  (`MarshalJSON` retains the simple string arrays for these non-aborting checks).
- [Chrome156 main delegate](https://github.com/chromium/chromium/blob/156.0.8078.73/chrome/app/chrome_main_delegate.cc#L1256)
  and [Windows executable main](https://github.com/chromium/chromium/blob/156.0.8078.73/chrome/app/chrome_exe_main_win.cc)
  (Windows `--version` cannot be assumed to be the POSIX early exit).
- [Chromium156 sandbox policy](https://github.com/chromium/chromium/blob/156.0.8078.73/sandbox/policy/win/sandbox_win.cc#L212)
  and [design](https://github.com/chromium/chromium/blob/156.0.8078.73/docs/design/sandbox.md)
  (USER_LOCKDOWN, delayed untrusted integrity, optional AppContainer, token evidence).
- [UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)
  (JOB_LIST/HANDLE_LIST), [process creation flags](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags),
  [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects),
  [GenerateConsoleCtrlEvent](https://learn.microsoft.com/en-us/windows/console/generateconsolectrlevent).
  Atomic membership retained; breakaway, force and job-on-close killing rejected.

The first attempted Microsoft URL under `win32/api/wincon` returned404; the official
`windows/console` page was then read. One guessed Chromium DLL main URL returned404;
the actual Windows exe source was read instead. Neither is executable evidence.
