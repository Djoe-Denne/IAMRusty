"""Baseline Docker trial policy, ownership and no E2E substitution contracts."""
import copy
import os
from pathlib import Path
import sys
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
        for forbidden in ("--privileged", "SYS_ADMIN", "docker.sock", "seccomp=unconfined", "apparmor=unconfined", "--pid=host"):
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
        self.assertEqual(len(c.FILES), 6)
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
        source = (CI / "browser_container.py").read_text()
        self.assertIn('"NOT_E2E_SANDBOX_TRIAL"', source)
        self.assertIn("child.sandbox is True", source)
        self.assertIn("child.renderer_observed is True", source)


if __name__ == "__main__":
    unittest.main()
