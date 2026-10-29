"""Parent diagnostic fixtures only: never a Chromium or HTTP qualification."""
import contextlib
import io
import json
from pathlib import Path
import unittest
from unittest import mock

from test_boundaries import scratch
from test_runner import runner
import safety


def host():
    return {key: "ABSENT" for key in (
        "unprivileged_userns_clone", "apparmor_restrict_unprivileged_userns",
        "max_user_namespaces", "apparmor_enabled", "parent_seccomp", "parent_no_new_privs")}


class DirectTests(unittest.TestCase):
    def direct(self, root, *, code=0, failure=None, stdout=b"", stderr=b"", cleanup=True):
        child = safety.Child()
        child.owned = {123: {"pid": 123, "uid": 456, "start": 789}}
        child.code, child.cleanup = code, cleanup
        child.stdout, child.stderr = stdout, stderr
        with mock.patch.object(runner, "verify_tools") as verify, \
                mock.patch.object(runner, "Child", return_value=child), \
                mock.patch.object(child, "execute", side_effect=failure, return_value=code) as execute, \
                self.assertRaises(safety.Rejected) as found:
            runner.direct_chromium(root, {"k6": Path("/pinned/k6"), "chromium": Path("/pinned/chrome")})
        return found.exception, verify, execute

    def test_fixed_argv_private_fresh_profile_and_closed_environment(self):
        with scratch() as root:
            (root / "private").mkdir()
            failure, verify, execute = self.direct(root)
            profile = root / "private/chromium-profile"
            self.assertTrue(profile.is_dir())
            verify.assert_called_once_with({"k6": Path("/pinned/k6"), "chromium": Path("/pinned/chrome")},
                                           ("k6", "chromium"))
            args, kwargs = execute.call_args
            self.assertEqual(args[0], [str(Path("/pinned/chrome")), "--headless", "--dump-dom", "--timeout=5000",
                "--disable-background-networking", "--disable-component-update", "--no-first-run",
                "--no-default-browser-check", "--disable-sync", "--metrics-recording-only",
                "--user-data-dir=" + str(profile), "about:blank"])
            self.assertTrue(kwargs["browser"])
            self.assertEqual(kwargs["timeout"], 15)
            self.assertEqual(kwargs["env"], {**safety.child_environment(root),
                                           "K6_BROWSER_EXECUTABLE_PATH": str(Path("/pinned/chrome"))})
            self.assertEqual(failure.code, "PREREQUISITE_UNAVAILABLE")
            with self.assertRaises(safety.Rejected) as second, mock.patch.object(runner, "Child") as child:
                runner.direct_chromium(root, {})
            self.assertEqual(second.exception.code, "PATH_INVALID")
            child.assert_not_called()

    def test_changed_identity_refuses_before_child(self):
        with scratch() as root:
            (root / "private").mkdir()
            with mock.patch.object(runner, "verify_tools", side_effect=safety.Rejected("TOOL_UNPINNED", True)), \
                    mock.patch.object(runner, "Child") as child, self.assertRaises(safety.Rejected) as found:
                runner.direct_chromium(root, {})
            self.assertEqual(found.exception.code, "TOOL_UNPINNED")
            child.assert_not_called()

    def test_no_observed_identity_cannot_attest_cleanup(self):
        with scratch() as root:
            (root / "private").mkdir()
            child = safety.Child()
            child.code, child.cleanup, child.sandbox = 0, True, True
            with mock.patch.object(runner, "verify_tools"), mock.patch.object(runner, "Child", return_value=child), \
                    mock.patch.object(child, "execute", return_value=0), self.assertRaises(safety.Rejected) as found:
                runner.direct_chromium(root, {"chromium": Path("/pinned/chrome")})
        self.assertEqual(found.exception.code, "CLEANUP_FAILED")
        self.assertEqual(found.exception.cleanup, "FAIL")

    def test_native_exit_and_lexical_source_never_use_k6_zero_label(self):
        for code, category in [(0, "ZERO"), (23, "NONZERO"), (-6, "SIGNAL"), (None, "ABSENT")]:
            with scratch() as root:
                (root / "private").mkdir()
                failure, _, _ = self.direct(root, code=code)
            obs = failure.direct_observation
            self.assertEqual(obs["source"], "CHROMIUM_DIRECT")
            self.assertEqual(obs["native_exit"], category)
            self.assertEqual(obs["native_diagnostic"], "NATIVE_CAUSE_UNCLASSIFIED")
            self.assertNotEqual(failure.code, "SANDBOX_UNPROVEN")  # supervisor fixture, not proof
            self.assertEqual(failure.cleanup, "PASS")

    def test_supervisor_failures_remain_failures_with_distinct_termination(self):
        for code in ["CHILD_TIMEOUT", "INTERRUPTED", "CLEANUP_FAILED", "SANDBOX_UNPROVEN", "OUTPUT_LIMIT"]:
            with scratch() as root:
                (root / "private").mkdir()
                error = safety.Rejected(code, code == "SANDBOX_UNPROVEN")
                failure, _, _ = self.direct(root, code=-15, failure=error, cleanup=code != "CLEANUP_FAILED")
            self.assertEqual(failure.code, code)
            self.assertEqual(failure.cleanup, "FAIL" if code == "CLEANUP_FAILED" else "PASS")
            if code == "OUTPUT_LIMIT":
                self.assertIsNone(getattr(failure, "direct_observation", None))
            else:
                self.assertEqual(failure.direct_observation["termination"],
                                 code if code in {"CHILD_TIMEOUT", "INTERRUPTED"} else "EXITED")
        with scratch() as root:
            (root / "private").mkdir()
            failure, _, _ = self.direct(root, code=-15, failure=safety.Rejected("CHILD_TIMEOUT"), cleanup=False)
        self.assertEqual(failure.code, "CLEANUP_FAILED")
        self.assertEqual(failure.cleanup, "FAIL")
        self.assertEqual(failure.direct_observation["termination"], "CHILD_TIMEOUT")

    def test_unknown_exception_and_forged_native_exit_stay_closed(self):
        for error, native in [(RuntimeError("CONFIDENTIAL_SENTINEL"), 5), (None, {"path": "/private"})]:
            with scratch() as root:
                (root / "private").mkdir()
                failure, _, _ = self.direct(root, failure=error, code=native)
            self.assertEqual(failure.code, "INTERNAL_ERROR")
            if isinstance(native, dict):
                self.assertIsNone(getattr(failure, "direct_observation", None))

    def test_streams_privacy_invalid_utf8_and_limit_fail_before_projection(self):
        for raw, expected in [(b"CONFIDENTIAL_SENTINEL", "PRIVACY_FAILED"), (b"\xff", "PRIVACY_FAILED"),
                              (b"https://private.invalid", "PRIVACY_FAILED"),
                              (b"x" * (safety.LIMIT + 1), "OUTPUT_LIMIT")]:
            for stream in ("stdout", "stderr"):
                with scratch() as root:
                    (root / "private").mkdir()
                    failure, _, _ = self.direct(root, **{stream: raw})
                self.assertEqual(failure.code, expected)
                self.assertIsNone(getattr(failure, "direct_observation", None))

    def test_native_category_is_not_a_proven_cause(self):
        with scratch() as root:
            (root / "private").mkdir()
            failure, _, _ = self.direct(root, code=1, stderr=b"No usable sandbox! private suffix")
        self.assertEqual(failure.direct_observation["native_diagnostic"], "NO_USABLE_SANDBOX_REPORTED")
        self.assertNotIn("private suffix", json.dumps(failure.direct_observation))

    def test_known_label_can_survive_privacy_refusal_without_native_text(self):
        with scratch() as root:
            (root / "private").mkdir()
            failure, _, _ = self.direct(root, code=-6, stderr=b"No usable sandbox! https://private.invalid/path")
        self.assertEqual(failure.code, "PRIVACY_FAILED")
        self.assertEqual(failure.direct_observation["native_diagnostic"], "NO_USABLE_SANDBOX_REPORTED")
        self.assertNotIn("https", json.dumps(failure.direct_observation))

    def main(self, root, child_error=None):
        child = safety.Child()
        child.owned = {123: {"pid": 123, "uid": 456, "start": 789}}
        child.code, child.cleanup, child.sandbox = 0, True, True
        events = []
        with mock.patch.object(runner, "linux_platform"), \
                mock.patch.object(runner, "new_output", return_value=root), \
                mock.patch.object(runner, "pinned_tools", side_effect=lambda **kw:
                    events.append(("pins", kw)) or ({"chromium": Path("/pinned/chrome")}, {})), \
                mock.patch.object(runner, "version_checks", side_effect=lambda *a: events.append("versions") or {}), \
                mock.patch.object(runner, "host_metadata", return_value=host()), \
                mock.patch.object(runner, "verify_tools"), \
                mock.patch.object(runner, "Child", side_effect=lambda: events.append("child") or child), \
                mock.patch.object(child, "execute", return_value=0, side_effect=child_error), \
                mock.patch.object(runner, "publish") as publish, \
                mock.patch.object(runner, "scenario_run") as scenario, \
                contextlib.redirect_stdout(io.StringIO()) as log:
            code = runner.main(["diagnose-chromium", "--output", str(root)])
        return code, json.loads(log.getvalue()), publish, scenario, events

    def test_main_zero_witness_still_inconclusive_nonzero_and_no_publication(self):
        with scratch() as root:
            (root / "private").mkdir()
            code, result, publish, scenario, events = self.main(root)
        self.assertEqual(code, 2)
        self.assertEqual(result["Chromium"], "INCONCLUSIVE")
        self.assertEqual(result["HTTP"], "INCONCLUSIVE")
        self.assertEqual(result["diagnostic"], "PREREQUISITE_UNAVAILABLE")
        self.assertEqual(result["cleanup"], "PASS")
        self.assertEqual(result["confidentiality"], "PASS")
        self.assertEqual(result["direct_observation"]["source"], "CHROMIUM_DIRECT")
        self.assertEqual(events, [("pins", {"browser": True}), "versions", "child"])
        publish.assert_not_called()
        scenario.assert_not_called()

    def test_main_privacy_cleanup_and_sandbox_never_publish(self):
        for diagnostic in ["PRIVACY_FAILED", "CLEANUP_FAILED", "SANDBOX_UNPROVEN"]:
            with scratch() as root:
                (root / "private").mkdir()
                code, result, publish, _, _ = self.main(root, safety.Rejected(diagnostic))
            self.assertNotEqual(code, 0)
            self.assertNotEqual(result["Chromium"], "PASS")
            self.assertNotEqual(result["HTTP"], "PASS")
            self.assertEqual(result["diagnostic"], diagnostic)
            if diagnostic == "CLEANUP_FAILED":
                self.assertEqual(result["cleanup"], "FAIL")
            if diagnostic == "PRIVACY_FAILED":
                self.assertEqual(result["confidentiality"], "FAIL")
            publish.assert_not_called()

    def test_no_cli_url_binary_or_argument_override(self):
        for option in ["--url", "--executable", "--args", "--config"]:
            with self.assertRaises(safety.Rejected):
                runner.arguments(["diagnose-chromium", "--output", "/private/run", option, "CONFIDENTIAL_SENTINEL"])

    def test_pins_and_versions_refusal_precede_native_launch(self):
        for guard in ("pinned_tools", "version_checks"):
            with scratch() as root, mock.patch.object(runner, "linux_platform"), \
                    mock.patch.object(runner, "new_output", return_value=root), \
                    mock.patch.object(runner, "pinned_tools", return_value=({}, {})), \
                    mock.patch.object(runner, "version_checks", return_value={}), \
                    mock.patch.object(runner, guard, side_effect=safety.Rejected("TOOL_UNPINNED", True)), \
                    mock.patch.object(runner, "Child") as child, \
                    mock.patch.object(runner, "publish") as publish, contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(runner.main(["diagnose-chromium", "--output", str(root)]), 2)
            child.assert_not_called()
            publish.assert_not_called()

    def test_main_revalidates_forged_exception_projection(self):
        failure = safety.Rejected("CLEANUP_FAILED")
        failure.direct_observation = {"source": "CONFIDENTIAL_SENTINEL", "path": "/private/path"}
        with scratch() as root, mock.patch.object(runner, "linux_platform"), \
                mock.patch.object(runner, "new_output", return_value=root), \
                mock.patch.object(runner, "pinned_tools", return_value=({}, {})), \
                mock.patch.object(runner, "version_checks", return_value={}), \
                mock.patch.object(runner, "host_metadata", return_value=host()), \
                mock.patch.object(runner, "direct_chromium", side_effect=failure), \
                mock.patch.object(runner, "publish") as publish, contextlib.redirect_stdout(io.StringIO()) as log:
            self.assertEqual(runner.main(["diagnose-chromium", "--output", str(root)]), 1)
        self.assertNotIn("direct_observation", json.loads(log.getvalue()))
        self.assertNotIn("CONFIDENTIAL_SENTINEL", log.getvalue())
        self.assertNotIn("/private/path", log.getvalue())
        publish.assert_not_called()

    def test_browser_never_falls_back_to_direct(self):
        with scratch() as root, mock.patch.object(runner, "linux_platform"), \
                mock.patch.object(runner, "new_output", return_value=root), \
                mock.patch.object(runner, "pinned_tools", return_value=({}, {})), \
                mock.patch.object(runner, "version_checks", return_value={}), \
                mock.patch.object(runner, "host_metadata", return_value=host()), \
                mock.patch.object(runner, "scenario_run", side_effect=safety.Rejected("SANDBOX_UNPROVEN", True)), \
                mock.patch.object(runner, "direct_chromium") as direct, contextlib.redirect_stdout(io.StringIO()) as log:
            self.assertEqual(runner.main(["browser", "--output", str(root)]), 2)
        self.assertEqual(json.loads(log.getvalue())["browser_source"], "K6")
        direct.assert_not_called()

    def test_ci_comparison_is_separate_and_cannot_turn_browser_failure_green(self):
        workflow = (runner.REPO / ".github/workflows/k6-functional-smoke.yml").read_text()
        self.assertIn("failure() && steps.tools.outcome == 'success' && steps.browser.outcome == 'failure'", workflow)
        self.assertEqual(workflow.count("run-linux.py diagnose-chromium"), 1)
        self.assertLess(workflow.index("run-linux.py browser"), workflow.index("run-linux.py diagnose-chromium"))
        self.assertNotIn("continue-on-error", workflow)
        self.assertNotIn("chromium-direct-check/result.json", workflow)
        self.assertIn('"$K6_PRIVATE_ROOT/chromium-direct-check"', workflow)

    def test_closed_projection_rejects_extra_fields_labels_and_types(self):
        good = {"version": 1, "source": "CHROMIUM_DIRECT", "native_exit": "ZERO",
                "native_diagnostic": "NATIVE_CAUSE_UNCLASSIFIED", "termination": "EXITED"}
        self.assertEqual(runner.closed_direct_observation(good), good)
        for bad in [{**good, "source": "K6"}, {**good, "version": True}, {**good, "path": "/private"},
                    {**good, "native_exit": 0}, {**good, "termination": "CONFIDENTIAL_SENTINEL"},
                    {**good, "native_diagnostic": "K6_ZERO_EXIT_NO_SANDBOX_WITNESS"}]:
            with self.assertRaises(safety.Rejected):
                runner.closed_direct_observation(bad)


