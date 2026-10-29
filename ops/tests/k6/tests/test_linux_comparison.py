"""Pure control-flow fixtures, never evidence of Ubuntu or Chromium qualification."""
import contextlib
import io
import json
import os
from pathlib import Path
import unittest
from unittest import mock

from test_boundaries import scratch
from test_direct_chromium import fake_child_reads, host
from test_runner import runner, summary
import safety


FLAG = "--compare-ubuntu-2204"
LOCK = json.loads((runner.HERE / "config/tools.lock.json").read_text())


@contextlib.contextmanager
def environment(version="22.04", *, system="linux", machine="x86_64", uid=123, distro="ubuntu"):
    with mock.patch.object(runner.sys, "platform", system), \
            mock.patch.object(runner.platform, "machine", return_value=machine), \
            mock.patch.object(runner.os, "getuid", return_value=uid, create=True), \
            mock.patch.object(Path, "read_text", return_value=f'ID={distro}\nVERSION_ID="{version}"\n'):
        yield


class SelectionTests(unittest.TestCase):
    def test_nominal_platform_guard_remains_ubuntu24(self):
        with environment("24.04"):
            runner.linux_platform()

    def test_only_browser_has_closed_comparison_flag(self):
        self.assertTrue(runner.arguments(["browser", FLAG, "--output", "/private/new"]).compare_ubuntu_2204)
        for mode in ("validate", "run", "selftest", "tools", "diagnose-chromium"):
            args = [mode, FLAG, "--output", "/private/new", "--config", "/private/config"]
            with self.subTest(mode=mode), self.assertRaises(safety.Rejected):
                runner.arguments(args)

    def test_nominal_ubuntu22_refused_before_any_child(self):
        for mode in ("validate", "run", "selftest", "tools", "browser", "diagnose-chromium"):
            argv = [mode]
            if mode in {"validate", "run", "selftest"}:
                argv += ["--config", "/private/config"]
            if mode != "validate":
                argv += ["--output", "/private/new"]
            with environment(), mock.patch.object(runner, "Child") as child, \
                    contextlib.redirect_stdout(io.StringIO()) as log:
                self.assertNotEqual(runner.main(argv), 0)
            self.assertEqual(json.loads(log.getvalue())["diagnostic"], "PLATFORM_UNQUALIFIED")
            child.assert_not_called()

    def test_comparison_refuses_other_platforms_before_paths_or_children(self):
        variants = [{"version": "24.04"}, {"version": "26.04"}, {"version": "unknown"},
                    {"system": "win32"}, {"machine": "aarch64"}, {"machine": "unknown"},
                    {"uid": 0}, {"distro": "debian"}]
        for values in variants:
            with self.subTest(values=values), environment(**values), \
                    mock.patch.object(runner, "new_output") as output, \
                    mock.patch.object(runner, "Child") as child, \
                    contextlib.redirect_stdout(io.StringIO()) as log:
                self.assertNotEqual(runner.main(["browser", FLAG, "--output", "/private/new"]), 0)
            self.assertEqual(json.loads(log.getvalue())["diagnostic"], "PLATFORM_UNQUALIFIED")
            output.assert_not_called()
            child.assert_not_called()


