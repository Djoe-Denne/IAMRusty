import contextlib
import copy
import io
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

from test_runner import runner, summary
import safety


@contextlib.contextmanager
def scratch():
    temp = tempfile.TemporaryDirectory()
    root = Path(temp.name)
    try:
        yield root
    finally:
        for path in root.rglob("*"):
            path.chmod(0o700 if path.is_dir() else 0o600)
        temp.cleanup()


class LaunchTests(unittest.TestCase):
    def test_invalid_config_precedes_tools_guard_and_children(self):
        for raw in [{}, {"origin": "https://CONFIDENTIAL_SENTINEL_foreign"}]:
            with scratch() as root:
                config = root / "config.json"
                config.write_text(json.dumps(raw))
                with mock.patch.object(runner, "linux_platform"), mock.patch.object(runner, "private_read", lambda p: Path(p).read_bytes()), \
                        mock.patch.object(runner, "pinned_tools") as tools, mock.patch.object(runner, "live_target") as guard, \
                        mock.patch.object(runner, "Child") as child, contextlib.redirect_stdout(io.StringIO()) as log:
                    code = runner.main(["run", "--config", str(config), "--output", str(root / "out")])
                self.assertNotEqual(code, 0)
                tools.assert_not_called()
                guard.assert_not_called()
                child.assert_not_called()
                self.assertNotIn("CONFIDENTIAL_SENTINEL", log.getvalue())

    def test_cli_rejection_does_not_echo_secret(self):
        with contextlib.redirect_stdout(io.StringIO()) as log, contextlib.redirect_stderr(io.StringIO()) as err:
            code = runner.main(["run", "--unknown", "CONFIDENTIAL_SENTINEL"])
        self.assertNotEqual(code, 0)
        self.assertNotIn("CONFIDENTIAL_SENTINEL", log.getvalue() + err.getvalue())

    def test_target_gate_precedes_k6(self):
        with scratch() as output, mock.patch.object(runner, "live_target", side_effect=runner.Rejected("TARGET_UNAVAILABLE", True)), \
                mock.patch.object(runner, "Child") as child:
            with self.assertRaises(runner.Rejected):
                runner.scenario_run("HTTP", {"origin": "http://127.0.0.1:18080"}, output, {"k6": Path("/tool/k6")})
            child.assert_not_called()

    def test_run_cli_without_public_binding_refuses_before_any_tool_execution(self):
        with mock.patch.object(runner, "linux_platform"), \
                mock.patch.object(runner, "configuration", return_value={"origin": "http://127.0.0.1:18080"}), \
                mock.patch.object(runner, "new_output", return_value=Path("/private/uncreated")), \
                mock.patch.object(runner, "live_target", side_effect=runner.Rejected("TARGET_BINDING_UNQUALIFIED", True)), \
                mock.patch.object(runner, "pinned_tools") as tools, \
                mock.patch.object(runner, "Child") as child, contextlib.redirect_stdout(io.StringIO()) as log:
            code = runner.main(["run", "--config", "/private/config", "--output", "/private/uncreated"])
        self.assertEqual(code, 2)
        projection = json.loads(log.getvalue())
        self.assertEqual(projection["HTTP"], "INCONCLUSIVE")
        self.assertEqual(projection["diagnostic"], "TARGET_BINDING_UNQUALIFIED")
        tools.assert_not_called()
        child.assert_not_called()

    def test_default_binding_cannot_be_self_attested(self):
        with mock.patch.object(runner.importlib.util, "spec_from_file_location") as spec, \
                mock.patch.object(runner.importlib.util, "module_from_spec") as module:
            spec.return_value.name = "test_k6_authority"
            module.return_value.LocalClusterTargetAnchor.return_value.check.return_value = {"task_id": "a" * 32}
            with self.assertRaises(runner.Rejected) as found:
                runner.live_target({"lease": "/private/lease", "task_id": "a" * 32})
            self.assertEqual(found.exception.code, "TARGET_BINDING_UNQUALIFIED")
            module.return_value.LocalClusterTargetAnchor.assert_called_once_with(
                "/private/lease", expected_cluster="aiforall-local-full")
            module.return_value.LocalClusterTargetAnchor.return_value.check.assert_called_once_with()

    def launch(self, transformation=None, code=0, native_error=None, phase="HTTP"):
        with scratch() as output:
            (output / "private").mkdir()
            def execute(argv, **kwargs):
                if native_error:
                    raise native_error
                env = dict(arg.split("=", 1) for i, arg in enumerate(argv) if i and argv[i-1] == "--env")
                value = summary(phase)
                value["run_id"] = env["K6_RUN_ID"]
                if transformation:
                    value = transformation(value)
                if value is not None:
                    Path(env["K6_SUMMARY_PATH"]).write_bytes(value if isinstance(value, bytes) else json.dumps(value).encode())
                return code
            with mock.patch.object(runner, "live_target") as guard, mock.patch.object(runner, "Child") as child, \
                    mock.patch.object(runner, "private_read", lambda p: Path(p).read_bytes()):
                child.return_value.cleanup = True
                child.return_value.execute.side_effect = execute
                try:
                    value = runner.scenario_run(phase, {"origin": "http://127.0.0.1:18080"}, output,
                                                {"k6": Path("/tool/k6"), "chromium": Path("/tool/chrome")})
                    return value, guard.call_count
                except runner.Rejected as error:
                    self.assertEqual(guard.call_count, 2 if phase == "HTTP" else 0)
                    raise error

    def test_success_requires_current_complete_summary_and_post_guard(self):
        result, calls = self.launch()
        self.assertEqual(calls, 2)
        self.assertEqual(result["cleanup"], "PASS")

    def test_no_summary_truncated_stale_absent_case_and_wrong_exit(self):
        for transform, code in [(lambda _: None, 0), (lambda _: b'{', 0),
                                (lambda x: {**x, "run_id": "stale"}, 0),
                                (lambda x: {**x, "cases": []}, 0), (None, 99)]:
            with self.subTest(code=code), self.assertRaises(runner.Rejected):
                self.launch(transform, code)

    def test_native_error_and_interrupt_still_postcheck(self):
        for code in ["TOOL_FAILED", "INTERRUPTED", "OUTPUT_LIMIT", "PRIVACY_FAILED", "CLEANUP_FAILED"]:
            with self.subTest(code=code), self.assertRaises(runner.Rejected):
                self.launch(native_error=runner.Rejected(code))

    def test_browser_never_reads_application_guard(self):
        result, calls = self.launch(phase="Chromium")
        self.assertEqual(calls, 0)
        self.assertEqual(result["sandbox"], "PASS")

    def test_false_check_requires_healthy_baseline_and_actual_failure(self):
        value = summary()
        value["negative_baseline"] = 1
        value["checks"]["iam-body"] = {"passes": 0, "fails": 1}
        runner.verify_false_check(value, "a" * 32, 99)
        for changed, code in [(value, 0), ({**value, "negative_baseline": 0}, 99),
                              ({**value, "negative_baseline": None}, 99)]:
            with self.assertRaises(runner.Rejected):
                runner.verify_false_check(changed, "a" * 32, code)


