"""Keep native Cargo aliases aligned with the existing Sonar CI commands."""

from pathlib import Path
import json
import re
import shlex
import shutil
import subprocess
import tempfile
import tomllib
import unittest


ROOT = Path(__file__).resolve().parents[2]


def ci_arguments(workflow: str, lane: str) -> list[str]:
    common = re.search(r"common_clippy=\((.*?)\n\s*\)", workflow, re.S)
    if common is None:
        raise ValueError("CI common_clippy definition not found")
    command = re.search(
        r"cargo clippy ([^\n]*(?:\\\n[^\n]*)*?)"
        rf"> target/sonar/clippy-{lane}\.jsonl",
        workflow,
    )
    if command is None:
        raise ValueError(f"CI {lane} Clippy command not found")
    expanded = command.group(1).replace("\\\n", " ")
    expanded = expanded.replace('"${common_clippy[@]}"', common.group(1))
    return ["clippy", *shlex.split(expanded)]


class ClippyProfileTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.config = tomllib.loads((ROOT / ".cargo/config.toml").read_text())
        cls.workflow = (ROOT / ".github/workflows/ci.yml").read_text()

    def test_production_matches_ci(self):
        self.assertEqual(
            self.config["alias"]["clippy-ci-production"],
            ci_arguments(self.workflow, "production"),
        )

    def test_tests_match_ci(self):
        self.assertEqual(
            self.config["alias"]["clippy-ci-tests"],
            ci_arguments(self.workflow, "tests"),
        )

    def test_production_policy_drift_is_detected(self):
        changed = self.workflow.replace("-W clippy::unwrap_used", "-A clippy::unwrap_used")
        self.assertNotEqual(
            self.config["alias"]["clippy-ci-production"],
            ci_arguments(changed, "production"),
        )

    def test_missing_lane_is_not_silently_accepted(self):
        with self.assertRaises(ValueError):
            ci_arguments(self.workflow, "missing")


@unittest.skipUnless(shutil.which("powershell.exe"), "Windows PowerShell required")
class ClippyRunnerTests(unittest.TestCase):
    def run_fixture(self, scenario):
        # Replace only the process boundary, in a private copy of the runner.
        # No Cargo, Rust compilation or real repository reports are touched.
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        script = root / "ops/scripts/clippy-ci.ps1"
        script.parent.mkdir(parents=True)
        shutil.copyfile(ROOT / "ops/scripts/clippy-ci.ps1", script)
        reports = root / "target/sonar"
        reports.mkdir(parents=True)
        (reports / "clippy.json").write_text("stale report")
        fixture = r'''
function Get-Command { param($Name, $ErrorAction) [pscustomobject]@{Source='fake-cargo.exe'} }
function Start-Process {
    param($FilePath, $ArgumentList, $WorkingDirectory, [switch]$NoNewWindow,
          [switch]$Wait, [switch]$PassThru, $RedirectStandardOutput, $RedirectStandardError)
    if (-not $Wait) { throw 'The runner must explicitly wait' }
    Add-Content (Join-Path $WorkingDirectory 'calls.txt') $ArgumentList
    $failed = ($scenario -eq 'production-fails' -and $ArgumentList -eq 'clippy-ci-production') -or
              ($scenario -eq 'tests-fail' -and $ArgumentList -eq 'clippy-ci-tests')
    $text = '{"reason":"build-finished","success":true}' + "`n"
    if ($scenario -eq 'incomplete') { $text = '{"reason":"compiler-artifact"}' + "`n" }
    if ($scenario -eq 'empty') { $text = '' }
    [IO.File]::WriteAllText($RedirectStandardOutput, $text)
    [IO.File]::WriteAllText($RedirectStandardError, 'fixture stderr')
    $result = [pscustomobject]@{ExitCode=0}
    if ($failed) { $result.ExitCode = 7 }
    $result | Add-Member -MemberType ScriptMethod -Name Dispose -Value {}
    return $result
}
'''
        command = (
            "$scenario = " + "'" + scenario + "';\n" + fixture
            + "\n& '" + str(script).replace("'", "''") + "'"
        )
        result = subprocess.run(
            [shutil.which("powershell.exe"), "-NoProfile", "-NonInteractive", "-Command", command],
            capture_output=True, timeout=30,
        )
        calls = (root / "calls.txt").read_text().splitlines()
        return result, reports, calls

    def test_success_waits_for_both_lanes_and_merges_exact_bytes(self):
        result, reports, calls = self.run_fixture("success")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls, ["clippy-ci-production", "clippy-ci-tests"])
        expected = b"".join((reports / f"clippy-{lane}.jsonl").read_bytes()
                            for lane in ("production", "tests"))
        self.assertEqual((reports / "clippy.json").read_bytes(), expected)
        for line in expected.splitlines():
            self.assertTrue(json.loads(line)["success"])

    def test_failed_production_stops_before_tests_and_invalidates_old_report(self):
        result, reports, calls = self.run_fixture("production-fails")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(calls, ["clippy-ci-production"])
        self.assertEqual((reports / "clippy.json").read_bytes(), b"")

    def test_failed_tests_do_not_publish_a_combined_report(self):
        result, reports, calls = self.run_fixture("tests-fail")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 2)
        self.assertEqual((reports / "clippy.json").read_bytes(), b"")

    def test_zero_exit_without_finished_event_is_rejected(self):
        for scenario in ("incomplete", "empty"):
            with self.subTest(scenario=scenario):
                result, reports, calls = self.run_fixture(scenario)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(calls, ["clippy-ci-production"])
                self.assertEqual((reports / "clippy.json").read_bytes(), b"")


if __name__ == "__main__":
    unittest.main()
