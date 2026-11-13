"""Docker OAuth launcher wiring, strict reception and ownership boundaries."""
import copy
import json
import os
from pathlib import Path
import socket
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

CI = Path(__file__).resolve().parents[1] / "ci"
sys.path.insert(0, str(CI))
import binaries as b
import campaign
import campaign_browser as c
from test_ci_browser_container import configuration, seccomp_baseline

RUN = "a" * 32
CID = "b" * 64


def result():
    return dict(qualification="OAUTH_BROWSER_EXECUTION", run_id=RUN, exit=0, cleanup=True, sandbox=True,
        renderer_observed=True, diagnostic=None,
        process_security=dict(apparmor="DOCKER_DEFAULT", seccomp_mode=2, filters=1, no_new_privileges=1),
        witnesses=[dict(pid=102, uid=1001, start_ticks=100, seccomp_mode=2, filters=2, additional_filter=True,
            user_namespace_isolated=True, pid_namespace_isolated=True, no_new_privileges=True, forbidden_flags_absent=True)])


def runtime(root):
    return SimpleNamespace(directory=root, save=Mock(), docker=Mock(return_value=b""), inspect=Mock(), container_ids=Mock(return_value=set()),
        state=dict(run_id=RUN, job_run_id="123", job_attempt="1", host=socket.gethostname(), boot_id="boot",
            browser_container=dict(uid=1001, gid=1001, before=[], image_id="sha256:" + "c" * 64, cleanup="NOT_RUN")))


class ResultTests(unittest.TestCase):
    def test_actual_witness_and_strict_source_identity_required(self):
        self.assertEqual(c.accept_result(json.dumps(result()).encode(), RUN, 1001), result())
        for change in (dict(run_id="d" * 32), dict(qualification="NOT_E2E_SANDBOX_TRIAL"), dict(extra="SENTINEL"),
                       dict(witnesses=[]), dict(cleanup=1), dict(diagnostic="SENTINEL"), dict(exit=True)):
            with self.subTest(change=change), self.assertRaises(b.Refused):
                c.accept_result(json.dumps({**result(), **change}).encode(), RUN, 1001)

    def test_each_isolation_fact_is_mandatory_and_no_duplicate_witness(self):
        for field, invalid in (("uid", 0), ("seccomp_mode", 0), ("filters", 1), ("filters", True),
                               ("user_namespace_isolated", False), ("pid_namespace_isolated", False),
                               ("no_new_privileges", False), ("forbidden_flags_absent", False)):
            value = result()
            value["witnesses"][0][field] = invalid
            with self.subTest(field=field), self.assertRaises(b.Refused):
                c.accept_result(json.dumps(value).encode(), RUN, 1001)
        value = result()
        value["witnesses"] *= 2
        with self.assertRaises(b.Refused):
            c.accept_result(json.dumps(value).encode(), RUN, 1001)
        value = result()
        value["process_security"]["apparmor"] = "unconfined"
        with self.assertRaises(b.Refused):
            c.accept_result(json.dumps(value).encode(), RUN, 1001)