class ArtifactTests(unittest.TestCase):
    def test_no_partial_publication_when_any_artifact_fails_sentinel(self):
        for result, provenance in [({"good": True}, {"bad": "CONFIDENTIAL_SENTINEL"}),
                                   ({"bad": "CONFIDENTIAL_SENTINEL"}, {"good": True})]:
            with scratch() as output, self.assertRaises(runner.Rejected):
                try:
                    runner.publish(output, result, provenance)
                finally:
                    self.assertEqual(list(output.iterdir()), [])

    def test_output_allowlist_and_size(self):
        with scratch() as output:
            runner.publish(output, {"version": 1}, {"version": 1})
            self.assertEqual({p.name for p in output.iterdir()}, {"result.json", "provenance.json"})
        with scratch() as output, self.assertRaises(runner.Rejected):
            runner.publish(output, {"large": "x" * 32768})

    def test_summary_secret_is_not_published(self):
        with self.assertRaises(runner.Rejected):
            LaunchTests().launch(lambda x: {**x, "secret": "CONFIDENTIAL_SENTINEL"})


class SelftestTests(unittest.TestCase):
    def exercise(self, unexpected_success=False):
        expected = {"false-check": "TOOL_FAILED", "absent-case": "SUMMARY_INVALID",
                    "missing-summary": "SUMMARY_INVALID", "interrupt": "INTERRUPTED",
                    "missing-input": "INPUT_INVALID", "refused-target": "INPUT_INVALID"}
        with scratch() as output:
            (output / "private").mkdir()
            def create(value):
                path = Path(value)
                path.mkdir()
                (path / "private").mkdir()
                return path
            with mock.patch.object(runner, "live_target"), mock.patch.object(runner, "new_output", side_effect=create), \
                    mock.patch.object(runner, "Child") as child, mock.patch.dict(os.environ, {"K6_TOOLS_DIR": "/tools"}):
                instance = child.return_value
                instance.cleanup = True
                def execute(argv, **kwargs):
                    name = argv[5]
                    instance.stdout = json.dumps({"version": 1, "diagnostic": expected[name], "non_success": True,
                        "cleanup": "NOT_STARTED" if name in {"missing-input", "refused-target"} else "PASS"}).encode()
                    return 0 if unexpected_success and name == "false-check" else 1
                instance.execute.side_effect = execute
                result = runner.selftest({"origin": "http://127.0.0.1:18080"}, output, {}, output / "input.json")
                self.assertEqual(instance.execute.call_count, 6)
                return result

    def test_every_negative_requires_actual_nonzero_launch_exit(self):
        self.assertEqual(len(self.exercise()), 6)
        with self.assertRaises(runner.Rejected) as found:
            self.exercise(unexpected_success=True)
        self.assertEqual(found.exception.code, "SELFTEST_FAILED")

    def test_missing_live_target_cannot_green_aggregate(self):
        with mock.patch.object(runner, "live_target", side_effect=runner.Rejected("TARGET_UNAVAILABLE", True)), \
                mock.patch.object(runner, "Child") as child, self.assertRaises(runner.Rejected):
            runner.selftest({}, Path("/unused"), {}, "/unused/config")
        child.assert_not_called()


