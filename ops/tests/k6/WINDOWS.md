# Native Windows adapter — implementation candidate, not qualification

Contract: [Accepted Windows mission1](../../../docs/contracts/k6-windows-functional.md).
Windows PowerShell **5.1**, native AMD64, non-elevated. No Rust, service, identity,
OAuth, HTTP server, WSL, Kind, Docker, VM, package installer or CI dispatcher.
The three JavaScript files are copied byte-for-byte into each fresh run. The
Accepted [version addendum](../../../docs/contracts/k6-windows-browser-version.md)
admits only the additional conditional version check in the existing browser case;
the HTTP scenario and summary.js are unchanged.
The Linux entrypoints, ChildPOSIX and private collection driver are not imported.

## Version proof and remaining qualification boundaries

**Chrome `--version` remains NOT_EXECUTED / INCONCLUSIVE.** The source-level
IS_POSIX finding is retained; no non-UI Windows CLI behavior is assumed, tested
opportunistically or replaced by file VersionInfo. The Accepted addendum resolves
the circular gate through `browser.version()` in the **same existing browser case**.
k6v2.3.0 actually exposes this synchronous API. It reads the version cached from
`Browser.getVersion` at the normal managed-browser connection, not a new CDP query
for each call. There is no new launch, scenario, port, remote connection or logging.

The parent takes `K6_EXPECTED_BROWSER_VERSION` only from the reverified fixed lock,
never the ambient environment or a free argument, and requires exactly156.0.8078.73
before launch. The case adds `browser-version` inside its existing context
try/finally. The actual API value must be a string of at most32characters, four
decimal integer components, and exactly equal to that pin. Missing/throwing API,
empty/malformed/149/different value or changed expectation cannot succeed. No
returned value or exception is logged or added to the summary. Legacy Linux with
no expectation still has its two original checks; Windows never accepts omission.

`tools` alone still returns **tools=INCONCLUSIVE / exit2** with
`CHROMIUM_VERSION_PENDING` after guarded k6 version/two inspections. In `browser`,
the static hash/identity/environment guards release only the existing case.
Tools can become PASS only after its current-run summary proves exactly1pass/0fail
for `browser-version` plus the original checks/cardinalities and valid child
cleanup/confidentiality. The projected pinned version then means **equality
observed via k6's cached Browser.getVersion**, labelled
`K6_API_CACHED_GETVERSION_EQUALITY`, not metadata or an independent fresh CDP read.
Chromium currently stays **INCONCLUSIVE even when the sampled primary-token
boolean is true**: review R-2 established that it is not the required effective,
case-bound renderer witness. No new thread/token investigation is implemented.
Final run-directory
cleanup or confidentiality failure revokes both positive verdicts and the version
projection. Startup failure or stale/absent version check gives no runtime proof.
No actual k6/Chromium run is qualified by this implementation or its fixtures.

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

Review R-1 additionally requires the **whole extracted bundle**, not only the
main executable. The parent derives a closed file/directory set from the already
authenticated ZIP, refuses ambiguous/case-colliding/traversing/link/device entries,
missing or additional extracted members, and compares every regular file against
its authenticated entry bytes. No extraction or alternate public manifest is
introduced. Per-file private ACL/nonreparse/identity/read-only-sharing leases are
retained, together with directory name-chain leases, and checked at the actual
child callsite. Bounds are4096entries,512MiB per file and2GiB declared expansion;
the decompression hash also counts actual streamed bytes. Inert ZIP/DLL fixtures
exercise these paths without loading their content.

**Remaining retention boundary, not a closed R-1 claim:** on the normal proven
cleanup path, these leases outlive all concerned children. The unchanged bounded
Child/Job can instead return cleanup-unproven while a child survives; the current
PowerShell finally/host exit then releases file handles. It cannot claim retention
until that survivor's eventual exit. All verdicts remain nonpositive, private data
is preserved and the parent's live-run freeze must remain in place pending an
explicit compatible retention disposition/review. No background supervisor,
indefinite wait, force or existing ACL mutation was silently added to solve this.

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

Review S-1 replaces check-then-CreateDirectory with native `NtCreateFile`
`FILE_CREATE`/`FILE_DIRECTORY_FILE`, relative to a pinned private parent, supplying
the DACL and receiving the new directory's identity handle in the same operation.
An existing path refuses atomically, without changing its ACL. Cleanup authority
is issued only after successful binding; a failed binding preserves the object.
The retained no-delete-sharing handle prevents root substitution. Cleanup rejects
a disposed/unbound/replaced identity before traversal and deletes the empty root
through its original handle (`FileDispositionInfo`), never by releasing the handle
then deleting a reusable pathname. This is a local helper capability, not a new
campaign input or external file-authority service.

The unqualified browser witness code reads only owned, image-matched processes.
It parses their actual Windows command lines, requires headless/new run-owned
profile/normal ephemeral internal k6 CDP (`--remote-debugging-port=0`), and refuses
external debugging addresses and sandbox/TLS bypass flags. It adds no debug port
or transport. A renderer must actually have a restricted primary token, untrusted
integrity SID, restricting NULL SID and no enabled privileges. That conservative
primary-token sample is distinct from a Job Object or a missing command flag,
but does not prove post-bootstrap effective restrictions or the renderer that
executed the JS. It no longer authorizes Chromium PASS under any combination of
those booleans. The compatible effective-witness decision remains with the lead/
architecte. Chromium nesting, token access, live timing and cleanup are **not
proven by fixtures**.

## Results and private data

Each new absolute output path must not exist and must be outside Git. A fresh
run ID, short live lease, source hashes, pinned hashes and observed k6 version
are correlated in the Windows envelope. The JS summary schema is unchanged.
Strict cardinalities are1 expected/started/completed/iteration, two exact HTTP
checks or three mandatory Windows browser checks (including `browser-version`),
each passes1/fails0, HTTP requests1 or browser requests0; absent/null,
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
for each operation. First integrate/review the lock and all required guard fixes.
The tool guards are not a permission to execute direct binary commands
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

Focused tests use small known Python children plus one local existing Node.js
fixture for the real scenario body, never k6/Chrome/HTTP. That fixture adapts only
the ESM bindings and runs thirteen known input cases with local collaborators;
it checks required checks, no logging, unchanged about:blank/42/750ms and closures.
It is not a new application harness or browser/version proof. The test
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
- [k6v2.3.0 browser mapping](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/browser/browser_mapping.go#L73),
  [module exposure](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/browser/module.go#L98),
  [Browser.Version/cache](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/common/browser.go#L787)
  (managed JS API, stripped product prefix and cached normal-connection version;
  no Playwright assumption, VersionInfo, page UA or separate probe).
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
- [NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile),
  [SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle),
  [FILE_DISPOSITION_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_disposition_info)
  (atomic FILE_CREATE and identity-bound empty-directory retirement).
- [.NET Framework ZipArchive](https://learn.microsoft.com/en-us/dotnet/api/system.io.compression.ziparchive?view=netframework-4.8.1),
  [entry attributes](https://learn.microsoft.com/en-us/dotnet/api/system.io.compression.ziparchiveentry.externalattributes?view=netframework-4.8.1)
  (bounded, read-only authentication of the existing extracted bundle).

The first attempted Microsoft URL under `win32/api/wincon` returned404; the official
`windows/console` page was then read. One guessed Chromium DLL main URL returned404;
the actual Windows exe source was read instead. Neither is executable evidence.
