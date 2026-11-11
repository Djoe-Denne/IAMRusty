"""Private-parent gate fixtures only, not a browser or runtime qualification."""
import contextlib
import copy
import io
import json
from pathlib import Path
import time
import unittest
from unittest import mock

from test_boundaries import scratch
from test_direct_chromium import host
from test_runner import runner
import safety


LOCK = json.loads((runner.HERE / "config/tools.lock.json").read_text())


@contextlib.contextmanager
def environment(version="22.04", *, system="linux", machine="x86_64", uid=123, distro="ubuntu"):
    with mock.patch.object(runner.sys, "platform", system), \
            mock.patch.object(runner.platform, "machine", return_value=machine), \
            mock.patch.object(runner.os, "getuid", return_value=uid, create=True), \
            mock.patch.object(Path, "read_text", return_value=f'ID={distro}\nVERSION_ID="{version}"\n'):
        yield


def metadata():
    return {"observed": {"kernel": "6.8.0", "architecture": "x86_64", "runner_image": "ubuntu22",
                         "runner_image_version": "20261026.341.1", "uid": 123},
            "versions": {name: LOCK[name]["version"] for name in ("k6", "chromium")},
            "tools": {name: LOCK[name]["executable_sha256"] for name in ("k6", "chromium")}}


def refusal():
    now = int(time.time() * 1000)
    return {"version": 1, "HTTP": "INCONCLUSIVE", "Chromium": "INCONCLUSIVE", "cleanup": "PASS",
            "confidentiality": "PASS", "run_id": "a" * 32, "started_unix_ms": now - 1000,
            "finished_unix_ms": now, "cleanup_scope": "LAUNCHER_CHILDREN_ONLY", "parent_runtime_cleanup": "INCONCLUSIVE",
            "comparison": {"platform": "ubuntu-22.04-linux-amd64-diagnostic-only",
                "qualification": "NOT_EQUIVALENT_TO_UBUNTU_24_04", "reference_run": "40973768642",
                "browser_launch": "ATTEMPTED", "case": "NOT_PROVEN", "sandbox_witness": "NOT_PROVEN", **metadata()},
            "browser_source": "K6", "browser_host": host(), "tools": "PASS", "diagnostic": "SANDBOX_UNPROVEN",
            "browser_native_diagnostic": "NATIVE_CAUSE_UNCLASSIFIED", "k6_exit": "SCRIPT_EXCEPTION",
            "browser_observation": {"native_exit": "NONZERO", "summary": "VALID",
                "counters": {"expected": "ONE", "started": "ZERO", "completed": "ZERO", "iterations": "ONE", "requests": "ZERO"},
                "renderer_observed": False, "namespace_reads": {"user": ["NOT_OBSERVED"], "pid": ["NOT_OBSERVED"]}}}


class ExitCodeTests(unittest.TestCase):
    def test_exact_pinned_table_and_unknown_signal_absent(self):
        table = {97: "CLOUD_TEST_RUN_FAILED", 98: "CLOUD_FAILED_TO_GET_PROGRESS", 99: "THRESHOLDS_HAVE_FAILED",
                 100: "SETUP_TIMEOUT", 101: "TEARDOWN_TIMEOUT", 102: "GENERIC_TIMEOUT",
                 103: "SCRIPT_STOPPED_FROM_REST_API", 104: "INVALID_CONFIG", 105: "EXTERNAL_ABORT",
                 106: "CANNOT_START_REST_API", 107: "SCRIPT_EXCEPTION", 108: "SCRIPT_ABORTED", 109: "GO_PANIC",
                 110: "MARKED_AS_FAILED", 0: "ZERO", None: "ABSENT", -15: "SIGNAL", 23: "UNKNOWN", 999999: "UNKNOWN"}
        for code, expected in table.items():
            self.assertEqual(runner.k6_exit_category(code), expected)
        for value in (False, True, "107", 107.0, {"bad": "CONFIDENTIAL_SENTINEL"}):
            with self.subTest(value=value), self.assertRaises(safety.Rejected):
                runner.k6_exit_category(value)

    def test_table_never_applies_to_chromium_direct(self):
        self.assertEqual(safety.native_exit_category(107), "NONZERO")
        self.assertEqual(safety.native_exit_category(99), "NONZERO")


