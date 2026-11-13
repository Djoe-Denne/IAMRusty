"""Baseline Docker trial policy, ownership and no E2E substitution contracts."""
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

CI = Path(__file__).resolve().parents[1] / "ci"
sys.path.insert(0, str(CI))
import binaries as b
import browser_container as c
import yaml


def configuration():
    return dict(Config=dict(User="1001:1001"), AppArmorProfile="docker-default", HostConfig=dict(
        Privileged=False, ReadonlyRootfs=True, NetworkMode="none", PidMode="", CapAdd=None,
        CapDrop=["ALL"], SecurityOpt=["no-new-privileges"]))


class ContainerPolicyTests(unittest.TestCase):
    def test_baseline_policy_retains_both_container_and_browser_protections(self):
        value = c.container_policy(configuration(), 1001, 1001)
        self.assertEqual(value["apparmor"], "docker-default")
        self.assertEqual(value["seccomp"], "docker_default_requested")
        self.assertFalse(value["privileged"])

    def test_created_request_is_distinct_from_the_applied_process_profile(self):
        value = configuration()
        value["AppArmorProfile"] = ""
        self.assertEqual(c.container_policy(value, 1001, 1001, applied=False)["apparmor"], "docker_default_requested")
        with self.assertRaises(b.Refused):
            c.container_policy(value, 1001, 1001, applied=True)
        actual = c.process_security("docker-default (enforce)\n", "Seccomp:\t2\nSeccomp_filters:\t1\nNoNewPrivs:\t1\n")
        self.assertEqual(actual, dict(apparmor="DOCKER_DEFAULT", seccomp_mode=2, filters=1, no_new_privileges=1))
        self.assertEqual(c.process_security("unconfined", "")["apparmor"], "OTHER")
        self.assertIsNone(c.process_security("docker-default", "")["seccomp_mode"])

    def test_root_privileged_extra_caps_host_pid_and_security_overrides_refused(self):
        for key, invalid in (("Privileged", True), ("ReadonlyRootfs", False), ("NetworkMode", "host"),
                             ("PidMode", "host"), ("CapAdd", ["SYS_ADMIN"]), ("CapDrop", []),
                             ("SecurityOpt", ["no-new-privileges", "seccomp=unconfined"]),
                             ("SecurityOpt", ["apparmor=unconfined"])):
            value = configuration()
            value["HostConfig"][key] = invalid
            with self.subTest(key=key, invalid=invalid), self.assertRaises(b.Refused):
                c.container_policy(value, 1001, 1001)
        value = configuration()
        value["Config"]["User"] = "0:0"
        with self.assertRaises(b.Refused):
            c.container_policy(value, 0, 0)
        value = configuration()
        value["AppArmorProfile"] = ""
        with self.assertRaises(b.Refused):
            c.container_policy(value, 1001, 1001)

    def test_create_is_id_pinned_networkless_and_never_exposes_host_daemon(self):
        state = dict(uid=1001, gid=1001, run_id="123", attempt="1", image_id="sha256:" + "a" * 64)
        env = dict(GITHUB_ACTIONS="true", GITHUB_JOB="trial", GITHUB_REF="refs/heads/master",
                   GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="1")
        with patch.dict(os.environ, env):
            args = c.create_arguments(state, Path("/private/trial"))
        self.assertEqual(args[-1], state["image_id"])
        self.assertIn("--read-only", args)
        self.assertEqual(args[args.index("--network") + 1], "none")
        self.assertEqual(args[args.index("--user") + 1], "1001:1001")
        for forbidden in ("--privileged", "SYS_ADMIN", "SYS_CHROOT", "docker.sock", "seccomp=unconfined", "apparmor=unconfined", "--pid=host"):
            self.assertNotIn(forbidden, " ".join(args))


