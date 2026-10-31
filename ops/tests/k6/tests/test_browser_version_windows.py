"""Windows version-check regression; actual JS body, known local inputs only."""
import json
import os
from pathlib import Path
import shutil
import unittest

import test_runner_windows as base


PIN = "156.0.8078.73"


def current_summary():
    value = base.summary()
    value["checks"]["browser-version"] = {"passes": 1, "fails": 0}
    return value


@unittest.skipUnless(os.name == "nt", "native Windows helpers only")
class BrowserVersionWindowsTests(unittest.TestCase):
    ps = base.WindowsTests.ps
    value = base.WindowsTests.value

    @classmethod
    def setUpClass(cls):
        cls.node = shutil.which("node.exe")
        if cls.node is None:
            raise RuntimeError("Existing local node.exe is required; no installation")
        cls.node = str(Path(cls.node).resolve())
        base.WindowsTests.setUpClass.__func__(cls)

    @classmethod
    def tearDownClass(cls):
        base.WindowsTests.tearDownClass.__func__(cls)

    def test_actual_js_version_matrix_and_finally(self):
        # Only ESM syntax/import bindings are adapted. The actual complete scenario
        # function runs, with local collaborators; no copied version predicate.
        program = r"""
const fs = require('fs');
const vm = require('vm');
let source = fs.readFileSync(process.argv[2], 'utf8');
source = source.replace(/^import .*;\r?\n/gm, '');
if (!source.includes('export default async function ()')) throw Error('FIXTURE_SHAPE');
source = source.replace('export const options', 'const options')
  .replace('export default async function ()', 'async function scenario()')
  .replace('export function handleSummary', 'function handleSummary');
const pin = '156.0.8078.73';
const cases = [
  ['exact', pin, pin], ['149', '149.0.7827.53', pin],
  ['other', '156.0.8078.74', pin], ['empty', '', pin],
  ['malformed', 'Chrome/156.0.8078.73', pin], ['type', 156, pin],
  ['long', '1'.repeat(33), pin], ['sentinel', 'PRIVATE_SENTINEL', pin],
  ['missing-api', null, pin], ['throw-api', null, pin],
  ['expected-missing', pin, undefined], ['expected-modified', '149.0.7827.53', '149.0.7827.53'],
  ['expected-empty', pin, ''],
];
(async () => {
  const results = [];
  for (const [name, actual, expected] of cases) {
    const checks = {}; const events = [];
    const page = { goto: async x => events.push(x), evaluate: async () => 42,
      url: () => 'about:blank', waitForTimeout: async n => events.push(n),
      close: async () => events.push('page-close') };
    const context = { newPage: async () => { events.push('new-page'); return page; }, close: async () => events.push('context-close') };
    const browser = { newContext: async () => { events.push('new-context'); return context; } };
    if (name !== 'missing-api') browser.version = () => {
      events.push('version-api');
      if (name === 'throw-api') throw Error('PRIVATE_SENTINEL https://private.invalid/');
      return actual;
    };
    const env = {}; if (expected !== undefined) env.K6_EXPECTED_BROWSER_VERSION = expected;
    const console = Object.fromEntries(['log','error','warn','info','debug'].map(k => [k, () => events.push('log-attempt')]));
    const sandbox = { browser, console, __ENV: env, check: (v, predicates) => {
      for (const [key, predicate] of Object.entries(predicates)) checks[key] = predicate(v);
    }, started: { add: n => events.push('started:'+n) },
    completed: { add: n => events.push('completed:'+n) }, project: () => null };
    vm.runInNewContext(source + ';this.runScenario=scenario;', sandbox);
    await sandbox.runScenario();
    results.push({name, checks, events});
  }
  process.stdout.write(JSON.stringify(results));
})().catch(() => { process.stdout.write('FIXTURE_FAILED'); process.exitCode=9; });
"""
        scenario = base.ROOT / "scenarios/smoke/browser-prerequisite.js"
        script = self.directory / "known-version-inputs.js"
        script.write_text(program, encoding="utf-8")
        code = (
            f"$e=New-K6Environment {base.quote(self.directory)} '{base.RUN_ID}' 'C:\\known\\chrome.exe' @('PRIVATE_SENTINEL'); "
            f"$c=[K6Windows.Child]::new({base.quote(self.node)},@({base.quote(script)},{base.quote(scenario)}),$e,{base.quote(self.directory)}); "
            "try { $created=@($c.Identities|ForEach-Object {@{pid=$_.Pid;created=$_.Created;image=$_.Image;ended=$_.Ended}}); "
            f"[IO.File]::AppendAllText({base.quote(self.audit)},(($created|ConvertTo-Json -Depth 5 -Compress)+[Environment]::NewLine)); "
            "$r=$c.Finish(5000,1000,$null); if(-not $r.Cleanup-or -not $r.CaptureComplete-or $r.ExitCode-ne 0){throw 'FIXTURE_NOT_JOINED'}; "
            "$ended=@($c.Identities|ForEach-Object {@{pid=$_.Pid;created=$_.Created;image=$_.Image;ended=$_.Ended}}); "
            f"[IO.File]::AppendAllText({base.quote(self.audit)},(($ended|ConvertTo-Json -Depth 5 -Compress)+[Environment]::NewLine)); "
            "Assert-K6Privacy $r.Stdout @('PRIVATE_SENTINEL'); Assert-K6Privacy $r.Stderr @('PRIVATE_SENTINEL'); "
            "[Console]::Out.Write($r.Stdout) } finally {$c.Dispose()}"
        )
        rows = json.loads(self.ps(code))
        self.assertEqual(len(rows), 13)
        for row in rows:
            with self.subTest(case=row["name"]):
                if row["name"] == "expected-missing":
                    self.assertNotIn("browser-version", row["checks"])
                    self.assertNotIn("version-api", row["events"])
                else:
                    self.assertIs(row["checks"].get("browser-version"), row["name"] == "exact")
                self.assertTrue(row["checks"]["browser-evaluation"])
                self.assertTrue(row["checks"]["browser-blank"])
                self.assertEqual(row["events"].count("page-close"), 1)
                self.assertEqual(row["events"].count("context-close"), 1)
                self.assertEqual(row["events"].count("started:1"), 1)
                self.assertEqual(row["events"].count("completed:1"), 1)
                self.assertEqual(row["events"].count("new-context"), 1)
                self.assertEqual(row["events"].count("new-page"), 1)
                self.assertEqual(row["events"].count("version-api"), 0 if row["name"] in {"missing-api", "expected-missing"} else 1)
                self.assertNotIn("log-attempt", row["events"])
                self.assertEqual(row["events"].count("about:blank"), 1)
                self.assertIn(750, row["events"])

    def test_expected_version_is_closed_and_not_ambient(self):
        code = (
            "$env:K6_EXPECTED_BROWSER_VERSION='149.0.7827.53'; "
            f"$e=New-K6Environment {base.quote(self.directory)} '{base.RUN_ID}' 'C:\\known\\chrome.exe' @('PRIVATE_SENTINEL') '{PIN}'; "
            f"Assert-K6BrowserEligibility $e '{PIN}'; "
            "if($e['K6_EXPECTED_BROWSER_VERSION']-cne '156.0.8078.73'){exit 5}; "
            "$e.Remove('K6_EXPECTED_BROWSER_VERSION')|Out-Null; "
            f"try {{Assert-K6BrowserEligibility $e '{PIN}';exit 6}} catch {{if($_.Exception.Message-cne 'BROWSER_EXPECTATION_INVALID'){{throw}}}}; "
            f"$e['K6_EXPECTED_BROWSER_VERSION']='149.0.7827.53';try {{Assert-K6BrowserEligibility $e '{PIN}';exit 7}} catch {{if($_.Exception.Message-cne 'BROWSER_EXPECTATION_INVALID'){{throw}}}}; 'CLOSED'"
        )
        self.assertEqual(self.ps(code), "CLOSED")

    def test_current_summary_version_required_and_nonvacant(self):
        self.ps(f"Assert-K6Summary {self.value(current_summary())} '{base.RUN_ID}' 'Chromium' 0; 'OK'")
        values = []
        for check in (None, {"passes": 0, "fails": 0}, {"passes": 0, "fails": 1}, {"passes": 2, "fails": 0}):
            value = current_summary()
            if check is None:
                del value["checks"]["browser-version"]
            else:
                value["checks"]["browser-version"] = check
            values.append(value)
        value = current_summary(); value["run_id"] = "a" * 32; values.append(value)
        for value in values:
            self.assertEqual(self.ps(
                f"try {{Assert-K6Summary {self.value(value)} '{base.RUN_ID}' 'Chromium' 0;exit 5}} catch {{'REJECTED'}}"), "REJECTED")

    def qualification_code(self, state, child):
        return (
            "$p=[ordered]@{tools='INCONCLUSIVE';chromium='INCONCLUSIVE';versions=@{chromium=$null};browser_version_evidence='NOT_PROVEN'}; "
            f"$s={self.value(state)};$c={self.value(child)}; "
            f"Set-K6BrowserEvidence $p {self.value(current_summary())} '{base.RUN_ID}' $c $s '{PIN}'; "
        )

    def test_same_run_functional_pass_without_sandbox_attestation(self):
        state = dict(Cleanup=True, Privacy=True, Executed=1)
        child = dict(ExitCode=0, Cleanup=True, IdentityComplete=True, CaptureComplete=True, Overflow=False)
        code = self.qualification_code(state, child)
        code += "if($p.tools-cne 'PASS'-or $p.chromium-cne 'PASS'-or $p.versions.chromium-cne '156.0.8078.73'-or $p.browser_version_evidence-cne 'K6_API_CACHED_GETVERSION_EQUALITY'){exit 5}; 'FUNCTIONAL_PASS'"
        self.assertEqual(self.ps(code), "FUNCTIONAL_PASS")

    def test_cleanup_or_privacy_missing_blocks_positive_evidence(self):
        child = dict(ExitCode=0, Cleanup=True, IdentityComplete=True, CaptureComplete=True, Overflow=False)
        for key in ("Cleanup", "Privacy"):
            for missing in (False, True):
                state = dict(Cleanup=True, Privacy=True, Executed=1)
                if missing:
                    del state[key]
                else:
                    state[key] = False
                expression = self.qualification_code(state, child)
                # Assert an actual refusal label, not an arbitrary early exception.
                self.assertEqual(self.ps("try {" + expression + ";exit 5} catch {if($_.Exception.Message-notmatch '(CLEANUP_UNPROVEN|PRIVACY_REFUSED)'){throw};'BLOCKED'}"), "BLOCKED")

    def test_final_cleanup_failure_revokes_both_passes(self):
        for field in ("cleanup", "confidentiality"):
            for value in (None, "FAIL", "INCONCLUSIVE", "NOT_EXECUTED"):
                projection = dict(tools="PASS", chromium="PASS", cleanup="PASS", confidentiality="PASS",
                                  versions={"chromium": PIN}, browser_version_evidence="K6_API_CACHED_GETVERSION_EQUALITY")
                projection[field] = value
                code = f"$p={self.value(projection)};Complete-K6WindowsVerdicts $p;if($p.tools-eq 'PASS'-or $p.chromium-eq 'PASS'-or $null-ne $p.versions.chromium){{exit 5}};'REVOKED'"
                self.assertEqual(self.ps(code), "REVOKED")

    def test_tools_only_never_claims_runtime_version(self):
        self.assertEqual(self.ps("try {Assert-K6ToolsOnly;exit 5} catch {if($_.Exception.Message-cne 'CHROMIUM_VERSION_PENDING'){throw};'INCONCLUSIVE'}"), "INCONCLUSIVE")


if __name__ == "__main__":
    unittest.main()
