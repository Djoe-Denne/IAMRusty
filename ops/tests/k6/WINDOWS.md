# Native Windows adapter — minimal browser verified

## Verified state — 2026-10-31

The lead executed the native browser entrypoint successfully: exit0, tools PASS,
Chromium functional PASS, cleanup PASS and confidentiality PASS. k6 reported2.3.0;
the mandatory same-case API check established Chrome156.0.8078.73 equality. The
single about:blank case validated42 and all three named checks. The fresh run
directory was removed; independent k6/Chrome inventories were0 before and after.
This is not a performance measurement, sandbox attestation or Linux qualification.

Final integrated verification: **34 Windows tests, exit0, zero skips,42.651s**;
11fixture identities observed ended and both fresh fixture directories absent.
The real run exposed and fixed the `k6.exe` version prefix and the missing `group`
system tag. Only the fixed Windows browser command adds that standard tag; shared
JavaScript, HTTP and summary.js remain unchanged. k6v2.3.0
[GroupSummary](https://github.com/grafana/k6/blob/v2.3.0/lib/test_state.go#L137-L143)
otherwise omits individual checks from the legacy summary.

HTTP/selftests remain INCONCLUSIVE without an admitted existing target. WSL remains
suspended and Linux unqualified. Sources/publication and old worktree disposition
are tracked separately from the functional result in the mission reports.

Contract: [Accepted Windows mission1, revision3 amendment](../../../docs/contracts/k6-windows-functional.md#amendement-révision3--simplification-windows).
The amendment supersedes the old ACL, executable/DLL locks and renderer-attestation
requirements. The code-custody proposal is abandoned, not a prerequisite.

Windows PowerShell **5.1**, native AMD64, non-elevated. No Rust/service/identity,
OAuth, HTTP server, WSL, Kind, Docker, VM, installer or CI dispatcher. The existing
three JavaScript sources are copied byte-for-byte into a fresh run. Linux entrypoints,
ChildPOSIX and the private collection driver are not imported as Windows evidence.

## Tools and version evidence

`config/tools.windows.lock.json` is only a version/provenance/SHA256 **manifest**,
not a system lock. The runner neither downloads nor installs tools. Its closed
schema, official URLs, k6 **2.3.0**, CfT **156.0.8078.73**, revision **1706539** and
Windows-amd64 asset names remain unchanged. Each archive is resolved beside the
versioned extracted directory from its explicit executable suffix. Both roots may
differ. No PATH discovery or Chrome149 fallback.

Ordinary SHA256 reads verify each archive/executable before use; DOS/PE, AMD64 and
PE32+ are checked before admitting the executable. The handles close immediately.
There are **no retained executable/DLL/ancestor locks, bundle custody, ACL/owner
checks or permission changes**. The runner uses ordinary inherited permissions;
the parent chooses explicit private paths outside Git. Paths must be canonical,
absolute Windows paths, without UNC/ADS or relative components.

`tools` executes k6 version and both inspections, but returns
**tools=INCONCLUSIVE / exit2 / CHROMIUM_VERSION_PENDING**: it has not observed a
running Chrome version. The [Accepted version addendum](../../../docs/contracts/k6-windows-browser-version.md)
requires `browser.version()` in the **same existing browser case**, not Chrome
`--version`, file metadata, an extra browser launch or a new CDP query.
The expected version comes only from the manifest. Exactly one successful current-run
`browser-version` check establishes equality with the pin. The public label
`K6_API_CACHED_GETVERSION_EQUALITY` identifies k6's cached normal-connection version;
it does not claim an independent fresh CDP read. CLI Chrome version remains
**NOT_EXECUTED / INCONCLUSIVE**.

## Browser and process lifetime

The sole case uses a fresh context/page, `about:blank`, `6 * 7 === 42`, version
equality and normal finally closures. TEMP, HOME and profile-related environment
paths point only to the new run. The environment is constructed afresh: no inherited
proxy, K6 arguments/output/cloud/debug, loader, remote browser or user profile.
Usage reporting is disabled. Native Chromium protections remain enabled: no
`no-sandbox`, sandbox-disabling flag, TLS bypass, UI, screenshot or HAR.
The existing internal ephemeral k6 CDP transport is not exposed as a campaign port.
Observed owned Chrome arguments still reject sandbox/TLS bypass or external debug flags.

**Chromium PASS is functional, not a sandbox qualification.** It requires k6 exit0,
the three mandatory checks (evaluation, blank page and version) each passes1/fails0,
exact nonzero case/iteration cardinalities, a current-run summary, cleanup and
confidentiality. No renderer/token/integrity/AppContainer probe or attestation is
performed or required. Missing attestation no longer refuses the run or its PASS.

The existing in-memory `Child` helper retains one unnamed empty Job Object and
assigns the process atomically at creation, before resuming it. Only stdin/stdout/
stderr handles are inherited. PID, creation time, image and process handles are
observed; no process-token queries remain. Job membership and retained handles,
not names/old PIDs/journals, establish ownership of descendants. There is no new
supervisor, force-kill, elevation, breakaway or KILL_ON_JOB_CLOSE.

Normal exit is preferred. Version/inspect calls allow10s each; browser allows30s.
Timeout or interruption requests CTRL_BREAK only for the newly created console
group, then performs bounded graceful joins. Missing observations, active descendants
or unjoined handles prevent cleanup PASS. Data belonging to a surviving or
unproven child is preserved for the parent, never force-deleted. Abrupt host
termination cannot guarantee cleanup. Fixture success does not prove real Chromium
Job nesting, process observation or cleanup; the first real run remains necessary.

Run-directory creation still refuses reuse atomically and retains its created
directory identity for cleanup. Its permissions are inherited normally, no custom
descriptor is supplied, all handle sharing is allowed and the parent handle closes
immediately. This is cleanup bookkeeping, not path locking or code custody.
Disposed/unbound/replaced directory identities and reparse entries refuse cleanup
before traversal. Only the owned empty root is deleted through its creation handle.

## Results, confidentiality and failure diagnosis

The existing JS summary schema is unchanged. Required cardinalities are exactly
one expected/started/completed/iteration; browser k6/http requests=0, HTTP requests=1.
Absent/null/string/boolean counts, missing/failed checks, stale run IDs, failed exit,
vacant cases and truncated/absent summaries never pass. Browser requests=0 is not a
measurement of all system traffic. Final cleanup/privacy failure revokes both tools
and browser positives, including the projected Chrome version.

Each native stream is capped at64KiB, strictly UTF-8 decoded, and checked for private
sentinels and credential patterns before any positive projection. Overflow/missing
EOF/invalid encoding is nonpositive. Public stdout contains only the closed fixed-label
Windows result, hashes, version equality and run ID: no native streams, arbitrary
exception text, private paths/PIDs, secrets or raw metrics.

For diagnosis, bounded `child-N.stdout.log`, `child-N.stderr.log` and
`process-<guid>.json` stay **private** in the fresh output directory. Process journals
contain observed identities, exit, reason and capture/cleanup facts, not PID recovery
authority. After successful process cleanup, all profiles/source copies/summaries and
other run data are removed. Success removes the entire directory. Failure retains
only these top-level diagnostic filenames; cleanup PASS then means process/profile
retirement, not removal of the deliberately retained diagnostics. If process cleanup
is unproven, all data is preserved. No logs/journals are publication artifacts.

Observed parent failures also retain bounded `parent-error.json` (exception type,
error ID, script line and summary presence/size) and, when available up to64KiB,
`parent-summary.json`. Neither affects PASS or exposes raw errors publicly. The
strict reader rejects an empty checks object as SCHEMA_INVALID, not a positive result.

## HTTP remains conditional and closed

`validate`, `run` and `selftest` still validate input/JSON and the fixed origin, then
return `TARGET_BINDING_UNQUALIFIED` without k6 or a GET. The Linux origin binding is
unqualified; a Windows JSON boolean, URL or self-attested lease is not authority.
Without an existing authorized, anchored and reachable target, HTTP and real HTTP
selftests remain **INCONCLUSIVE**. No fake endpoint/deployer/tunnel/token is added.
Those real selftest paths are not implemented as executable HTTP paths while target
admission is absent. WSL and Linux remain INCONCLUSIVE; Linux CI remains suspended.

## Focused verification and next run

Author validation: `python.exe -B -m unittest discover -s ops/tests/k6/tests -p
'test_*windows*.py'` executed **32 tests, exit0, zero skips** on the simplified
code (final run40.739s, including archive-mismatch refusal). Known Python
fixtures and one existing Node fixture only; **no k6/Chrome/HTTP/runtime run**.
All11 observed fixture identities ended; their temporary data directories were
removed and only private identity inventories were retained. No forced termination
or permission changes. These results do not qualify the real tools.

The Node fixture exercises the unchanged scenario body with13 known version inputs,
including missing/throwing API, wrong149, malformed/missing expectations and finally
closures. Other focused checks cover pins/version, nonvacant/current summaries,
environment/privacy, timeout/interruption, descendants and owned cleanup. Tests for
the removed ACL, bundle-lock and renderer-token mechanisms have been removed.

After integrating this delta and a short check, the **lead** runs the existing
browser entrypoint with verified explicit Windows tool paths and a fresh private
output, without another hypothetical-risk review pipeline:

```powershell
powershell.exe -NoProfile -NonInteractive -File ops/tests/k6/run-windows.ps1 -Mode browser -Output $env:K6_BROWSER_RUN_DIR
```

This is a proposed next command, **not execution evidence**. The author does not run
the real binaries. Diagnose a real failure using its bounded private diagnostics,
exit and observed process facts; do not add protection without a reproducible defect
or explicit user requirement. Integration/publication/final cleanup remain with the lead.

## Official sources and retained choices

- [k6v2.3.0 browser_type.go](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/chromium/browser_type.go):
  explicit executable, ephemeral internal CDP and no default `no-sandbox` retained.
- [k6v2.3.0 Browser.Version](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/common/browser.go#L787),
  [mapping](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/browser/browser_mapping.go#L73):
  reuse the ratified same-case cached version API; no CLI probe or extra CDP client.
- [NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile),
  [CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew),
  [SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle):
  existing atomic creation/identity-bound cleanup retained, custom descriptors and
  denied sharing removed; ordinary permissions instead of ACL/custody machinery.
- [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects),
  [GenerateConsoleCtrlEvent](https://learn.microsoft.com/en-us/windows/console/generateconsolectrlevent):
  retain existing bounded process ownership/graceful cleanup, no force or new supervisor.