class ReceiptTests(unittest.TestCase):
    def test_exact_creation_receipt_required_no_adoption_by_label(self):
        state = dict(run_id="123", attempt="1")
        row = dict(event="created", kind="container", id="a" * 64, owner=c.OWNER,
                   run_id="123", run_attempt="1", safe_to_delete="no")
        c.validate_receipt([row], state, "a" * 64)
        for change in (dict(event="stopped"), dict(run_id="999"), dict(run_attempt="2"),
                       dict(owner="foreign"), dict(safe_to_delete="ask")):
            with self.subTest(change=change), self.assertRaises(b.Refused):
                c.validate_receipt([{**row, **change}], state, "a" * 64)
        with self.assertRaises(b.Refused):
            c.validate_receipt([row, {**row, "event": "reclassified", "safe_to_delete": "ask"}], state, "a" * 64)


def seccomp_baseline():
    return dict(defaultAction="SCMP_ACT_ERRNO", defaultErrnoRet=1, archMap=[], syscalls=[
        dict(names=["read", "write", "exit"], action="SCMP_ACT_ALLOW"),
        dict(names=["clone3"], action="SCMP_ACT_ERRNO", errnoRet=38),
        dict(names=["chroot"], action="SCMP_ACT_ALLOW", includes=dict(caps=["CAP_SYS_CHROOT"])),
        dict(names=["clone", "setns", "unshare", "mount", "bpf"], action="SCMP_ACT_ALLOW", includes=dict(caps=["CAP_SYS_ADMIN"]))])


