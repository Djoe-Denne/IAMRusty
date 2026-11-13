"""Offline admission, restoration and cleanup boundaries of the hosted parent."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch
import zipfile

CI = Path(__file__).resolve().parents[1] / "ci"
sys.path.insert(0, str(CI))
import binaries as b
import campaign as c
import image_input as i
import runtime as r
import browser as browser_profile
import yaml

SHA, SDK, H = "a" * 40, "b" * 40, "c" * 40


def provenance():
    return dict(schema="aiforall.k6-images/v1", target_sha=SHA, harness_sha=H, sdk_sha=SDK,
                cargo_profile="dev", instrumentation="none", qualification="PACKAGING_AND_ABI_ONLY_NOT_E2E",
                archive_sha256="d" * 64, roles={role: dict(image_id="sha256:" + str(index + 1) * 64,
                    tag=f"local/k6-{role}:k6-123-1", abi=dict(resolution="PASS", paths=i.PATHS[role]),
                    manifest=dict(source_sha=SHA, sdk_sha=SDK, instrumentation="none", role=role))
                    for index, role in enumerate(b.ROLES)})


class ImageAdmissionTests(unittest.TestCase):
    def test_saved_images_are_not_mistaken_for_e2e_qualification(self):
        value = provenance()
        cleanup = dict(schema="aiforall.k6-packaging-cleanup/v1", status="PASS", owned_running=0)
        self.assertEqual(i.provenance(value, cleanup, SHA, H), value)
        for field, invalid in (("target_sha", H), ("harness_sha", SHA), ("instrumentation", "llvm"),
                               ("cargo_profile", "release"), ("qualification", "E2E_PASS")):
            with self.subTest(field=field), self.assertRaises(b.Refused):
                i.provenance({**value, field: invalid}, cleanup, SHA, H)
        for invalid in (True, 1, None):
            with self.assertRaises(b.Refused):
                i.provenance(value, {**cleanup, "owned_running": invalid}, SHA, H)

    def test_exact_abi_paths_and_binary_provenance_required(self):
        cleanup = dict(schema="aiforall.k6-packaging-cleanup/v1", status="PASS", owned_running=0)
        for field, invalid in (("tag", "foreign:latest"), ("image_id", "latest"),
                               ("abi", dict(resolution="PASS", paths=["/app/iam-service"])),
                               ("manifest", dict(source_sha=H, sdk_sha=SDK, instrumentation="none", role="iam"))):
            value = provenance()
            value["roles"]["iam"][field] = invalid
            with self.assertRaises(b.Refused):
                i.provenance(value, cleanup, SHA, H)

    def test_manual_success_same_repository_attempt_only(self):
        run = dict(id=123, run_attempt=1, workflow_id=456, path=".github/workflows/k6-images.yml",
                   event="workflow_dispatch", head_branch="master", status="completed", conclusion="success",
                   repository=dict(id=789, full_name="o/r"), head_repository=dict(id=789, full_name="o/r"))
        i.admit(run, 456, "o/r", 789, 123, 1)
        for changes in (dict(event="push"), dict(head_branch="feature"), dict(conclusion="failure"),
                        dict(run_attempt=2), dict(workflow_id=999), dict(head_repository=dict(id=111, full_name="fork/r"))):
            with self.assertRaises(b.Refused):
                i.admit({**run, **changes}, 456, "o/r", 789, 123, 1)

    def test_closed_zip_refuses_paths_extras_duplicates_and_symlinks(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            for index, extra in enumerate((None, "../escape", "secrets.pem", "report.json")):
                path = root / "archive.zip"
                with zipfile.ZipFile(path, "w") as archive:
                    for name in i.FILES:
                        archive.writestr(name, b"payload")
                    if extra:
                        archive.writestr(extra, b"private")
                if extra:
                    with self.assertRaises(b.Refused):
                        i.extract(path, root / str(index))
                else:
                    i.extract(path, root / str(index))
                    self.assertEqual({p.name for p in (root / str(index)).iterdir()}, i.FILES)


class RestorationTests(unittest.TestCase):
    def test_browser_environment_carries_pinned_version_and_keeps_sandbox_tls(self):
        state = dict(run_id="a" * 32, tools=dict(chromium=dict(path="/private/chrome", version="156.0.8078.73")))
        env = c.stimulus_environment(Path("/private/case"), Path("/private/campaign"), state, True)
        self.assertEqual(env["K6_EXPECTED_BROWSER_VERSION"], "156.0.8078.73")
        self.assertEqual(env["K6_BROWSER_EXECUTABLE_PATH"], "/private/chrome")
        self.assertEqual(env["K6_BROWSER_ARGS"], "enable-logging,log-level=2,log-file=" + str(Path("/private/case/private/chromium.log")))
        self.assertEqual(env["K6_BROWSER_DEBUG"], "false")
        self.assertNotIn("K6_INSECURE_SKIP_TLS_VERIFY", env)
        self.assertEqual(env["SSL_CERT_FILE"], str(Path("/private/campaign/private/pki/ca-bundle.crt")))
        self.assertNotIn("K6_EXPECTED_BROWSER_VERSION", c.stimulus_environment(Path("/private/case"), Path("/private/campaign"), state, False))

    def test_dns_oracle_binds_clusterip_to_ready_current_pod_not_podip_to_dns(self):
        service = dict(metadata=dict(uid="service-uid"), spec=dict(clusterIP="10.96.0.10"))
        pod = dict(metadata=dict(uid="current-pod"), status=dict(podIP="10.244.0.20"))
        slices = [dict(metadata=dict(ownerReferences=[dict(kind="Service", uid="service-uid")]), ports=[dict(port=8443)],
                       endpoints=[dict(conditions=dict(ready=True), targetRef=dict(uid="current-pod"), addresses=["10.244.0.20"])])]
        clusters = [dict(name="iam", host_statuses=[dict(address=dict(socket_address=dict(address="10.96.0.10")))])]
        self.assertTrue(c.iam_endpoint_ready(service, slices, pod, clusters))
        wrong_dns = copy.deepcopy(clusters)
        wrong_dns[0]["host_statuses"][0]["address"]["socket_address"]["address"] = "10.244.0.20"
        self.assertFalse(c.iam_endpoint_ready(service, slices, pod, wrong_dns))
        for key, value in (("conditions", dict(ready=False)), ("targetRef", dict(uid="old-pod")), ("addresses", ["10.244.0.99"])):
            changed = copy.deepcopy(slices)
            changed[0]["endpoints"][0][key] = value
            self.assertFalse(c.iam_endpoint_ready(service, changed, pod, clusters))
        changed = copy.deepcopy(slices)
        changed[0]["metadata"]["ownerReferences"][0]["uid"] = "foreign-service"
        self.assertFalse(c.iam_endpoint_ready(service, changed, pod, clusters))

    def test_sql_socket_invocation_does_not_inherit_readiness_tcp_address(self):
        runtime = Mock()
        runtime.directory = Path("unused")
        runtime.state = dict(values=dict(database="k6_" + "a" * 32))
        runtime.k.return_value = b"1\n"
        parent = c.Campaign(runtime)
        parent.postgres_pod = Mock(return_value="postgres-owned-pod")
        self.assertEqual(parent.sql("SELECT 1"), "1")
        args = runtime.k.call_args.args[0]
        self.assertEqual(args[:11], ["exec", "postgres-owned-pod", "-c", "postgres", "--", "env", "-u", "PGHOSTADDR", "-u", "PGHOST", "psql"])
        self.assertEqual(args[args.index("--host") + 1], "/var/run/postgresql")
        self.assertNotIn("PGSSLMODE=disable", args)

    def campaign(self):
        runtime = Mock()
        runtime.directory = Path("unused")
        runtime.state = dict(certificate_phase="uncertain", restoration="NOT_REQUIRED")
        parent = c.Campaign(runtime)
        parent.certificate = Mock()
        parent.scenario = Mock()
        return parent

    def test_restoration_requires_positive_scenario_not_only_a_patch(self):
        parent = self.campaign()
        parent.restore()
        parent.certificate.assert_called_once_with("restored")
        parent.scenario.assert_called_once_with("auth-envoy-san-restored")
        self.assertEqual(parent.state["restoration"], "PASS")

    def test_restoration_patch_or_positive_proof_failure_not_pass(self):
        for operation in ("certificate", "scenario"):
            parent = self.campaign()
            getattr(parent, operation).side_effect = b.Refused("TEST_FAILURE")
            with self.assertRaises(b.Refused):
                parent.restore()
            self.assertNotEqual(parent.state["restoration"], "PASS")

    def test_already_restored_is_not_replayed(self):
        parent = self.campaign()
        parent.state["restoration"] = "PASS"
        parent.restore()
        parent.certificate.assert_not_called()

    def test_cleanup_exception_still_publishes_failure_and_no_zero_leak_claim(self):
        parent = self.campaign()
        parent.restore = Mock(side_effect=b.Refused("RESTORE_FAILED"))
        parent.r.close.side_effect = b.Refused("CLOSE_FAILED")
        parent.r.receipts.return_value = []
        parent.state.update(run_id="a" * 32, verdict="PASS", completed_cases=[], images=provenance(), tools={})
        with tempfile.TemporaryDirectory() as directory:
            parent.public = Path(directory)
            parent.root = Path(directory)
            (parent.root / "private").mkdir()
            with patch.dict(c.os.environ, {"GITHUB_SHA": H}):
                c.finish(parent)
            report = json.loads((parent.public / "report.json").read_text())
            cleanup = json.loads((parent.public / "cleanup.json").read_text())
            self.assertEqual(report["verdict"], "FAIL")
            self.assertEqual(report["restoration"], "FAIL")
            self.assertIsNone(cleanup["owned_running"])

    def test_hard_interruption_without_child_receipt_never_claims_zero_leaks(self):
        parent = self.campaign()
        parent.restore = Mock()
        parent.r.close.return_value = dict(status="PASS", owned_running=0, errors=[])
        parent.r.receipts.return_value = []
        parent.state.update(run_id="a" * 32, verdict="PASS", completed_cases=[], images=provenance(), tools={})
        with tempfile.TemporaryDirectory() as directory:
            parent.root = Path(directory)
            parent.public = parent.root / "public"
            parent.public.mkdir()
            (parent.root / "private").mkdir()
            (parent.root / "private/auth-oauth-browser.attempt").write_text("attempted")
            with patch.dict(c.os.environ, {"GITHUB_SHA": H}):
                c.finish(parent)
            cleanup = json.loads((parent.public / "cleanup.json").read_text())
            self.assertEqual(cleanup["status"], "FAIL")
            self.assertIn("CHILD_CLEANUP_UNPROVEN", cleanup["errors"])
            self.assertIsNone(cleanup["owned_running"])


class WorkflowTests(unittest.TestCase):
    def test_manual_and_reusable_workflow_keep_always_cleanup_before_raw_allowlisted_upload(self):
        value = yaml.load((b.ROOT / ".github/workflows/k6-e2e.yml").read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(set(value["on"]), {"workflow_dispatch", "workflow_call"})
        self.assertEqual(value["permissions"], dict(contents="read", actions="read"))
        steps = value["jobs"]["e2e"]["steps"]
        clean = next(n for n, step in enumerate(steps) if "campaign.py clean" in step.get("run", ""))
        self.assertEqual(steps[clean]["if"], "${{ always() }}")
        self.assertIn("upload-artifact@", steps[clean + 1]["uses"])
        for step in steps:
            command = step.get("run", "")
            self.assertNotIn("${{", command)
            for forbidden in ("cargo ", "docker build", "no-sandbox", "sysctl", "apparmor", "ignore-certificate-errors"):
                self.assertNotIn(forbidden, command)
            if "uses" in step:
                self.assertRegex(step["uses"], r"@[a-f0-9]{40}$")
        for path in steps[clean + 1]["with"]["path"].splitlines():
            self.assertTrue(path.startswith("${{ runner.temp }}/k6-e2e-private/campaign/public/"))
            self.assertNotIn("*", path)
        self.assertEqual(steps[-1]["with"]["path"], "${{ runner.temp }}/k6-e2e-site/")

    def test_private_hosted_cluster_only(self):
        with self.assertRaises(b.Refused):
            r.hosted({})
        text = (CI / "runtime.py").read_text()
        self.assertNotIn("activate_project", text)
        self.assertNotIn('"prune"', text)
        self.assertNotIn('"--force"', text)
        self.assertNotIn('"rm"', text)


class NodeOwnershipTests(unittest.TestCase):
    def test_cli_diagnostic_never_contains_private_error_content(self):
        raw = b'Error from server (Invalid): secret invalid password=DO_NOT_EXPORT'
        self.assertEqual(r.cli_failure("kubectl", raw), "E2E_CLI_FAILED_KUBECTL_INVALID")
        self.assertNotIn("DO_NOT_EXPORT", r.cli_failure("kubectl", raw))
        self.assertEqual(r.cli_failure("kubectl", b"unrecognized private output"), "E2E_CLI_FAILED_KUBECTL_UNCLASSIFIED")

    def test_name_or_label_does_not_grant_cleanup_authority(self):
        state = dict(job_run_id="123", job_attempt="1", run_id="a" * 32, cluster="k6-e2e-123-1", before_containers=[])
        row = dict(event="created", kind="container", id="b" * 64, owner=r.OWNER, run_id="123", run_attempt="1",
                   campaign="a" * 32, source="docker-engine-create-event", name="k6-e2e-123-1-control-plane", safe_to_delete="no")
        self.assertEqual(r.owned_nodes([row], state), {"b" * 64})
        for changed in (dict(source="label"), dict(run_attempt="2"), dict(campaign="c" * 32), dict(owner="foreign"),
                        dict(safe_to_delete="ask"), dict(name="other-control-plane")):
            with self.subTest(changed=changed), self.assertRaises(b.Refused):
                r.owned_nodes([{**row, **changed}], state)
        with self.assertRaises(b.Refused):
            r.owned_nodes([row], {**state, "before_containers": ["b" * 64]})
        with self.assertRaises(b.Refused):
            r.owned_nodes([row, row], state)
        self.assertEqual(r.owned_nodes([], state), set())


class NativeBrowserDiagnosticTests(unittest.TestCase):
    def test_private_fatal_projection_never_exports_urls_or_secrets(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "chromium.log"
            path.write_text('[FATAL:zygote] No usable sandbox! https://private.invalid/?token=SENTINEL\n')
            result = browser_profile.private_log_diagnostic(path)
            self.assertEqual(result, dict(log="CLASSIFIED", causes=["NO_USABLE_SANDBOX"]))
            self.assertNotIn("SENTINEL", json.dumps(result))

    def test_multiple_causes_unknown_empty_absent_invalid_or_large_are_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "chromium.log"
            self.assertEqual(browser_profile.private_log_diagnostic(path)["log"], "ABSENT")
            for raw, expected in ((b"", "EMPTY"), (b"private unknown", "UNCLASSIFIED"),
                                  (b"\xff", "WITHHELD"), (b"a" * 65537, "WITHHELD")):
                path.write_bytes(raw)
                self.assertEqual(browser_profile.private_log_diagnostic(path)["log"], expected)
            path.write_text('chrome_crashpad_handler: --database is required\nFailed to move to new namespace: Operation not permitted')
            self.assertEqual(browser_profile.private_log_diagnostic(path)["causes"], ["CRASHPAD", "NAMESPACE_DENIED"])

    def test_special_file_is_withheld(self):
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(browser_profile.private_log_diagnostic(Path(directory))["log"], "WITHHELD")


if __name__ == "__main__":
    unittest.main()
