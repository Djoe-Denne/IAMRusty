"""Known native Windows fixtures only; never execute k6/Chrome or contact IAM.

Every live fixture is Python, with a bounded natural lifetime. No Kill/terminate,
HTTP fake, permission changes, or runtime startup.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
import uuid
import zipfile


ROOT = Path(__file__).resolve().parents[1]
PS = Path(os.environ.get("SystemRoot", r"C:\Windows")) / "System32/WindowsPowerShell/v1.0/powershell.exe"
TEMP = Path(os.environ.get("LOCALAPPDATA", "")) / "Temp/opencode"
RUN_ID = "1234567890abcdef1234567890abcdef"


def quote(value):
    return "'" + str(value).replace("'", "''") + "'"


def summary(kind="Chromium"):
    checks = ("iam-status", "iam-body") if kind == "HTTP" else ("browser-evaluation", "browser-blank", "browser-version")
    return dict(version=1, run_id=RUN_ID, phase=kind,
                cases=["iam-health" if kind == "HTTP" else "browser-prerequisite"],
                expected=1, started=1, completed=1, iterations=1,
                requests=1 if kind == "HTTP" else 0,
                checks={n: dict(passes=1, fails=0) for n in checks})


def inspect(kind="HTTP"):
    scenario = dict(executor="per-vu-iterations", vus=1, iterations=1,
                    maxDuration="15s" if kind == "HTTP" else "20s",
                    gracefulStop="2s" if kind == "HTTP" else "3s")
    if kind != "HTTP":
        scenario["options"] = {"browser": {"type": "chromium"}}
    return dict(scenarios={"iam-health" if kind == "HTTP" else "browser-prerequisite": scenario},
                thresholds=dict(checks=["rate==1"], case_started=["count==1"], case_completed=["count==1"]),
                maxRedirects=0, discardResponseBodies=False)


def lock():
    # Synthetic known inputs, not production trust pins and never executed.
    return dict(version=1, platform="windows-amd64", k6=dict(
        version="2.3.0", url="https://github.com/grafana/k6/releases/download/v2.3.0/k6-v2.3.0-windows-amd64.zip",
        checksum_url="https://github.com/grafana/k6/releases/download/v2.3.0/k6-v2.3.0-checksums.txt",
        archive="k6-v2.3.0-windows-amd64.zip",
        archive_sha256="112276d495e5741c968e2bc09ea6196099c1275bd6db9ee0875d173c7148ce43",
        executable=r"k6-v2.3.0-windows-amd64\k6.exe", executable_sha256="a" * 64),
        chromium=dict(version="156.0.8078.73", revision="1706539",
                      url="https://storage.googleapis.com/chrome-for-testing-public/156.0.8078.73/win64/chrome-win64.zip",
                      metadata_url="https://googlechromelabs.github.io/chrome-for-testing/156.0.8078.73.json",
                      archive="chrome-win64.zip", archive_sha256="b" * 64,
                      executable=r"chrome-win64\chrome.exe", executable_sha256="c" * 64))


@unittest.skipUnless(os.name == "nt", "native Windows interop only")
class WindowsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not PS.is_file() or not TEMP.is_dir():
            raise RuntimeError("Explicit Windows PowerShell5.1 and approved temp directory required")
        # Fresh fixture data, ordinary inherited Windows permissions.
        cls.directory = TEMP / ("k6-windows-unit-" + uuid.uuid4().hex)
        cls.directory.mkdir()
        cls.audit = cls.directory / "fixture-identities.jsonl"
        cls.fixture_count = 0

    @classmethod
    def tearDownClass(cls):
        # The suite checks Ended by retained handles BEFORE disposing each Child.
        # Persist just the observed fixture inventory privately for parent recovery.
        for lease in cls.directory.glob("process-*.json"):
            with cls.audit.open("a", encoding="utf-8") as out:
                out.write(lease.read_text(encoding="utf-8") + "\n")
        if cls.audit.exists():
            target = cls.directory.with_suffix(".jsonl")
            target.write_bytes(cls.audit.read_bytes())
        import shutil
        shutil.rmtree(cls.directory)

    def ps(self, code, expect=0):
        prefix = ("$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; "
                  f". {quote(ROOT / 'lib/windows.ps1')}; "
                  f"Add-Type -Path {quote(ROOT / 'lib/windows-native.cs')}; ")
        command = prefix + code
        script = self.directory / ("fixture-" + uuid.uuid4().hex + ".ps1")
        script.write_text(command, encoding="utf-8-sig")
        # No timeout= auto-kill. Interop and fixture lifetimes are bounded themselves.
        p = subprocess.run([str(PS), "-NoProfile", "-NonInteractive", "-File", str(script)],
                           capture_output=True, cwd=ROOT, env=os.environ.copy())
        script.unlink()
        out = p.stdout.decode("utf-8-sig", "strict").strip()
        err = p.stderr.decode("utf-8-sig", "replace")
        self.assertEqual(p.returncode, expect, (out, err))
        if expect == 0:
            self.assertEqual(err, "", err)
        return out

    def value(self, value):
        return "(ConvertFrom-K6Json " + quote(json.dumps(value)) + ")"

    def rejects(self, expression):
        out = self.ps("try { " + expression + "; 'UNEXPECTED_SUCCESS'; exit 9 } catch { 'REJECTED' }")
        self.assertEqual(out, "REJECTED")

    def test_json_actual_parser_duplicate_case_escaped_truncated_depth(self):
        for text in ['{"a":1,"a":2}', '{"a":1,"A":2}', '{"a":1,"\\u0061":2}',
                     '{"a":', '{"a":NaN}', '[01]', '[' * 17 + '0' + ']' * 17,
                     '{"a":1} trailing', 'true false']:
            with self.subTest(text=text):
                self.rejects("ConvertFrom-K6Json " + quote(text))
        self.assertEqual(self.ps("$v=ConvertFrom-K6Json '{\"a\":[1,true,null]}'; if($v.a.Count-ne 3){exit 5}; 'OK'"), "OK")

    def test_summary_positive_both_actual_callsites(self):
        for kind in ("HTTP", "Chromium"):
            self.ps(f"Assert-K6Summary {self.value(summary(kind))} '{RUN_ID}' '{kind}' 0; 'OK'")

    def test_empty_check_fields_schema_label_and_nominal_summary(self):
        empty_checks = summary(); empty_checks["checks"] = {}
        code = ("try {Assert-K6Fields (ConvertFrom-K6Json '{}') @('browser-evaluation'); exit 5} "
                "catch {if($_.Exception.Message-cne 'SCHEMA_INVALID'){throw}}; "
                f"try {{Assert-K6Summary {self.value(empty_checks)} '{RUN_ID}' 'Chromium' 0; exit 6}} "
                "catch {if($_.Exception.Message-cne 'SCHEMA_INVALID'){throw}}; "
                f"Assert-K6Summary {self.value(summary())} '{RUN_ID}' 'Chromium' 0; 'SCHEMA_AND_NOMINAL_OK'")
        self.assertEqual(self.ps(code), "SCHEMA_AND_NOMINAL_OK")
        self.assertIn("'--system-tags', 'name,check,scenario,group'", (ROOT / "run-windows.ps1").read_text(encoding="utf-8"))

    def test_summary_nonvacant_exact_and_current(self):
        cases = []
        for name in ("expected", "started", "completed", "iterations", "requests"):
            for bad in (None, 0, False, "1", 2):
                v = summary("HTTP"); v[name] = bad; cases.append(v)
        for name in ("started", "completed", "iterations", "checks"):
            v = summary(); del v[name]; cases.append(v)
        for patch in ({"cases": []}, {"cases": ["absent"]}, {"cases": "browser-prerequisite"},
                      {"phase": "HTTP"}, {"run_id": "a" * 32}, {"extra": "not allowed"},
                      {"phase": ["Chromium"]}, {"run_id": [RUN_ID]},
                      {"checks": {}}, {"checks": {"browser-evaluation": {"passes": 0, "fails": 0}}}):
            v = summary(); v.update(patch); cases.append(v)
        code = ""
        for v in cases:
            kind = v.get("phase") if v.get("phase") != "HTTP" or "iam-health" in v.get("cases", []) else "Chromium"
            code += f"try {{ Assert-K6Summary {self.value(v)} '{RUN_ID}' '{kind}' 0; exit 5 }} catch {{}}; "
        self.assertEqual(self.ps(code + "'ALL_REJECTED'"), "ALL_REJECTED")
        self.rejects(f"Assert-K6Summary {self.value(summary())} '{RUN_ID}' 'Chromium' 1")

    def test_inspect_exact_options_no_performance_threshold(self):
        for kind in ("HTTP", "Chromium"):
            self.ps(f"Assert-K6Inspect {self.value(inspect(kind))} '{kind}'; 'OK'")
        changes = [dict(vus=0), dict(iterations=2), dict(executor="shared-iterations"),
                   dict(executor=["per-vu-iterations"]), dict(maxDuration=["15s"]),
                   dict(maxDuration="1m"), dict(gracefulStop="30s")]
        code = ""
        for change in changes:
            v = inspect(); v["scenarios"]["iam-health"].update(change)
            code += f"try {{ Assert-K6Inspect {self.value(v)} 'HTTP'; exit 5 }} catch {{}}; "
        v = inspect(); v["thresholds"]["http_req_duration"] = ["p(95)<100"]
        code += f"try {{ Assert-K6Inspect {self.value(v)} 'HTTP'; exit 5 }} catch {{}}; 'OK'"
        self.assertEqual(self.ps(code), "OK")

    def test_pins_missing_unknown_linux_or_floating_rejected(self):
        self.rejects("Read-K6WindowsLock " + quote(self.directory / "absent-lock.json"))
        mutations = [("platform", "linux-amd64"), ("platform", ["windows-amd64"]), ("version", 2)]
        for parent, key, bad in (("k6", "version", "latest"), ("chromium", "version", "149.0.7827.53"),
                                 ("chromium", "revision", 1706539), ("k6", "version", ["2.3.0"]),
                                 ("k6", "executable_sha256", "0" * 64), ("chromium", "archive_sha256", "not-a-hash"),
                                 ("chromium", "url", "https://example.invalid/latest.zip")):
            v = lock(); v[parent][key] = bad
            path = self.directory / "bad-lock.json"; path.write_text(json.dumps(v), encoding="utf-8")
            self.rejects("Read-K6WindowsLock " + quote(path))
        for key, bad in mutations:
            v = lock(); v[key] = bad
            path = self.directory / "bad-lock.json"; path.write_text(json.dumps(v), encoding="utf-8")
            self.rejects("Read-K6WindowsLock " + quote(path))
        path = self.directory / "known-lock.json"; path.write_text(json.dumps(lock()), encoding="utf-8")
        self.ps("$v=Read-K6WindowsLock " + quote(path) + "; if($v.platform-cne 'windows-amd64'){exit 5}; 'OK'")

    def test_version_no_149_or_other_platform_or_multiline(self):
        self.ps("Assert-K6Version 'k6.exe v2.3.0 (commit/e088784614, go1.26.8, windows/amd64)' 'k6'; 'OK'")
        self.ps("Assert-K6Version 'k6 v2.3.0 (commit/abcdef, go1.26.1, windows/amd64)' 'k6'; Assert-K6Version 'Google Chrome for Testing 156.0.8078.73' 'chromium'; 'OK'")
        for value, kind in [("k6 v2.3.0 (commit/abcdef, go1.26.1, linux/amd64)", "k6"),
                            ("k6 v2.3.1 (commit/abcdef, go1.26.1, windows/amd64)", "k6"),
                            ("k6Xexe v2.3.0 (commit/e088784614, go1.26.8, windows/amd64)", "k6"),
                            ("k6.exe v2.3.1 (commit/e088784614, go1.26.8, windows/amd64)", "k6"),
                            ("k6.exe v2.3.0 (commit/e088784614, go1.26.8, linux/amd64)", "k6"),
                            ("Google Chrome 149.0.7827.53", "chromium"),
                            ("Google Chrome for Testing 156.0.8078.73\nEXTRA", "chromium")]:
            self.rejects(f"Assert-K6Version {quote(value)} '{kind}'")
        self.rejects("Assert-K6ToolsOnly")

    def test_environment_drops_ambient_proxy_k6_loader_and_profiles(self):
        code = ("$env:K6_BROWSER_ARGS='no-sandbox'; $env:HTTPS_PROXY='sensitive'; $env:K6_OUT='cloud'; $env:NODE_OPTIONS='bad'; "
                f"$e=New-K6Environment {quote(self.directory)} '{RUN_ID}' 'C:\\known\\chrome.exe' @('PRIVATE_SENTINEL'); "
                "foreach($n in @('K6_BROWSER_ARGS','HTTPS_PROXY','K6_OUT','NODE_OPTIONS','K6_INSECURE_SKIP_TLS_VERIFY')){if($e.ContainsKey($n)){exit 5}}; "
                "if($e['K6_NO_USAGE_REPORT']-cne 'true'-or $e['K6_BROWSER_HEADLESS']-cne 'true'){exit 6}; 'OK'")
        self.assertEqual(self.ps(code), "OK")

    def test_privacy_sentinels_and_credentials_in_positive_negative_streams(self):
        for text in ("PRIVATE_SENTINEL", "prefix private_sentinel suffix", "Bearer AAAAAAAAAAAA", "https://private.invalid/", r"C:\private\file",
                     "access_token=secret", "cookie: secret", "state=private", "eyJAAAAAAAAAA.payload.signature"):
            self.rejects(f"Assert-K6Privacy {quote(text)} @('PRIVATE_SENTINEL')")
        self.assertEqual(self.ps("Assert-K6Privacy '{\"checks\":2}' @('PRIVATE_SENTINEL'); 'OK'"), "OK")

    def test_explicit_paths_no_ads_or_relative(self):
        for value in ("relative", r"C:\x\..\y", r"C:\x\file:stream", r"\\server\share\x", "C:\\x.\\file"):
            self.rejects("[K6Windows.Native]::Absolute(" + quote(value) + ")")

    def test_private_files_pin_hash_and_pe_before_any_execution(self):
        toolroot = self.directory / "known-tool"
        exe = toolroot / "chrome-win64/chrome.exe"; exe.parent.mkdir(parents=True)
        blob = bytearray(128); blob[:2] = b"MZ"; blob[60:64] = (64).to_bytes(4, "little")
        blob[64:68] = b"PE\0\0"; blob[68:70] = (0x8664).to_bytes(2, "little"); blob[88:90] = (0x20B).to_bytes(2, "little")
        exe.write_bytes(blob); archive = toolroot / "chrome-win64.zip"
        with zipfile.ZipFile(archive, "w") as z:
            z.writestr("chrome-win64/chrome.exe", blob)
        pin = lock()["chromium"]; pin["executable_sha256"] = hashlib.sha256(blob).hexdigest(); pin["archive_sha256"] = hashlib.sha256(archive.read_bytes()).hexdigest()
        expr = f"$t=Open-K6PinnedTool {self.value(pin)} {quote(exe)}; if($t.Exe-cne {quote(exe)}){{exit 5}}; Assert-K6PinnedTool $t; 'OK'"
        self.assertEqual(self.ps(expr), "OK")
        archive_bytes = archive.read_bytes()
        archive.write_bytes(b"changed archive")
        self.rejects(f"Open-K6PinnedTool {self.value(pin)} {quote(exe)}")
        archive.write_bytes(archive_bytes)
        exe.write_bytes(b"not PE")
        self.rejects(f"Open-K6PinnedTool {self.value(pin)} {quote(exe)}")
        pin["executable_sha256"] = hashlib.sha256(exe.read_bytes()).hexdigest()
        self.rejects(f"Open-K6PinnedTool {self.value(pin)} {quote(exe)}")

    def test_owned_tree_reparse_refused_before_traversal(self):
        source = self.directory / "owned-tree"; target = self.directory / "owned-target"
        target.mkdir(); (target / "keep.txt").write_text("known", encoding="ascii")
        link = source / "junction"
        code = (f"$o=[K6Windows.OwnedDirectory]::Create({quote(source)}); [void](New-Item -ItemType Junction -Path {quote(link)} -Target {quote(target)}); "
                "try {try {Remove-K6OwnedTree $o;exit 5} catch {if($_.Exception.Message-cne 'CLEANUP_UNPROVEN'){throw}}; "
                f"if(-not [IO.File]::Exists({quote(target / 'keep.txt')})){{exit 6}}; "
                "'REFUSED' "
                f"}} finally {{[IO.Directory]::Delete({quote(link)},$false);$o.Dispose()}}")
        self.assertEqual(self.ps(code), "REFUSED")

    def test_entry_tools_browser_no_lock_or_invalid_pin_never_launch(self):
        for mode in ("tools", "browser"):
            output = self.directory / ("not-created-" + mode)
            env = os.environ.copy()
            env["K6_WINDOWS_EXE"] = str(self.directory / "not-pinned.exe")
            env["K6_BROWSER_EXECUTABLE_PATH"] = str(self.directory / "not-pinned-chrome.exe")
            p = subprocess.run([str(PS), "-NoProfile", "-NonInteractive", "-File", str(ROOT / "run-windows.ps1"),
                                "-Mode", mode, "-Output", str(output)], capture_output=True, env=env)
            self.assertEqual(p.returncode, 2)
            v = json.loads(p.stdout.decode("utf-8-sig"))
            self.assertIn(v["reason"], {"WINDOWS_LOCK_MISSING", "PIN_INVALID"}, v)
            self.assertEqual(v["tools"], "NOT_EXECUTED")
            self.assertEqual(v["cleanup"], "NOT_EXECUTED")
            self.assertFalse(output.exists()); self.assertEqual(p.stderr, b"")
    def fixture(self, code, timeout=3000, grace=1000, cancel=False, extra="", expect="NONE"):
        args = f"@('-B','-c',{quote(code)})"
        ps = (f"$e=New-K6Environment {quote(self.directory)} '{RUN_ID}' 'C:\\known\\chrome.exe' @('PRIVATE_SENTINEL'); "
              f"$c=[K6Windows.Child]::new({quote(sys.executable)},{args},$e,{quote(self.directory)}); "
              "try { "
              "$created=@($c.Identities|ForEach-Object {@{pid=$_.Pid;created=$_.Created;image=$_.Image;ended=$_.Ended}}); "
              f"[IO.File]::AppendAllText({quote(self.audit)},(($created|ConvertTo-Json -Depth 5 -Compress)+[Environment]::NewLine)); " +
              ("$c.Cancel(); " if cancel else "") +
              f"$r=$c.Finish({timeout},{grace},$null); " + extra +
              "$allEnded=$false; $watch=[Diagnostics.Stopwatch]::StartNew(); "
              "do {$allEnded=@($c.Identities|Where-Object {-not $_.Ended}).Count-eq 0; if(-not $allEnded){[Threading.Thread]::Sleep(10)}} while(-not $allEnded-and $watch.ElapsedMilliseconds-lt 3000); "
              "if(-not $allEnded){throw 'FIXTURE_NOT_JOINED'}; "
              "$ids=@($c.Identities|ForEach-Object {@{pid=$_.Pid;created=$_.Created;image=$_.Image;ended=$_.Ended}}); "
              f"[IO.File]::AppendAllText({quote(self.audit)},(($ids|ConvertTo-Json -Depth 5 -Compress)+[Environment]::NewLine)); "
              "@{reason=$r.Reason;cleanup=$r.Cleanup;complete=$r.IdentityComplete;capture=$r.CaptureComplete;total=$r.Total;active=$r.Active;exit=$r.ExitCode;overflow=$r.Overflow;stdout=$r.Stdout;stderr=$r.Stderr;identities=$ids}|ConvertTo-Json -Depth 5 -Compress "
              "} finally {$c.Dispose()}")
        result = json.loads(self.ps(ps))
        self.assertEqual(result["reason"], expect, result)
        self.assertTrue(result["identities"], result)
        for identity in result["identities"]:
            self.assertTrue(identity["ended"])
            self.assertGreater(identity["created"], 0)
            self.assertEqual(Path(identity["image"]).resolve(), Path(sys.executable).resolve())
        return result

    def test_native_success_nonzero_actual_handles_and_private_captures(self):
        r = self.fixture("import time;print('SAFE');time.sleep(.15)")
        self.assertTrue(r["cleanup"]); self.assertTrue(r["capture"]); self.assertEqual(r["exit"], 0)
        self.assertEqual(r["total"], 1); self.assertEqual(r["active"], 0); self.assertEqual(r["stdout"].strip(), "SAFE")
        r = self.fixture("import time,sys;time.sleep(.15);sys.exit(7)")
        self.assertEqual(r["exit"], 7); self.assertTrue(r["cleanup"])

    def test_native_descendant_contained_before_root_execution(self):
        code = ("import subprocess,sys,time; "
                "p=subprocess.Popen([sys.executable,'-B','-c','import time;time.sleep(.35)']); "
                "time.sleep(.15);p.wait()")
        r = self.fixture(code)
        self.assertEqual(r["total"], 2); self.assertEqual(len(r["identities"]), 2); self.assertTrue(r["cleanup"])

    def test_native_timeout_and_interruption_join_without_force(self):
        r = self.fixture("import time;time.sleep(.4)", timeout=40, expect="TIMEOUT")
        self.assertTrue(r["cleanup"])
        r = self.fixture("import time;time.sleep(.4)", cancel=True, expect="INTERRUPTED")
        self.assertTrue(r["cleanup"])

    def test_native_cleanup_failure_is_not_green_then_fixture_naturally_joins(self):
        r = self.fixture("import signal,time;signal.signal(signal.SIGBREAK,signal.SIG_IGN);time.sleep(.8)",
                         timeout=200, grace=10, expect="TIMEOUT")
        self.assertFalse(r["cleanup"])
        self.assertFalse(r["capture"])
        self.assertEqual(r["active"], 1)

    def test_native_unavailable_observation_handle_not_adopted_as_success(self):
        # Invalidate only the observation copy, not the retained root ownership
        # handle. Restore it before fixture cleanup; no OS handle is force-closed.
        code = (f"$e=New-K6Environment {quote(self.directory)} '{RUN_ID}' 'C:\\known\\chrome.exe' @('PRIVATE_SENTINEL'); "
                f"$c=[K6Windows.Child]::new({quote(sys.executable)},@('-B','-c','import time;time.sleep(.15)'),$e,{quote(self.directory)}); "
                "$id=$c.Identities[0]; $f=$id.GetType().GetField('Handle',[Reflection.BindingFlags]'NonPublic,Instance'); $handle=$f.GetValue($id); "
                "try {$f.SetValue($id,[IntPtr]::Zero);$r=$c.Finish(1000,1000,$null);if($r.Cleanup){exit 5};'NOT_GREEN'} "
                "finally {$f.SetValue($id,$handle); "
                "$record=@{pid=$id.Pid;created=$id.Created;image=$id.Image;ended=$id.Ended}; "
                f"[IO.File]::AppendAllText({quote(self.audit)},(($record|ConvertTo-Json -Compress)+[Environment]::NewLine)); $c.Dispose()}}")
        self.assertEqual(self.ps(code), "NOT_GREEN")

    def test_native_missing_handle_never_green(self):
        # Dispose while no process exists is not a fake success or adoption.
        self.rejects(f"[K6Windows.Child]::new({quote(self.directory / 'missing.exe')},@(),[Collections.Generic.Dictionary[string,string]]::new(),{quote(self.directory)})")

    def test_native_overflow_sticky_capture(self):
        r = self.fixture("import sys,time;sys.stdout.write('A'*70000);sys.stdout.flush();time.sleep(.2)", expect="OUTPUT_LIMIT")
        self.assertTrue(r["overflow"]); self.assertLessEqual(len(r["stdout"]), 65536)

    def test_real_invoke_callsite_privacy_withholds_sentinel_and_still_joins(self):
        # Known Python fixture passed through the *actual* Invoke-K6Child boundary.
        # This private helper is not exposed as CLI command/executable/argv override.
        archive = self.directory / "known-child.zip"; archive.write_bytes(b"known")
        code = (f"$ep={quote(sys.executable)}; $ap={quote(archive)}; "
                "$t=@{Exe=$ep;Archive=$ap;Pin=@{executable_sha256=[K6Windows.Native]::Hash($ep);archive_sha256=[K6Windows.Native]::Hash($ap)}}; "
                f"$e=New-K6Environment {quote(self.directory)} '{RUN_ID}' 'C:\\known\\chrome.exe' @('PRIVATE_SENTINEL'); "
                "$state=@{Cleanup=$true;Privacy=$true;Executed=0}; try { try { "
                f"Invoke-K6Child $t @('-B','-c',\"import time;print('PRIVATE_SENTINEL');time.sleep(.15)\") $e {quote(self.directory)} @('PRIVATE_SENTINEL') 3000 $state; exit 5 "
                "} catch {if($_.Exception.Message-notmatch 'PRIVACY_REFUSED'){throw}}; "
                "if(-not $state.Cleanup-or $state.Privacy){exit 6}; "
                f"$log=Join-Path {quote(self.directory)} 'child-1.stdout.log'; "
                "if([IO.FileInfo]::new($log).Length-gt 65536-or [IO.File]::ReadAllText($log).Trim()-cne 'PRIVATE_SENTINEL'){exit 7}; 'WITHHELD_AND_JOINED' "
                "} finally {}")
        self.assertEqual(self.ps(code), "WITHHELD_AND_JOINED")

    def test_target_all_modes_fail_closed_no_http(self):
        # These exercise the real entrypoint; no tool lock/bin is needed or invoked.
        missing = self.directory / "missing-config.json"
        for mode in ("validate", "run", "selftest"):
            p = subprocess.run([str(PS), "-NoProfile", "-NonInteractive", "-File", str(ROOT / "run-windows.ps1"),
                                "-Mode", mode, "-Config", str(missing)], capture_output=True)
            self.assertNotEqual(p.returncode, 0)
            v = json.loads(p.stdout.decode("utf-8-sig"))
            self.assertEqual(v["http"], "INCONCLUSIVE"); self.assertEqual(v["tools"], "NOT_EXECUTED")
            self.assertNotIn(str(missing), p.stdout.decode("utf-8-sig"))
        v = dict(version=1, profile="local-full", cluster="aiforall-local-full", context="kind-aiforall-local-full",
                 task_id="1" * 32, target_id="2" * 32, origin="http://127.0.0.1:18080", lease="self-attested", observations="self-attested")
        path = self.directory / "not-authority.json"; path.write_text(json.dumps(v), encoding="utf-8")
        self.assertEqual(self.ps(f"try {{Assert-K6TargetInput {quote(path)};exit 5}} catch {{if($_.Exception.Message-cne 'TARGET_BINDING_UNQUALIFIED'){{throw}};'CLOSED'}}"), "CLOSED")
        v["origin"] = "https://example.invalid"; path.write_text(json.dumps(v), encoding="utf-8")
        self.assertEqual(self.ps(f"try {{Assert-K6TargetInput {quote(path)};exit 5}} catch {{if($_.Exception.Message-cne 'TARGET_REFUSED'){{throw}};'CLOSED'}}"), "CLOSED")


if __name__ == "__main__":
    unittest.main()