class SeccompTests(unittest.TestCase):
    def test_reference_retained_byte_semantics_except_one_exact_namespace_rule(self):
        before = seccomp_baseline()
        original = copy.deepcopy(before)
        after = c.derive_seccomp(before)
        self.assertEqual(before, original)
        self.assertEqual({key: value for key, value in after.items() if key != "syscalls"},
                         {key: value for key, value in before.items() if key != "syscalls"})
        self.assertEqual(after["syscalls"][:-1], before["syscalls"])
        self.assertEqual(after["syscalls"][-1], c.SANDBOX_RULE)
        self.assertEqual(after["syscalls"][-1]["names"], ["clone", "setns", "unshare", "chroot"])
        self.assertEqual(after["defaultAction"], "SCMP_ACT_ERRNO")
        self.assertEqual(after["syscalls"][1]["errnoRet"], 38)
        self.assertEqual(after["syscalls"][2], dict(names=["chroot"], action="SCMP_ACT_ALLOW", includes=dict(caps=["CAP_SYS_CHROOT"])))

    def test_allow_all_or_already_broad_baseline_refused(self):
        for invalid in (dict(defaultAction="SCMP_ACT_ALLOW"), dict(defaultErrnoRet=0), dict(extra="unexpected")):
            with self.assertRaises(b.Refused):
                c.derive_seccomp({**seccomp_baseline(), **invalid})
        value = seccomp_baseline()
        value["syscalls"].append(dict(names=["clone"], action="SCMP_ACT_ALLOW"))
        with self.assertRaises(b.Refused):
            c.derive_seccomp(value)

    def test_inspected_custom_profile_must_equal_verified_derived_profile(self):
        expected = c.derive_seccomp(seccomp_baseline())
        value = configuration()
        value["HostConfig"]["SecurityOpt"].append("seccomp=" + json.dumps(expected))
        policy = c.container_policy(value, 1001, 1001, expected_seccomp=expected)
        self.assertEqual(policy["seccomp"], "pinned_moby_plus_chromium_namespaces")
        for invalid in ("unconfined", json.dumps({**expected, "defaultAction": "SCMP_ACT_ALLOW"})):
            changed = copy.deepcopy(value)
            changed["HostConfig"]["SecurityOpt"][-1] = "seccomp=" + invalid
            with self.assertRaises(b.Refused):
                c.container_policy(changed, 1001, 1001, expected_seccomp=expected)
        with self.assertRaises(b.Refused):
            c.container_policy(value, 1001, 1001)

    def test_acquisition_is_hashed_closed_and_never_reuses_existing_profile(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / "private").mkdir()
            raw = json.dumps(seccomp_baseline()).encode()
            revision = "a" * 40
            url = "https://raw.githubusercontent.com/moby/profiles/" + revision + "/seccomp/default.json"
            pin = dict(schema="aiforall.chromium-seccomp/v1", approved_namespace_syscalls=["clone", "setns", "unshare"],
                       approved_filesystem_syscalls=["chroot"],
                       revision=revision, source=url, sha256=hashlib.sha256(raw).hexdigest())
            (root / "seccomp.lock.json").write_text(json.dumps(pin))

            def response(_url, timeout):
                stream = io.BytesIO(raw)
                stream.url = url
                return stream

            with patch.object(c, "HERE", root), patch.object(c.urllib.request, "build_opener") as opener:
                opener.return_value.open.side_effect = response
                path, derived, facts = c.acquire_seccomp(root)
                self.assertEqual(derived, c.derive_seccomp(seccomp_baseline()))
                self.assertEqual(facts["derived_sha256"], b.sha(path))
                self.assertEqual(facts["added_syscalls"], ["clone", "setns", "unshare", "chroot"])
                with self.assertRaises(b.Refused):
                    c.acquire_seccomp(root)

    def test_chromium_mode_checks_the_profile_hash_before_create(self):
        env = dict(GITHUB_ACTIONS="true", GITHUB_JOB="trial", GITHUB_REF="refs/heads/master", GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="1")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            path = root / "profile.json"
            path.write_text(json.dumps(c.derive_seccomp(seccomp_baseline())))
            state = dict(uid=1001, gid=1001, run_id="123", attempt="1", image_id="sha256:" + "a" * 64,
                         policy="chromium", seccomp_path=str(path), seccomp_sha256=b.sha(path))
            with patch.dict(os.environ, env):
                args = c.create_arguments(state, root)
                self.assertIn("seccomp=" + str(path), args)
                path.write_text("tampered")
                with self.assertRaises(b.Refused):
                    c.create_arguments(state, root)

    def test_pinned_source_is_immutable_and_workflow_mode_is_explicit(self):
        pin = json.loads((CI / "seccomp.lock.json").read_text())
        self.assertRegex(pin["source"], r"https://raw.githubusercontent.com/moby/profiles/[a-f0-9]{40}/seccomp/default.json$")
        self.assertRegex(pin["sha256"], r"^[a-f0-9]{64}$")
        value = yaml.load((b.ROOT / ".github/workflows/k6-browser-container.yml").read_text(), Loader=yaml.BaseLoader)
        mode = value["on"]["workflow_dispatch"]["inputs"]["seccomp_mode"]
        self.assertEqual(mode["default"], "baseline")
        self.assertEqual(mode["options"], ["baseline", "chromium"])