class ParentGateTests(unittest.TestCase):
    def test_direct_flag_keeps_closed_platform_and_requires_browser_result(self):
        for variant in ({"version": "24.04"}, {"version": "26.04"}, {"system": "win32"},
                        {"distro": "debian"}, {"uid": 0}, {"machine": "aarch64"}):
            with environment(**variant), mock.patch.object(runner, "Child") as child, \
                    contextlib.redirect_stdout(io.StringIO()) as log:
                code = runner.main(["diagnose-chromium", "--compare-ubuntu-2204", "--browser-result", "/private/refusal",
                                    "--output", "/private/new"])
            self.assertNotEqual(code, 0)
            self.assertEqual(json.loads(log.getvalue())["diagnostic"], "PLATFORM_UNQUALIFIED")
            child.assert_not_called()
        with environment(), mock.patch.object(runner, "Child") as child, contextlib.redirect_stdout(io.StringIO()):
            self.assertNotEqual(runner.main(["diagnose-chromium", "--compare-ubuntu-2204", "--output", "/private/new"]), 0)
        child.assert_not_called()
        with environment("24.04"), mock.patch.object(runner, "Child") as child, contextlib.redirect_stdout(io.StringIO()):
            self.assertNotEqual(runner.main(["diagnose-chromium", "--browser-result", "", "--output", "/private/new"]), 0)
        child.assert_not_called()

    def test_closed_predicate_accepts_only_current_incomplete_refusal(self):
        value = refusal()
        self.assertTrue(runner.admissible_browser_refusal(value, metadata(), host()))
        changes = [("tools", "FAIL"), ("cleanup", "FAIL"), ("confidentiality", "FAIL"),
                   ("confidentiality", "NOT_QUALIFIED"), ("diagnostic", "TOOL_VERSION"),
                   ("HTTP", "PASS"), ("Chromium", "PASS"), ("browser_source", "CHROMIUM_DIRECT"),
                   ("k6_exit", "CONFIDENTIAL_SENTINEL"), ("k6_exit", "ZERO"), ("version", True),
                   ("started_unix_ms", True), ("finished_unix_ms", int(time.time() * 1000) - 180000),
                   ("finished_unix_ms", int(time.time() * 1000) + 10000), ("run_id", "old")]
        for key, replacement in changes:
            with self.subTest(key=key, replacement=replacement):
                self.assertFalse(runner.admissible_browser_refusal({**value, key: replacement}, metadata(), host()))
        for mutate in (lambda x: x["comparison"].update(case="COMPLETE"),
                       lambda x: x["comparison"].update(sandbox_witness="OBSERVED"),
                       lambda x: x["comparison"]["observed"].update(uid=124),
                       lambda x: x["comparison"]["tools"].update(k6="b" * 64),
                       lambda x: x["comparison"]["observed"].update(runner_image_version="other"),
                       lambda x: x["browser_observation"].update(summary="ABSENT"),
                       lambda x: x["browser_observation"]["counters"].update(completed="ONE"),
                       lambda x: x["browser_observation"]["counters"].update(expected="ABSENT"),
                       lambda x: x.update(extra="CONFIDENTIAL_SENTINEL")):
            changed = copy.deepcopy(value)
            mutate(changed)
            self.assertFalse(runner.admissible_browser_refusal(changed, metadata(), host()))
        for malformed in (None, {}, [], "CONFIDENTIAL_SENTINEL"):
            self.assertFalse(runner.admissible_browser_refusal(malformed, metadata(), host()))

    def direct(self, root, raw, *, comparison=True):
        previous = root / "previous" / "private"
        previous.mkdir(parents=True, exist_ok=True)
        receipt = previous / "comparison-result.json"
        if raw is not None:
            receipt.chmod(0o600) if receipt.exists() else None
            receipt.write_bytes(json.dumps(raw).encode() if isinstance(raw, dict) else raw)
            receipt.chmod(0o400)
        output = root / "new-direct"
        output.mkdir(exist_ok=True)
        (output / "private").mkdir(exist_ok=True)
        child = safety.Child()
        child.code, child.cleanup, child.sandbox = 0, True, True
        child.owned = {1234: {"pid": 1234, "uid": 123, "start": 5678}}
        argv = ["diagnose-chromium", "--output", str(output), "--browser-result", str(receipt)]
        if comparison:
            argv += ["--compare-ubuntu-2204"]
        with environment(), mock.patch.object(runner, "new_output", return_value=output), \
                mock.patch.object(runner, "pinned_tools", return_value=({"k6": Path("/pinned/k6"), "chromium": Path("/pinned/chrome")}, LOCK)), \
                mock.patch.object(runner, "comparison_metadata", return_value=metadata()), \
                mock.patch.object(runner, "host_metadata", return_value=host()), \
                mock.patch.object(runner, "private_read", lambda p: Path(p).read_bytes()), \
                mock.patch.object(runner, "version_checks", return_value={}) as versions, \
                mock.patch.object(runner, "verify_tools"), mock.patch.object(runner, "Child", return_value=child) as factory, \
                mock.patch.object(child, "execute", return_value=0) as execute, \
                mock.patch.object(runner, "publish") as publish, contextlib.redirect_stdout(io.StringIO()) as log:
            code = runner.main(argv)
        publish.assert_not_called()
        self.assertNotEqual(code, 0)
        self.assertNotIn("CONFIDENTIAL_SENTINEL", log.getvalue())
        result = json.loads(log.getvalue())
        self.assertEqual(result["HTTP"], "INCONCLUSIVE")
        self.assertEqual(result["Chromium"], "INCONCLUSIVE")
        self.assertNotIn("k6_exit", result)
        return result, factory, execute, versions

    def test_valid_refusal_permits_one_native_zero_direct_never_qualification(self):
        with scratch() as root:
            result, factory, execute, versions = self.direct(root, refusal())
            self.assertEqual(result["direct_observation"]["native_exit"], "ZERO")
            self.assertEqual(result["cleanup"], "PASS")
            self.assertEqual(execute.call_count, 1)
            versions.assert_called_once()
            result, factory, execute, versions = self.direct(root, None)
            factory.assert_not_called()
            execute.assert_not_called()
            versions.assert_not_called()

    def test_missing_invalid_stale_hostile_or_failed_refusal_precedes_every_child(self):
        for raw in (None, b"{", b"CONFIDENTIAL_SENTINEL", b"x" * (safety.ARTIFACT_LIMIT + 1), {},
                    {**refusal(), "cleanup": "FAIL"}, {**refusal(), "confidentiality": "FAIL"},
                    {**refusal(), "tools": "FAIL"}, {**refusal(), "finished_unix_ms": 1}):
            with self.subTest(raw=str(raw)[:70]), scratch() as root:
                _, factory, execute, versions = self.direct(root, raw)
                factory.assert_not_called()
                execute.assert_not_called()
                versions.assert_not_called()

    def test_browser_receipt_does_not_activate_nominal_direct(self):
        with scratch() as root:
            _, factory, execute, versions = self.direct(root, refusal(), comparison=False)
            factory.assert_not_called()
            execute.assert_not_called()
            versions.assert_not_called()

    def test_workflow_no_longer_launches_browser_prerequisite(self):
        workflow = (runner.REPO / ".github/workflows/k6-functional-smoke.yml").read_text()
        self.assertNotIn("run-linux.py browser", workflow)
        self.assertNotIn("ubuntu2204-comparison:", workflow)
        self.assertNotIn("steps.browser.outcome", workflow)
        self.assertNotIn("continue-on-error", workflow)