class DirectoryPolicyTests(unittest.TestCase):
    """Portable policy oracle with injected stat metadata, not POSIX evidence."""
    def test_private_output_parent_does_not_require_private_grandparent(self):
        with scratch() as parent:
            real_stat = Path.stat
            metadata = {parent: (123, 0o700), parent.parent: (0, 0o1777)}
            def injected_stat(path, *args, **kwargs):
                if path in metadata:
                    uid, mode = metadata[path]
                    return SimpleNamespace(st_uid=uid, st_mode=stat.S_IFDIR | mode)
                return real_stat(path, *args, **kwargs)
            with mock.patch.object(Path, "stat", injected_stat), mock.patch.object(os, "getuid", return_value=123, create=True):
                output = safety.new_output(str(parent / "run"))
            self.assertTrue(output.is_dir())
            self.assertTrue((output / "private/empty.json").is_file())

    def test_immediate_parent_must_still_be_private_and_owned(self):
        for uid, mode in [(123, 0o755), (124, 0o700)]:
            with self.subTest(uid=uid, mode=mode), scratch() as parent:
                real_stat = Path.stat
                def injected_stat(path, *args, **kwargs):
                    if path == parent:
                        return SimpleNamespace(st_uid=uid, st_mode=stat.S_IFDIR | mode)
                    return real_stat(path, *args, **kwargs)
                with mock.patch.object(Path, "stat", injected_stat), mock.patch.object(os, "getuid", return_value=123, create=True), \
                        self.assertRaises(runner.Rejected) as found:
                    safety.new_output(str(parent / "run"))
                self.assertEqual(found.exception.code, "PATH_INVALID")
                self.assertFalse((parent / "run").exists())