class ProbeDiagnosticTests(unittest.TestCase):
    def test_stage_summary_is_closed_and_never_reinterpreted_as_e2e(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "stages.json"
            self.assertEqual(c.probe_progress(path), {"state": "ABSENT"})
            stages = dict(started=1, context=1, version=1, page=0, evaluation=0)
            path.write_text(json.dumps(stages))
            self.assertEqual(c.probe_progress(path), dict(state="AVAILABLE", counts=stages))
            for invalid in ({**stages, "private": "SENTINEL"}, {**stages, "started": True}, {**stages, "page": 2}):
                path.write_text(json.dumps(invalid))
                self.assertEqual(c.probe_progress(path), {"state": "WITHHELD"})

    def test_direct_raw_fatal_distinguishes_chroot_and_display_without_text(self):
        cases = [
            (b'Check failed: sys_chroot("/proc/self/fdinfo/") == 0 private=SENTINEL', "FILESYSTEM_SANDBOX_FAILURE"),
            (b'Missing X server or $DISPLAY private=SENTINEL', "DISPLAY_SERVER_MISSING"),
            (b'EGL_NOT_INITIALIZED private=SENTINEL', "GRAPHICS_INIT_FAILURE"),
            (b'libGL error: failed to load driver: swrast private=SENTINEL', "GRAPHICS_INIT_FAILURE"),
        ]
        for raw, expected in cases:
            result = c.browser_profile.native_diagnostic(raw)
            self.assertEqual(result, dict(log="CLASSIFIED", causes=[expected]))
            self.assertNotIn("SENTINEL", json.dumps(result))
        self.assertEqual(c.browser_profile.native_diagnostic(b"\xff")["log"], "WITHHELD")

    def test_failed_probe_native_diagnostic_cannot_promote_verdict(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            # Only the diagnostic result is tested; it contains no PASS field.
            with patch.object(c, "new_output", return_value=root), patch.object(c, "Child") as factory:
                child = factory.return_value
                child.execute.side_effect = c.Rejected("CHILD_TIMEOUT")
                child.code = 0
                child.cleanup = True
                child.stderr = b""
                result = c.diagnose_native(root, dict(chromium=dict(path="/private/chrome")))
                self.assertEqual(result["qualification"], "DIAGNOSTIC_ONLY_NOT_A_PROBE_PASS")
                self.assertNotIn("verdict", result)
                self.assertTrue(result["cleanup"])


class WorkflowTests(unittest.TestCase):
    def test_closed_context_packages_only_public_verified_tools_and_harness(self):
        dockerfile = (CI / "Dockerfile.browser-trial").read_text()
        self.assertIn("FROM ${RUNTIME_IMAGE} AS runtime", dockerfile)
        self.assertIn("USER 1000:1000", dockerfile)
        self.assertIn("COPY --chown=0:0 browser/", dockerfile)
        self.assertIn("chmod -R a+rX", dockerfile)
        for forbidden in ("cargo ", "COPY . ", "4755", "SYS_ADMIN", "no-sandbox"):
            self.assertNotIn(forbidden, dockerfile)
        self.assertTrue((CI / "Dockerfile.browser-trial.dockerignore").read_text().startswith("**\n"))
        self.assertEqual(len(c.FILES), 7)
        for name in c.FILES:
            self.assertTrue((CI.parent / name).is_file())

    def test_manual_workflow_always_closes_and_only_uploads_closed_results(self):
        value = yaml.load((b.ROOT / ".github/workflows/k6-browser-container.yml").read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(set(value["on"]), {"workflow_dispatch"})
        self.assertEqual(value["permissions"], dict(contents="read"))
        steps = value["jobs"]["trial"]["steps"]
        for step in steps:
            if "uses" in step:
                self.assertRegex(step["uses"], r"@[a-f0-9]{40}$")
            for forbidden in ("cargo ", "sysctl", "apparmor_parser", "${{"):
                self.assertNotIn(forbidden, step.get("run", ""))
        self.assertEqual(steps[-2]["if"], "${{ always() }}")
        self.assertEqual(steps[-1]["if"], "${{ always() }}")
        for path in steps[-1]["with"]["path"].splitlines():
            self.assertTrue(path.endswith(("public/result.json", "public/cleanup.json", "public/docker-resources.jsonl")))

    def test_probe_cannot_be_mistaken_for_the_business_scenarios(self):
        probe = (CI / "browser-sandbox-probe.js").read_text()
        self.assertNotIn(".goto(", probe)
        self.assertNotIn("http.get(", probe)
        self.assertIn("context.newPage()", probe)
        self.assertIn("page.evaluate", probe)
        self.assertIn("setTimeout(resolve, 1500)", probe)
        source = (CI / "browser_container.py").read_text()
        self.assertIn('"NOT_E2E_SANDBOX_TRIAL"', source)
        self.assertIn("child.sandbox is True", source)
        self.assertIn("child.renderer_observed is True", source)


if __name__ == "__main__":
    unittest.main()