class ComparisonTests(unittest.TestCase):
    def launch(self, *, stage_error=None, transform=None, browser_error=None, forged_versions=None,
               forged_host=None, image="ubuntu22", output_bytes=b"", real_versions=False, inspect_transform=None):
        events = []
        tools = {"k6": Path("/pinned/k6"), "chromium": Path("/pinned/chrome")}
        versions = {name: LOCK[name]["version"] for name in tools}
        if forged_versions is not None:
            versions = forged_versions

        def verify(value, names):
            self.assertEqual(value, tools)
            events.append(("identity", names))

        def child_factory():
            child = safety.Child()
            child.cleanup, child.sandbox = True, True
            child.renderer_observed = True
            child.namespace_reads = {"user": {"READABLE"}, "pid": {"READABLE"}}
            child.sandbox_proof = [{"pid": 1234, "uid": 123, "start": 5678}]

            def execute(argv, **kwargs):
                if argv[-1] in {"version", "--version"}:
                    name = "k6" if argv[-1] == "version" else "chromium"
                    events.append(("version", name))
                    text = "k6 v" if name == "k6" else "Google Chrome for Testing "
                    child.stdout = (text + ("0.0.0" if stage_error == name + "-version" else LOCK[name]["version"])).encode()
                elif "inspect" in argv:
                    phase = next(key for key, path in runner.SCENARIOS.items() if str(path) == argv[-1])
                    events.append(("inspect", phase))
                    self.assertFalse(kwargs.get("browser", False))
                    self.assertEqual(kwargs["timeout"], 10)
                    if stage_error == phase:
                        raise safety.Rejected("TOOL_FAILED")
                    data = {"scenarios": {runner.CASES[phase]: {
                        "executor": "per-vu-iterations", "vus": 1, "iterations": 1, "maxDuration": "15s"}}}
                    child.stdout = json.dumps(inspect_transform(data) if inspect_transform else data).encode()
                else:
                    events.append(("browser", "Chromium"))
                    self.assertTrue(kwargs["browser"])
                    self.assertEqual(kwargs["timeout"], 30)
                    self.assertEqual(argv[-1], str(runner.SCENARIOS["Chromium"]))
                    self.assertEqual(kwargs["env"], {**safety.child_environment(kwargs["output"]),
                        "K6_BROWSER_EXECUTABLE_PATH": str(tools["chromium"])})
                    child.stdout = output_bytes
                    if browser_error:
                        child.cleanup = browser_error != "CLEANUP_FAILED"
                        raise safety.Rejected(browser_error, browser_error == "SANDBOX_UNPROVEN")
                    safety.confidential(child.stdout)
                    params = dict(arg.split("=", 1) for i, arg in enumerate(argv) if i and argv[i-1] == "--env")
                    data = {**summary("Chromium"), "run_id": params["K6_RUN_ID"]}
                    if transform:
                        data = transform(data)
                    if data is not None:
                        Path(params["K6_SUMMARY_PATH"]).write_bytes(
                            data if isinstance(data, bytes) else json.dumps(data).encode())
                child.code = 0
                return 0
            child.execute = execute
            return child

        def pinned(**kwargs):
            events.append(("pins", kwargs))
            if stage_error == "pins":
                raise safety.Rejected("TOOL_UNPINNED", True)
            return tools, LOCK

        original_versions = runner.version_checks

        def check_versions(*args):
            events.append(("versions", None))
            if stage_error == "versions":
                raise safety.Rejected("TOOL_VERSION")
            return original_versions(*args) if real_versions else versions

        with scratch() as root:
            (root / "private").mkdir()
            with mock.patch.object(runner, "new_output", return_value=root), \
                    mock.patch.object(runner, "pinned_tools", side_effect=pinned), \
                    mock.patch.object(runner, "version_checks", side_effect=check_versions), \
                    mock.patch.object(runner, "verify_tools", side_effect=verify), \
                    mock.patch.object(runner, "digest", return_value="a" * 64), \
                    mock.patch.object(runner, "host_metadata", return_value=host() if forged_host is None else forged_host), \
                    mock.patch.object(runner, "private_read", lambda p: Path(p).read_bytes()), \
                    mock.patch.object(runner, "Child", side_effect=child_factory), \
                    mock.patch.object(runner, "publish") as publish, \
                    mock.patch.object(runner, "live_target") as target, \
                    mock.patch.object(runner, "direct_chromium") as direct, \
                    mock.patch.dict(os.environ, {"ImageOS": image, "ImageVersion": "20261025.1",
                        "K6_BROWSER_ARGS": "CONFIDENTIAL_SENTINEL", "HTTPS_PROXY": "https://CONFIDENTIAL_SENTINEL"}), \
                    environment(), contextlib.redirect_stdout(io.StringIO()) as log, \
                    contextlib.redirect_stderr(io.StringIO()) as err:
                code = runner.main(["browser", FLAG, "--output", str(root)])
            publish.assert_not_called()
            target.assert_not_called()
            direct.assert_not_called()
            self.assertFalse((root / "result.json").exists())
            self.assertFalse((root / "provenance.json").exists())
            raw = log.getvalue()
            self.assertLessEqual(len(raw.encode()), safety.ARTIFACT_LIMIT)
            self.assertNotIn("CONFIDENTIAL_SENTINEL", raw + err.getvalue())
            self.assertNotEqual(code, 0)
            result = json.loads(raw)
            self.assertEqual(result["HTTP"], "INCONCLUSIVE")
            self.assertEqual(result["Chromium"], "INCONCLUSIVE")
            self.assertEqual(result["parent_runtime_cleanup"], "INCONCLUSIVE")
            return code, result, events

    def test_positive_case_stays_nonqualifying_nonpublished_with_facts_separate(self):
        code, result, events = self.launch()
        self.assertEqual(code, 2)
        self.assertEqual([e[0] for e in events if e[0] != "identity"],
                         ["pins", "versions", "inspect", "inspect", "browser"])
        self.assertEqual([e[1] for e in events if e[0] == "inspect"], ["HTTP", "Chromium"])
        self.assertEqual([e[1] for e in events if e[0] == "identity"],
                         [("k6",), ("k6",), ("k6", "chromium")])
        comparison = result["comparison"]
        self.assertEqual(comparison["platform"], "ubuntu-22.04-linux-amd64-diagnostic-only")
        self.assertEqual(comparison["qualification"], "NOT_EQUIVALENT_TO_UBUNTU_24_04")
        self.assertEqual(comparison["browser_launch"], "ATTEMPTED")
        self.assertEqual(comparison["case"], "COMPLETE")
        self.assertEqual(comparison["sandbox_witness"], "OBSERVED")
        self.assertEqual(comparison["observed"]["uid"], 123)
        self.assertEqual(comparison["versions"], {name: LOCK[name]["version"] for name in ("k6", "chromium")})
        self.assertEqual(comparison["tools"], {"k6": "a" * 64, "chromium": "a" * 64})
        self.assertEqual(result["cleanup"], "PASS")
        self.assertEqual(result["confidentiality"], "PASS")

    def test_failed_prerequisite_stops_before_browser(self):
        for stage in ("pins", "versions", "HTTP", "Chromium"):
            with self.subTest(stage=stage):
                _, result, events = self.launch(stage_error=stage)
                self.assertNotIn(("browser", "Chromium"), events)
                self.assertNotEqual(result.get("tools"), "PASS")

    def test_real_version_checks_precede_both_inspections_and_browser(self):
        _, _, events = self.launch(real_versions=True)
        self.assertEqual([e for e in events if e[0] != "identity"], [
            ("pins", {"browser": True}), ("versions", None), ("version", "k6"), ("version", "chromium"),
            ("inspect", "HTTP"), ("inspect", "Chromium"), ("browser", "Chromium")])
        for name in ("k6", "chromium"):
            _, result, events = self.launch(real_versions=True, stage_error=name + "-version")
            self.assertEqual(result["diagnostic"], "TOOL_VERSION")
            self.assertFalse(any(e[0] in {"inspect", "browser"} for e in events))

    def test_invalid_inspection_never_enters_browser(self):
        _, result, events = self.launch(inspect_transform=lambda _: {"scenarios": {}})
        self.assertEqual(result["diagnostic"], "INPUT_INVALID")
        self.assertNotIn(("browser", "Chromium"), events)

    def test_missing_stale_truncated_zero_case_and_sentinel_summary_cannot_complete(self):
        transforms = [lambda _: None, lambda _: b"{", lambda x: {**x, "run_id": "b" * 32},
                      lambda x: {**x, "started": 0}, lambda x: {**x, "checks": {}},
                      lambda _: b"CONFIDENTIAL_SENTINEL"]
        for transform in transforms:
            with self.subTest(transform=transform):
                _, result, _ = self.launch(transform=transform)
                self.assertNotEqual(result["comparison"]["case"], "COMPLETE")

    def test_cleanup_privacy_interruption_and_missing_witness_remain_blocking(self):
        for error in ("CLEANUP_FAILED", "PRIVACY_FAILED", "INTERRUPTED", "SANDBOX_UNPROVEN", "OUTPUT_LIMIT"):
            with self.subTest(error=error):
                _, result, events = self.launch(browser_error=error)
                self.assertEqual(result["diagnostic"], error)
                self.assertEqual(events.count(("browser", "Chromium")), 1)
                self.assertEqual(result["cleanup"], "FAIL" if error == "CLEANUP_FAILED" else "PASS")
                self.assertNotEqual(result["comparison"]["sandbox_witness"], "OBSERVED")
                if error == "PRIVACY_FAILED":
                    self.assertEqual(result["confidentiality"], "FAIL")

    def test_sentinels_and_forged_metadata_are_not_projected_or_launched(self):
        cases = [{"forged_versions": {"k6": "CONFIDENTIAL_SENTINEL", "chromium": LOCK["chromium"]["version"]}},
                 {"forged_host": {**host(), "parent_seccomp": "CONFIDENTIAL_SENTINEL"}}]
        for kwargs in cases:
            with self.subTest(kwargs=kwargs):
                _, _, events = self.launch(**kwargs)
                self.assertNotIn(("browser", "Chromium"), events)
        _, result, _ = self.launch(image="https://CONFIDENTIAL_SENTINEL")
        self.assertEqual(result["comparison"]["observed"]["runner_image"], "not-projectable")
        _, result, events = self.launch(image="CONFIDENTIAL_SENTINEL")
        self.assertEqual(result["confidentiality"], "FAIL")
        self.assertNotIn(("browser", "Chromium"), events)
        _, result, _ = self.launch(output_bytes=b"CONFIDENTIAL_SENTINEL")
        self.assertEqual(result["confidentiality"], "FAIL")

    def test_real_child_reader_privacy_and_cleanup_are_not_bypassed(self):
        for cleanup, payload in ((False, b""), (True, b""),
                                 (False, b"CONFIDENTIAL_SENTINEL"), (True, b"CONFIDENTIAL_SENTINEL")):
            with scratch() as root:
                (root / "private").mkdir()
                with fake_child_reads(payload, 1, cleanup) as child, \
                        environment(uid=789), mock.patch.object(runner, "new_output", return_value=root), \
                        mock.patch.object(runner, "pinned_tools", return_value=(
                            {"k6": Path("/pinned/k6"), "chromium": Path("/pinned/chrome")}, LOCK)), \
                        mock.patch.object(runner, "version_checks", return_value={name: LOCK[name]["version"] for name in ("k6", "chromium")}), \
                        mock.patch.object(runner, "host_metadata", return_value=host()), \
                        mock.patch.object(runner, "digest", return_value="a" * 64), \
                        mock.patch.object(runner, "verify_tools"), mock.patch.object(runner, "inspect_scenario"), \
                        mock.patch.object(runner, "Child", return_value=child), \
                        mock.patch.object(runner, "publish") as publish, \
                        contextlib.redirect_stdout(io.StringIO()) as log:
                    code = runner.main(["browser", FLAG, "--output", str(root)])
                result = json.loads(log.getvalue())
                self.assertNotEqual(code, 0)
                self.assertEqual(result["Chromium"], "INCONCLUSIVE")
                self.assertEqual(result["cleanup"], "PASS" if cleanup else "FAIL")
                self.assertEqual(result["diagnostic"], "PRIVACY_FAILED" if payload else
                                 "SANDBOX_UNPROVEN" if cleanup else "CLEANUP_FAILED")
                self.assertNotIn("CONFIDENTIAL_SENTINEL", log.getvalue())
                self.assertNotEqual(result["comparison"]["case"], "COMPLETE")
                publish.assert_not_called()

    def test_workflow_retains_canonical_job_and_minimal_nonpublishing_comparison(self):
        workflow = (runner.REPO / ".github/workflows/k6-functional-smoke.yml").read_text()
        self.assertIn("runs-on: ubuntu-24.04", workflow)
        diagnostic = workflow.split("  ubuntu2204-comparison:", 1)[1]
        self.assertIn("runs-on: ubuntu-22.04", diagnostic)
        self.assertEqual(diagnostic.count("run-linux.py browser " + FLAG), 1)
        self.assertIn("provision-linux.py", diagnostic)
        self.assertIn("persist-credentials: false", diagnostic)
        self.assertIn("umask 077", diagnostic)
        self.assertIn('mktemp -d "$RUNNER_TEMP/k6-private.XXXXXXXX"', diagnostic)
        for forbidden in ("upload-artifact", "continue-on-error", "diagnose-chromium", "sudo", "secrets.",
                          "workflow_dispatch", "apt-get", "sysctl", "apparmor_parser"):
            self.assertNotIn(forbidden, diagnostic)
        self.assertEqual(diagnostic.count("uses:"), 1)
        self.assertIn("actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683", diagnostic)
