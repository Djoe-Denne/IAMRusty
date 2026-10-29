"""Closed diagnostics only; fixtures are not Linux/Chromium qualification."""
import contextlib
import io
import json
import os
from pathlib import Path
import unittest
from unittest import mock

from test_boundaries import scratch
from test_runner import runner, summary
import safety


def observation():
    return {"native_exit": "NONZERO", "summary": "ABSENT",
            "counters": {key: "ABSENT" for key in ("expected", "started", "completed", "iterations", "requests")},
            "renderer_observed": False,
            "namespace_reads": {key: ["NOT_OBSERVED"] for key in ("user", "pid")}}


class ObservationTests(unittest.TestCase):
    def test_native_exit_exact_types_and_categories(self):
        for code, expected in [(None, "ABSENT"), (0, "ZERO"), (99, "NONZERO"), (-9, "SIGNAL")]:
            self.assertEqual(safety.native_exit_category(code), expected)
        for code in [False, True, "1", 1.0, {"text": "CONFIDENTIAL_SENTINEL"}]:
            with self.assertRaises(safety.Rejected):
                safety.native_exit_category(code)

    def inspect_summary(self, raw):
        with scratch() as root:
            path = root / "summary.json"
            if raw is not None:
                path.write_bytes(raw)
            with mock.patch.object(runner, "private_read", lambda p: Path(p).read_bytes()):
                return runner.browser_summary_observation(path, "a" * 32)

    def test_current_summary_classifies_counts_without_qualifying_them(self):
        data = summary("Chromium")
        data.update(started=None, completed=0, iterations=2)
        expected = {"expected": "ONE", "started": "ABSENT", "completed": "ZERO", "iterations": "OTHER", "requests": "ZERO"}
        self.assertEqual(self.inspect_summary(json.dumps(data).encode()), ("VALID", expected))
        del data["started"]
        self.assertEqual(self.inspect_summary(json.dumps(data).encode()), ("VALID", expected))
        with self.assertRaises(safety.Rejected):
            runner.verify_summary(data, "a" * 32, "Chromium")

    def test_absent_truncated_stale_and_invalid_types_never_project_counts(self):
        absent = {key: "ABSENT" for key in observation()["counters"]}
        self.assertEqual(self.inspect_summary(None), ("ABSENT", absent))
        bad = [b'{', b'{}', b'[]', b'x' * (safety.ARTIFACT_LIMIT + 1), b'{"version":1,"version":1}']
        for key, value in [("run_id", "b" * 32), ("phase", "HTTP"), ("cases", []),
                           ("version", True), ("started", True), ("completed", 1.5),
                           ("iterations", -1), ("checks", {"unknown": {"passes": 1, "fails": 0}})]:
            bad.append(json.dumps({**summary("Chromium"), key: value}).encode())
        for raw in bad:
            with self.subTest(raw=raw[:60]):
                self.assertEqual(self.inspect_summary(raw), ("INVALID", absent))

    def test_summary_privacy_failure_is_not_just_invalid(self):
        for raw in [b'CONFIDENTIAL_SENTINEL', b'{"url":"https://example.invalid/private"}', b'\xff']:
            with self.assertRaises(safety.Rejected) as found:
                self.inspect_summary(raw)
            self.assertEqual(found.exception.code, "PRIVACY_FAILED")

    def test_summary_links_read_denials_and_size_do_not_bypass_private_reader(self):
        with scratch() as root:
            path = root / "summary.json"
            path.write_text("{}")
            with mock.patch.object(Path, "is_symlink", return_value=True), \
                    mock.patch.object(runner, "private_read") as read:
                self.assertEqual(runner.browser_summary_observation(path, "a" * 32)[0], "INVALID")
                read.assert_not_called()
            for error in [PermissionError(), safety.Rejected("PATH_INVALID"), safety.Rejected("OUTPUT_LIMIT")]:
                with mock.patch.object(runner, "private_read", side_effect=error):
                    self.assertEqual(runner.browser_summary_observation(path, "a" * 32)[0], "INVALID")

    def test_projection_allowlists_keys_labels_types_and_returns_fresh_data(self):
        good = observation()
        self.assertEqual(safety.closed_browser_observation(good), good)
        copied = safety.closed_browser_observation(good)
        copied["counters"]["started"] = "ONE"
        self.assertEqual(good["counters"]["started"], "ABSENT")
        invalid = [{**good, "path": "private"}, {**good, "native_exit": "CONFIDENTIAL_SENTINEL"},
                   {**good, "renderer_observed": 1}, {**good, "summary": "CURRENT"},
                   {**good, "counters": {**good["counters"], "started": 1}},
                   {**good, "namespace_reads": {"user": ["READABLE"], "pid": ["NOT_OBSERVED"]}},
                   {**good, "namespace_reads": {"user": ["NOT_OBSERVED", "NOT_OBSERVED"], "pid": []}},
                   {**good, "renderer_observed": True, "namespace_reads": {"user": ["/private/path"], "pid": ["OTHER"]}},
                   {**good, "counters": {**good["counters"], "requests": "ONE"}}]
        for value in invalid:
            with self.subTest(value=value), self.assertRaises(safety.Rejected):
                safety.closed_browser_observation(value)

    def test_only_owned_renderer_observations_are_accumulated(self):
        child = safety.Child()
        current = {"pid": 123, "start": 456, "uid": 789, "args": [b"--type=renderer"],
                   "namespace_reads": {"user": "DENIED", "pid": "DISAPPEARED"}}
        child.observe_renderer(current)
        self.assertFalse(child.renderer_observed)
        child.owned[123] = {**current}
        with mock.patch.object(os, "getuid", return_value=789, create=True):
            child.observe_renderer({**current, "start": 457})
            self.assertFalse(child.renderer_observed)
            child.observe_renderer(current)
            child.observe_renderer({**current, "namespace_reads": {"user": "READABLE", "pid": "OTHER"}})
        self.assertTrue(child.renderer_observed)
        self.assertEqual(child.namespace_reads, {"user": {"DENIED", "READABLE"}, "pid": {"DISAPPEARED", "OTHER"}})
        self.assertFalse(child.sandbox)
        self.assertEqual(child.sandbox_proof, [])

    def test_inventory_observation_is_kept_without_extra_scan_or_witness(self):
        child = safety.Child()
        child.observe_browser = True
        renderer = {"pid": 123, "start": 456, "uid": 789, "ppid": 42, "session": 42,
                    "args": [b"--type=renderer"], "namespace_reads": {"user": "DENIED", "pid": "NOT_OBSERVED"}}
        foreign = {**renderer, "pid": 124, "uid": 790}
        with mock.patch.object(Path, "iterdir", return_value=[Path("123"), Path("124")]) as scan, \
                mock.patch.object(safety, "proc", side_effect=[renderer, foreign]) as proc, \
                mock.patch.object(os, "getuid", return_value=789, create=True), \
                mock.patch.object(os, "getpid", return_value=99):
            child.inventory(42)
        scan.assert_called_once()
        self.assertEqual(proc.call_count, 2)
        self.assertEqual(set(child.owned), {123})
        self.assertTrue(child.renderer_observed)
        self.assertEqual(child.namespace_reads["user"], {"DENIED"})
        self.assertFalse(child.sandbox)

    def test_optional_read_errors_retain_identity_without_additional_reads(self):
        # Base stat/status are mandatory; optional reads may fail independently.
        stat_raw = "123 (chrome) " + " ".join(["S", "1", "123", "123"] + ["0"] * 15 + ["456"])
        errors = [(PermissionError(), "DENIED"), (FileNotFoundError(), "DISAPPEARED"), (OSError(), "OTHER")]
        for failed, (error, label) in ((name, error) for name in ("user", "pid", "cmdline") for error in errors):
            def text(path, *args, **kwargs):
                return stat_raw if path.name == "stat" else "Uid:\t789\t789\t789\t789\nSeccomp:\t2\n"
            def link(path):
                if path.name == failed:
                    raise error
                return "private-link"
            with mock.patch.object(Path, "read_text", text), \
                    mock.patch.object(Path, "open", side_effect=error if failed == "cmdline" else None,
                                      return_value=io.BytesIO(b"--type=renderer\0")), \
                    mock.patch.object(os, "readlink", side_effect=link) as readlink:
                value = safety.proc(123)
            self.assertEqual({k: value[k] for k in ("pid", "start", "uid")}, {"pid": 123, "start": 456, "uid": 789})
            expected = {"user": label, "pid": "NOT_OBSERVED"} if failed == "user" else \
                       {"user": "READABLE", "pid": label} if failed == "pid" else \
                       {"user": "NOT_OBSERVED", "pid": "NOT_OBSERVED"}
            self.assertEqual(value["namespace_reads"], expected)
            self.assertEqual(readlink.call_count, 1 if failed == "user" else 2 if failed == "pid" else 0)
            for name in ("user", "pid"):
                self.assertEqual(value[name + "ns"], "private-link" if expected[name] == "READABLE" else None)

    def browser_failure(self, transform=None, code=99, error_code="SANDBOX_UNPROVEN", child_fields=None):
        with scratch() as output:
            (output / "private").mkdir()
            child = safety.Child()
            child.cleanup, child.code = error_code != "CLEANUP_FAILED", code
            for key, value in (child_fields or {}).items():
                setattr(child, key, value)
            def execute(argv, **kwargs):
                values = dict(arg.split("=", 1) for i, arg in enumerate(argv) if i and argv[i - 1] == "--env")
                data = {**summary("Chromium"), "run_id": values["K6_RUN_ID"]}
                raw = transform(data) if transform else json.dumps(data).encode()
                if raw is not None:
                    Path(values["K6_SUMMARY_PATH"]).write_bytes(raw)
                raise safety.Rejected(error_code, error_code == "SANDBOX_UNPROVEN")
            with mock.patch.object(runner, "Child", return_value=child), \
                    mock.patch.object(child, "execute", side_effect=execute), \
                    mock.patch.object(runner, "verify_tools"), \
                    mock.patch.object(runner, "private_read", lambda p: Path(p).read_bytes()), \
                    self.assertRaises(safety.Rejected) as found:
                runner.scenario_run("Chromium", None, output, {"k6": Path("/k6"), "chromium": Path("/chrome")})
            return found.exception

    def test_sandbox_failure_still_observes_current_summary_not_success(self):
        failure = self.browser_failure()
        self.assertEqual(failure.code, "SANDBOX_UNPROVEN")
        self.assertTrue(failure.inconclusive)
        self.assertEqual(failure.browser_observation["native_exit"], "NONZERO")
        self.assertEqual(failure.browser_observation["summary"], "VALID")
        self.assertEqual(failure.browser_observation["counters"]["completed"], "ONE")
        self.assertEqual(failure.cleanup, "PASS")
        for transform, state in [(lambda _: None, "ABSENT"), (lambda _: b'{', "INVALID"),
                                 (lambda x: json.dumps({**x, "run_id": "stale"}).encode(), "INVALID")]:
            failure = self.browser_failure(transform, code=None, error_code="INTERRUPTED")
            self.assertEqual(failure.code, "INTERRUPTED")
            self.assertEqual(failure.browser_observation["summary"], state)
            self.assertEqual(failure.browser_observation["native_exit"], "ABSENT")

    def test_privacy_and_cleanup_errors_never_become_sandbox_success(self):
        failure = self.browser_failure(lambda _: b'CONFIDENTIAL_SENTINEL')
        self.assertEqual(failure.code, "PRIVACY_FAILED")
        self.assertEqual(failure.cleanup, "PASS")
        failure = self.browser_failure(error_code="CLEANUP_FAILED")
        self.assertEqual(failure.code, "CLEANUP_FAILED")
        self.assertEqual(failure.cleanup, "FAIL")
        failure = self.browser_failure(lambda _: b'CONFIDENTIAL_SENTINEL', error_code="CLEANUP_FAILED")
        self.assertEqual(failure.code, "PRIVACY_FAILED")
        self.assertEqual(failure.cleanup, "FAIL")

    def test_forged_supervisor_data_cannot_replace_failure_or_cleanup(self):
        for fields in [{"namespace_reads": {}}, {"namespace_reads": None},
                       {"renderer_observed": "CONFIDENTIAL_SENTINEL"},
                       {"code": {"argv": "/private/path"}},
                       {"namespace_reads": {"user": {"CONFIDENTIAL_SENTINEL"}, "pid": set()}}]:
            failure = self.browser_failure(error_code="CLEANUP_FAILED", child_fields=fields)
            self.assertEqual(failure.code, "CLEANUP_FAILED")
            self.assertEqual(failure.cleanup, "FAIL")
            self.assertIsNone(failure.browser_observation)

    def test_main_revalidates_exception_projection_and_never_publishes_on_failure(self):
        good = observation()
        for value in [good, {**good, "argv": "CONFIDENTIAL_SENTINEL"},
                      {**good, "native_exit": {"path": "/private/path"}}, ["untrusted"]]:
            failure = safety.Rejected("SANDBOX_UNPROVEN", True)
            failure.browser_observation, failure.cleanup = value, "PASS"
            with scratch() as root, mock.patch.object(runner, "linux_platform"), \
                    mock.patch.object(runner, "new_output", return_value=root), \
                    mock.patch.object(runner, "pinned_tools", return_value=({}, {})), \
                    mock.patch.object(runner, "version_checks", return_value={}), \
                    mock.patch.object(runner, "scenario_run", side_effect=failure), \
                    mock.patch.object(runner, "publish") as publish, contextlib.redirect_stdout(io.StringIO()) as log:
                self.assertEqual(runner.main(["browser", "--output", str(root)]), 2)
            result = json.loads(log.getvalue())
            self.assertEqual(result["Chromium"], "INCONCLUSIVE")
            if value == good:
                self.assertEqual(result["browser_observation"], good)
            else:
                self.assertNotIn("browser_observation", result)
            self.assertNotIn("CONFIDENTIAL_SENTINEL", log.getvalue())
            self.assertNotIn("/private/path", log.getvalue())
            publish.assert_not_called()

    def test_main_privacy_and_cleanup_failures_stay_nonpass_without_artifacts(self):
        for code, cleanup in [("PRIVACY_FAILED", "PASS"), ("CLEANUP_FAILED", "FAIL")]:
            failure = safety.Rejected(code)
            failure.browser_observation, failure.cleanup = observation(), cleanup
            with scratch() as root, mock.patch.object(runner, "linux_platform"), \
                    mock.patch.object(runner, "new_output", return_value=root), \
                    mock.patch.object(runner, "pinned_tools", return_value=({}, {})), \
                    mock.patch.object(runner, "version_checks", return_value={}), \
                    mock.patch.object(runner, "scenario_run", side_effect=failure), \
                    mock.patch.object(runner, "publish") as publish, contextlib.redirect_stdout(io.StringIO()) as log:
                self.assertEqual(runner.main(["browser", "--output", str(root)]), 1)
            result = json.loads(log.getvalue())
            self.assertEqual(result["Chromium"], "FAIL")
            self.assertEqual(result["cleanup"], cleanup)
            if code == "PRIVACY_FAILED":
                self.assertEqual(result["confidentiality"], "FAIL")
            publish.assert_not_called()