@unittest.skipUnless(sys.platform == "linux", "Real POSIX/process checks require Linux; no WSL startup in unit tests")
class PosixTests(unittest.TestCase):
    def test_output_private_parent_below_public_grandparent_and_fail_closed_edges(self):
        with scratch() as root:
            root.chmod(0o700)
            public = root / "public-grandparent"
            public.mkdir(mode=0o777)
            public.chmod(0o777)  # Only the new test-owned directory, never /tmp.
            parent = public / "private-parent"
            parent.mkdir(mode=0o700)
            output = safety.new_output(str(parent / "run"))
            self.assertEqual(output.stat().st_uid, os.getuid())
            self.assertEqual(stat.S_IMODE(output.stat().st_mode), 0o700)
            self.assertEqual(safety.private_read(output / "private/empty.json"), b"{}")
            with self.assertRaises(runner.Rejected) as found:
                safety.new_output(str(public / "refused-public-parent"))
            self.assertEqual(found.exception.code, "PATH_INVALID")
            self.assertFalse((public / "refused-public-parent").exists())
            public_file = public / "private-file-public-parent.json"
            public_file.write_text("{}")
            public_file.chmod(0o400)
            with self.assertRaises(runner.Rejected) as found:
                safety.private_read(public_file)
            self.assertEqual(found.exception.code, "PATH_INVALID")

            linked = root / "linked-parent"
            linked.symlink_to(parent, target_is_directory=True)
            try:
                with self.assertRaises(runner.Rejected) as found:
                    safety.new_output(str(linked / "refused-link"))
                self.assertEqual(found.exception.code, "PATH_INVALID")
            finally:
                linked.unlink()
            for path in [output, output / "private/empty.json"]:
                with self.subTest(path=path.name), self.assertRaises(runner.Rejected) as found:
                    safety.new_output(str(path))
                self.assertEqual(found.exception.code, "PATH_INVALID")
            self.assertEqual((output / "private/empty.json").read_text(), "{}")
            (root / ".git").mkdir()
            with self.assertRaises(runner.Rejected) as found:
                safety.new_output(str(parent / "refused-git-tree"))
            self.assertEqual(found.exception.code, "PATH_INVALID")

    def test_private_modes_outside_git_and_symlink(self):
        with scratch() as root:
            root.chmod(0o700)
            path = root / "input.json"
            path.write_text("{}")
            path.chmod(0o400)
            self.assertEqual(safety.private_read(path), b"{}")
            path.chmod(0o600)
            with self.assertRaises(runner.Rejected):
                safety.private_read(path)
            path.chmod(0o400)
            link = root / "link.json"
            link.symlink_to(path)
            with self.assertRaises(runner.Rejected):
                safety.private_read(link)
            link.unlink()
            (root / ".git").mkdir()
            with self.assertRaises(runner.Rejected):
                safety.private_read(path)

    def supervise(self, source, **kwargs):
        with scratch() as root:
            root.chmod(0o700)
            output = safety.new_output(str(root / "run"))
            child = safety.Child()
            try:
                code = child.execute([sys.executable, "-B", "-c", source], output=output,
                                     env=safety.child_environment(output), timeout=3, **kwargs)
                self.assertTrue(child.cleanup)
                return child, code
            except runner.Rejected:
                self.assertTrue(child.cleanup)
                raise

    def test_actual_process_success_nonzero_and_no_ambient_overrides(self):
        child, code = self.supervise("import os,sys;assert 'HTTPS_PROXY' not in os.environ;sys.exit(7)")
        self.assertEqual(code, 7)
        self.assertEqual(child.stdout, b"")

    def test_native_stderr_sentinel_and_output_flood_rejected(self):
        for source, expected in [("import sys;sys.stderr.write('CONFIDENTIAL_SENTINEL')", "PRIVACY_FAILED"),
                                 ("import sys;sys.stdout.write('x'*70000)", "OUTPUT_LIMIT")]:
            with self.subTest(expected=expected), self.assertRaises(runner.Rejected) as found:
                self.supervise(source)
            self.assertEqual(found.exception.code, expected)

    def test_interruption_reaps_descendants_by_identity(self):
        with self.assertRaises(runner.Rejected) as found:
            self.supervise("import subprocess,sys,time;subprocess.Popen([sys.executable,'-c','import time;time.sleep(30)']);time.sleep(30)", interrupt_after=0.2)
        self.assertEqual(found.exception.code, "INTERRUPTED")

    def test_foreign_preexisting_child_not_stopped(self):
        import subprocess
        foreign = subprocess.Popen([sys.executable, "-c", "import time;time.sleep(30)"])
        try:
            self.supervise("pass")
            self.assertIsNone(foreign.poll())
        finally:
            foreign.terminate()
            foreign.wait(timeout=5)


class SandboxTests(unittest.TestCase):
    def test_renderer_requires_filter_and_both_namespace_boundaries(self):
        parent = {"userns": "u1", "pidns": "p1", "filters": 1}
        good = {"args": [b"--type=renderer"], "uid": 123, "seccomp": "2", "userns": "u2", "pidns": "p2", "filters": 2, "no_new_privs": "1"}
        with mock.patch.object(os, "getuid", return_value=123, create=True):
            self.assertTrue(safety.sandbox_witness(good, parent))
            for key, value in [("seccomp", "0"), ("userns", "u1"), ("pidns", "p1"), ("userns", None), ("pidns", None),
                               ("args", []), ("filters", 1), ("no_new_privs", "0")]:
                self.assertFalse(safety.sandbox_witness({**good, key: value}, parent))
            with self.assertRaises(runner.Rejected):
                safety.sandbox_witness({**good, "args": good["args"] + [b"--no-sandbox"]}, parent)


if __name__ == "__main__":
    unittest.main()