class HostMetadataTests(unittest.TestCase):
    def read(self, contents):
        def open_file(path, *a, **kw):
            value = contents.get(path.as_posix(), FileNotFoundError())
            if isinstance(value, BaseException):
                raise value
            return io.BytesIO(value)
        with mock.patch.object(Path, "open", open_file):
            return runner.host_metadata()

    def test_read_states_zero_absent_denied_unreadable_are_distinct(self):
        result = self.read({"/proc/sys/kernel/unprivileged_userns_clone": b"0\n",
            "/proc/sys/kernel/apparmor_restrict_unprivileged_userns": PermissionError(),
            "/proc/sys/user/max_user_namespaces": b"CONFIDENTIAL_SENTINEL",
            "/sys/module/apparmor/parameters/enabled": b"Y\n",
            "/proc/self/status": b"Seccomp:\t2\nNoNewPrivs:\t0\n"})
        self.assertEqual(result, {"unprivileged_userns_clone": "ZERO",
            "apparmor_restrict_unprivileged_userns": "DENIED", "max_user_namespaces": "UNREADABLE",
            "apparmor_enabled": "ENABLED", "parent_seccomp": "FILTER", "parent_no_new_privs": "ZERO"})
        self.assertEqual(set(self.read({}).values()), {"ABSENT"})
        self.assertEqual(self.read({"/proc/self/status": b"Seccomp: 2\n"})["parent_no_new_privs"], "ABSENT")

    def test_fixed_files_bounded_reads_and_closed_values(self):
        for raw in [b"-1", b"1.0", b"\xff", b"1" * 17000, b"1\nextra", b"00"]:
            result = self.read({"/proc/sys/user/max_user_namespaces": raw})
            self.assertEqual(result["max_user_namespaces"], "UNREADABLE")
        for raw, label in [(b"1", "ONE"), (b"1024\n", "POSITIVE"), (OSError(), "UNREADABLE")]:
            self.assertEqual(self.read({"/proc/sys/user/max_user_namespaces": raw})["max_user_namespaces"], label)
        self.assertEqual(self.read({"/proc/self/status": b"Seccomp: 2\nSeccomp: 0\nNoNewPrivs: 1\n"})["parent_seccomp"], "UNREADABLE")

    def test_host_projection_is_revalidated_not_printed_arbitrarily(self):
        for bad in [{**host(), "path": "/private"}, {**host(), "max_user_namespaces": "CONFIDENTIAL_SENTINEL"},
                    {**host(), "parent_seccomp": 2}, {**host(), "apparmor_enabled": "ZERO"}]:
            with self.assertRaises(safety.Rejected):
                runner.closed_host_metadata(bad)

    def test_host_read_uses_only_named_files_and_bounded_reads(self):
        paths, limits = [], []
        class Stream(io.BytesIO):
            def read(self, size=-1):
                limits.append(size)
                return super().read(size)
        def open_file(path, mode):
            paths.append(path.as_posix())
            self.assertEqual(mode, "rb")
            return Stream(b"0")
        with mock.patch.object(Path, "open", open_file):
            runner.host_metadata()
        self.assertEqual(paths, ["/proc/sys/kernel/unprivileged_userns_clone",
            "/proc/sys/kernel/apparmor_restrict_unprivileged_userns", "/proc/sys/user/max_user_namespaces",
            "/sys/module/apparmor/parameters/enabled", "/proc/self/status"])
        self.assertEqual(limits, [65, 65, 65, 65, 16385])

    def test_main_rejects_forged_host_metadata_before_child_without_echo(self):
        with scratch() as root, mock.patch.object(runner, "linux_platform"), \
                mock.patch.object(runner, "new_output", return_value=root), \
                mock.patch.object(runner, "pinned_tools", return_value=({}, {})), \
                mock.patch.object(runner, "version_checks", return_value={}), \
                mock.patch.object(runner, "host_metadata", return_value={**host(), "max_user_namespaces": "CONFIDENTIAL_SENTINEL"}), \
                mock.patch.object(runner, "direct_chromium") as direct, \
                mock.patch.object(runner, "publish") as publish, contextlib.redirect_stdout(io.StringIO()) as log:
            self.assertEqual(runner.main(["diagnose-chromium", "--output", str(root)]), 1)
        self.assertNotIn("CONFIDENTIAL_SENTINEL", log.getvalue())
        direct.assert_not_called()
        publish.assert_not_called()
