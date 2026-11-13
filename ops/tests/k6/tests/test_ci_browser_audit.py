"""Read-only sandbox diagnostic evidence is never an E2E result."""
import json
from pathlib import Path
import sys
import unittest

CI = Path(__file__).resolve().parents[1] / "ci"
sys.path.insert(0, str(CI))
import browser_audit as a
import yaml

BOOT = "a" * 32
OWNED = [dict(pid=123, uid=1001, start=999)]
MESSAGE = 'apparmor="DENIED" operation="userns_create" profile="unconfined" pid=123 comm="chrome" fsuid=1001 private="SENTINEL"'


def row(message=MESSAGE, **changes):
    return json.dumps(dict(_BOOT_ID=BOOT, __REALTIME_TIMESTAMP="150", MESSAGE=message, **changes)).encode()


class AuditTests(unittest.TestCase):
    def test_exact_owned_denial_is_confirmed_without_any_raw_projection(self):
        result = a.audit_denials(row(), OWNED, BOOT, 100, 200)
        self.assertEqual(result, dict(status="CONFIRMED_APPARMOR_USERNS_DENIAL", categories=["USERNS_CREATE_DENIED"], matched_events=1))
        self.assertNotIn("SENTINEL", json.dumps(result))
        self.assertNotIn("chrome", json.dumps(result))

    def test_comm_alone_boot_window_and_uid_never_grant_correlation(self):
        for raw in (row(MESSAGE.replace("pid=123", "pid=124")),
                    row(MESSAGE.replace("fsuid=1001", "fsuid=1002")),
                    row(MESSAGE.replace('comm="chrome"', 'comm="unshare"')),
                    json.dumps(dict(_BOOT_ID="b" * 32, __REALTIME_TIMESTAMP="150", MESSAGE=MESSAGE)).encode(),
                    json.dumps(dict(_BOOT_ID=BOOT, __REALTIME_TIMESTAMP="99", MESSAGE=MESSAGE)).encode(),
                    json.dumps(dict(_BOOT_ID=BOOT, __REALTIME_TIMESTAMP="201", MESSAGE=MESSAGE)).encode()):
            with self.subTest(raw=raw):
                self.assertEqual(a.audit_denials(raw, OWNED, BOOT, 100, 200)["status"], "INCONCLUSIVE")

    def test_no_event_or_no_ownership_is_inconclusive_not_policy_cleared(self):
        self.assertEqual(a.audit_denials(b"", OWNED, BOOT, 100, 200)["status"], "INCONCLUSIVE")
        self.assertEqual(a.audit_denials(row(), [], BOOT, 100, 200)["status"], "INCONCLUSIVE")
        self.assertEqual(a.audit_denials(row(MESSAGE.replace('"DENIED"', '"ALLOWED"')), OWNED, BOOT, 100, 200)["status"], "INCONCLUSIVE")

    def test_userns_capability_requires_its_specific_profile(self):
        message = MESSAGE.replace('operation="userns_create"', 'operation="capable"').replace('profile="unconfined"', 'profile="unprivileged_userns"')
        self.assertEqual(a.audit_denials(row(message), OWNED, BOOT, 100, 200)["categories"], ["USERNS_CAPABILITY_DENIED"])
        self.assertEqual(a.audit_denials(row(message.replace('profile="unprivileged_userns"', 'profile="other"')), OWNED, BOOT, 100, 200)["status"], "INCONCLUSIVE")

    def test_effective_profile_is_closed(self):
        self.assertEqual(a.profile("unconfined\n"), "UNCONFINED")
        self.assertEqual(a.profile("unprivileged_userns (enforce)"), "UNPRIVILEGED_USERNS")
        self.assertEqual(a.profile("private-sentinel-profile"), "OTHER")

    def test_native_launch_has_no_navigation_or_security_bypass(self):
        args = a.native_arguments("/private/chrome", Path("/private/output"))
        self.assertIn("--no-startup-window", args)
        self.assertIn("--remote-debugging-address=127.0.0.1", args)
        self.assertFalse(any(value.startswith(("http:", "https:", "data:", "about:")) for value in args))
        for value in ("--no-sandbox", "--disable-setuid-sandbox", "--disable-seccomp-filter-sandbox", "--ignore-certificate-errors"):
            self.assertNotIn(value, args)

    def test_workflow_cannot_compile_bootstrap_stack_or_change_policy(self):
        workflow = yaml.load((CI.parents[3] / ".github/workflows/k6-browser-diagnostic.yml").read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(set(workflow["on"]), {"workflow_dispatch"})
        self.assertEqual(workflow["permissions"], {"contents": "read"})
        steps = workflow["jobs"]["diagnose"]["steps"]
        for step in steps:
            command = step.get("run", "")
            for forbidden in ("cargo ", "docker ", "kind ", "sysctl", "apparmor_parser", "chmod 4755", "${{"):
                self.assertNotIn(forbidden, command)
            if "uses" in step:
                self.assertRegex(step["uses"], r"@[a-f0-9]{40}$")
        self.assertEqual(steps[-2]["if"], "${{ always() }}")
        for path in steps[-1]["with"]["path"].splitlines():
            self.assertTrue(path.endswith(("public/diagnostic.json", "public/cleanup.json")))


if __name__ == "__main__":
    unittest.main()