class LaunchTests(unittest.TestCase):
    def test_mount_inspection_allows_only_two_exact_binds_and_private_tmpfs(self):
        output, inputs = Path("/private/case"), Path("/private/input")
        mounts = [dict(Type="bind", Source=str(output), Destination=str(output), RW=True),
                  dict(Type="bind", Source=str(inputs), Destination="/run/campaign-input", RW=False)]
        c.validate_mounts(mounts, output, inputs)
        c.validate_mounts(mounts + [dict(Type="tmpfs", Destination="/tmp", Source="", RW=True)], output, inputs)
        for extra in (dict(Type="volume", Source="foreign", Destination="/data", RW=True),
                      dict(Type="bind", Source="/var/run/docker.sock", Destination="/var/run/docker.sock", RW=True),
                      dict(Type="tmpfs", Destination="/run/campaign-input", Source="", RW=True)):
            with self.assertRaises(b.Refused):
                c.validate_mounts(mounts + [extra], output, inputs)
        changed = copy.deepcopy(mounts)
        changed[1]["RW"] = True
        with self.assertRaises(b.Refused):
            c.validate_mounts(changed, output, inputs)

    def test_host_network_is_explicit_and_all_other_policies_stay_closed(self):
        profile = c.trial.derive_seccomp(seccomp_baseline())
        value = configuration()
        value["HostConfig"]["NetworkMode"] = "host"
        value["HostConfig"]["SecurityOpt"].append("seccomp=" + json.dumps(profile))
        with self.assertRaises(b.Refused):
            c.trial.container_policy(value, 1001, 1001, expected_seccomp=profile)
        self.assertEqual(c.trial.container_policy(value, 1001, 1001,
            expected_seccomp=profile, expected_network="host")["network"], "host")
        for key, invalid in (("PidMode", "host"), ("CapAdd", ["SYS_ADMIN"]), ("ReadonlyRootfs", False), ("Privileged", True)):
            changed = copy.deepcopy(value)
            changed["HostConfig"][key] = invalid
            with self.assertRaises(b.Refused):
                c.trial.container_policy(changed, 1001, 1001, expected_seccomp=profile, expected_network="host")

    def test_arguments_mount_only_case_and_closed_inputs_never_host_secrets(self):
        env = dict(GITHUB_ACTIONS="true", GITHUB_JOB="e2e", GITHUB_REF="refs/heads/master", GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="1")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            r = runtime(root)
            profile = root / "seccomp.json"
            profile.write_text(json.dumps(c.trial.derive_seccomp(seccomp_baseline())))
            r.state["browser_container"].update(profile=str(profile), profile_sha256=b.sha(profile))
            with patch.dict(os.environ, {**env, "GH_TOKEN": "SECRET_SENTINEL", "HTTPS_PROXY": "SECRET_SENTINEL"}):
                args = c.create_arguments(r, root / "case", root / "input")
            rendered = " ".join(args)
            self.assertEqual(args[args.index("--network") + 1], "host")
            self.assertEqual(args.count("--mount"), 2)
            self.assertIn("dst=/run/campaign-input,readonly", rendered)
            self.assertIn("--read-only", args)
            self.assertIn("--cap-drop ALL", rendered)
            self.assertNotIn("--pid", args)
            for forbidden in ("--privileged", "SYS_ADMIN", "SYS_CHROOT", "docker.sock", "kubeconfig", "SECRET_SENTINEL", "--pid=host", "--no-sandbox"):
                self.assertNotIn(forbidden, rendered)
            profile.write_text("changed")
            with patch.dict(os.environ, env), self.assertRaises(b.Refused):
                c.create_arguments(r, root / "case", root / "input")

    def test_context_contains_real_scenario_and_imports_not_an_application_build(self):
        for name in c.ADDITIONAL_FILES:
            self.assertTrue((CI.parent / name).is_file(), name)
        self.assertIn("scenarios/auth/oauth-browser.js", c.ADDITIONAL_FILES)
        self.assertIn("lib/summary.js", c.ADDITIONAL_FILES)
        self.assertIn("lib/campaign-target.js", c.ADDITIONAL_FILES)

    def test_container_executes_real_oauth_with_closed_environment_and_same_origins(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            (output / "private").mkdir()
            child = Mock(code=0, cleanup=True, sandbox=True, renderer_observed=True, sandbox_proof=result()["witnesses"])
            child.execute.return_value = 0
            pins = {name: dict(path="/opt/browser/" + name, version="pinned") for name in ("k6", "chromium")}
            with patch.object(c, "hosted"), patch.object(c, "private_path"), patch.object(Path, "is_file", return_value=True), \
                 patch.object(Path, "read_text", return_value=""), patch.object(c.browser_profile, "tools", return_value=pins), \
                 patch.object(c.trial, "process_security", return_value=result()["process_security"]), patch.object(c, "Child", return_value=child), \
                 patch.dict(os.environ, {"GH_TOKEN": "SENTINEL", "HTTPS_PROXY": "SENTINEL"}):
                self.assertEqual(c.in_container(output, RUN), 0)
            args, kwargs = child.execute.call_args
            self.assertTrue(Path(args[0][-1]).as_posix().endswith("scenarios/auth/oauth-browser.js"))
            self.assertNotIn("browser-sandbox-probe.js", " ".join(args[0]))
            self.assertEqual(kwargs["env"]["K6_ORIGIN"], "https://127.0.0.1:18443")
            self.assertEqual(Path(kwargs["env"]["K6_CAMPAIGN_FILE"]).as_posix(), "/run/campaign-input/target.json")
            self.assertNotIn("SENTINEL", json.dumps(kwargs["env"]))
            self.assertTrue(kwargs["browser"])
            stored = json.loads((output / "private/browser-result.json").read_text())
            self.assertEqual(c.accept_result(json.dumps(stored).encode(), RUN, 1001), stored)

    def test_campaign_invokes_docker_and_still_receives_all_26_checks(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "private").mkdir()
            output = root / "private/auth-oauth-browser"
            (output / "private").mkdir(parents=True)
            summary = dict(version=1, run_id=RUN, phase="Chromium", cases=["auth-oauth-browser"], expected=1,
                           started=1, completed=1, iterations=1, requests=0,
                           checks={name: dict(passes=1, fails=0) for name in campaign.evidence.OAUTH_CHECKS})
            (output / "private/summary.json").write_text(json.dumps(summary))
            r = runtime(root)
            r.cli = Mock()
            r.state.update(tools={role: dict(path="/unexecuted/" + role, sha256="hash", version="pinned") for role in ("k6", "chromium")},
                           completed_cases=[], process_evidence={})
            parent = campaign.Campaign(r)
            parent.publish = Mock()
            with patch.object(campaign, "new_output", return_value=output), patch.object(campaign, "sha", return_value="hash"), \
                 patch.object(campaign.browser_profile, "trust"), patch.object(c, "execute", return_value=result()) as launch, \
                 patch.object(campaign.Child, "execute", side_effect=AssertionError("native browser forbidden")):
                parent.scenario("auth-oauth-browser")
                launch.assert_called_once_with(r, output)
            self.assertEqual(r.state["completed_cases"], ["auth-oauth-browser"])
            self.assertTrue(r.state["process_evidence"]["auth-oauth-browser"]["sandbox"])
            parent.publish.assert_any_call("auth-oauth-browser.json", summary)


class CleanupTests(unittest.TestCase):
    def test_browser_cleanup_failure_blocks_parent_even_if_kind_cleanup_passes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "private").mkdir()
            (root / "public").mkdir()
            r = runtime(root)
            r.state.update(verdict="PASS", restoration="NOT_REQUIRED", process_evidence={}, completed_cases=[], tools={},
                images=dict(target_sha="t", sdk_sha="s", harness_sha="h", roles={}))
            r.close = Mock(return_value=dict(status="PASS", owned_running=0))
            r.receipts = Mock(return_value=[])
            parent = campaign.Campaign(r)
            parent.restore = Mock()
            parent.publish = Mock()
            with patch.object(c, "clean", side_effect=b.Refused("E2E_BROWSER_STOP_UNPROVEN")), \
                 patch.object(c, "browser_receipts", return_value=[]), patch.dict(os.environ, {"GITHUB_SHA": "h"}):
                campaign.finish(parent)
            self.assertEqual(r.state["verdict"], "FAIL")
            self.assertEqual(r.state["cleanup"]["status"], "FAIL")
            self.assertIn("BROWSER_CONTAINER_CLEANUP_UNPROVEN", r.state["cleanup"]["errors"])

    def test_received_container_only_is_stopped_and_removed_then_cleanup_is_idempotent(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            r = runtime(root)
            r.state["browser_container"]["container_id"] = CID
            rows = [dict(kind="container", event="created", id=CID, owner=c.OWNER,
                         run_id="123", run_attempt="1", campaign=RUN, safe_to_delete="no")]
            r.container_ids.side_effect = [{CID}, set(), set(), set()]
            r.inspect.side_effect = [dict(Id=CID, Image=r.state["browser_container"]["image_id"], State=dict(Running=True),
                Config=dict(Labels={"aiforall.dev/oauth-browser-campaign": RUN, "aiforall.dev/oauth-browser-run": "123",
                                    "aiforall.dev/oauth-browser-attempt": "1"})), dict(State=dict(Running=False))]
            with patch.object(c, "hosted"), patch.object(Path, "read_text", return_value="boot"), \
                 patch.object(c, "ROOT", root), patch.object(c, "browser_receipts", return_value=rows), patch.object(c, "record") as record:
                c.clean(r)
                c.clean(r)
                r.docker.assert_any_call(["stop", "--time", "20", CID], timeout=30)
                r.docker.assert_any_call(["rm", CID])
                self.assertEqual([call.args[1] for call in record.call_args_list], ["stopped", "reclassified", "removed"])
            self.assertEqual(r.state["browser_container"]["cleanup"], "PASS")

    def test_missing_conflicting_and_foreign_receipts_never_delete(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for rows, before in (([], []), ([dict(kind="container", event="created", id=CID, owner="foreign")], []),
                                 ([dict(kind="container", event="created", id=CID)], [CID])):
                r = runtime(root)
                r.state["browser_container"].update(container_id=CID, before=before)
                with patch.object(c, "hosted"), patch.object(Path, "read_text", return_value="boot"), \
                     patch.object(c, "ROOT", root), patch.object(c, "browser_receipts", return_value=rows), self.assertRaises(b.Refused):
                    c.clean(r)
                r.docker.assert_not_called()


if __name__ == "__main__":
    unittest.main()
