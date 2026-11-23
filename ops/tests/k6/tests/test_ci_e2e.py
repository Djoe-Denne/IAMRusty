"""Offline admission, restoration and cleanup boundaries of the hosted parent."""
import copy
import json
from pathlib import Path
import sqlite3
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

    def test_pr_local_packaging_cannot_admit_a_different_target_or_attempt(self):
        repo, ref = "o/r", "refs/heads/master"
        env = dict(GITHUB_ACTIONS="true", GITHUB_JOB="e2e", GITHUB_EVENT_NAME="issue_comment",
                   GITHUB_REPOSITORY=repo, GITHUB_REF=ref, GITHUB_SHA=H, RUNNER_ENVIRONMENT="github-hosted",
                   GITHUB_WORKFLOW_REF=f"{repo}/.github/workflows/k6-e2e-pr.yml@{ref}",
                   K6_PR_MODE="true", K6_PR_TARGET_SHA=SHA, GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="1")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            source = root / "packaged"
            source.mkdir()
            archive = source / "images.tar.gz"
            archive.write_bytes(b"synthetic archive")
            (source / "provenance.json").write_text(json.dumps({**provenance(), "archive_sha256": b.sha(archive)}))
            (source / "cleanup.json").write_text(json.dumps(dict(schema="aiforall.k6-packaging-cleanup/v1",
                status="PASS", owned_running=0, run_id="123", run_attempt="1")))
            (source / "docker-resources.jsonl").write_text("{}\n")
            output = root / "images"
            for run, attempt, target in (("123", "1", H), ("123", "2", SHA), ("456", "1", SHA)):
                with self.subTest(attempt=attempt, target=target), self.assertRaisesRegex(b.Refused, "IMAGE_CURRENT_CI_ONLY"):
                    i.from_packaging(source, output, run, attempt, target, env)
                self.assertFalse(output.exists())
            i.from_packaging(source, output, "123", "1", SHA, env)
            receipt = json.loads((output / "admission.json").read_bytes())
            self.assertEqual((receipt["mode"], receipt["target_sha"], receipt["packaging_sha"]), ("pr-ci-binaries", SHA, H))

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
    def test_standard_pki_id_cannot_collide_with_harness_pki_or_publish_unclean_child(self):
        case = "pki"
        record = dict(id=case, script="smoke/pki.js", phase="HTTP", lifecycle="standard",
                      timeout_seconds=47, requests=3, checks={"probe": "functional", "cleanup": "cleanup"})
        for code, clean in ((0, True), (99, True), (0, False)):
            with self.subTest(code=code, clean=clean), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                output = root / "private" / "cases" / case
                (output / "private").mkdir(parents=True)
                (root / "private/pki").mkdir()
                state = dict(run_id="a" * 32, tools=dict(k6=dict(path="/unexecuted/k6", sha256="tool")),
                             completed_cases=[], process_evidence={})
                parent = c.Campaign(Mock(directory=root, state=state))
                parent.public.mkdir()
                value = dict(version=1, run_id=state["run_id"], phase="HTTP", cases=[case], expected=1,
                             started=1, completed=1, iterations=1, requests=3,
                             checks={name: dict(passes=1, fails=0) for name in record["checks"]})
                (output / "private/summary.json").write_text(json.dumps(value))
                child = Mock(code=code, cleanup=clean)
                child.execute.return_value = code
                catalog = {**c.evidence.CATALOG, case: record}
                cases = {**c.evidence.CASES, case: ("HTTP", frozenset(record["checks"]), 3)}
                with patch.object(c.evidence, "CATALOG", catalog), patch.object(c.evidence, "CASES", cases), \
                     patch.object(c, "new_output", return_value=output) as allocate, patch.object(c, "Child", return_value=child), \
                     patch.object(c, "sha", side_effect=lambda path: b.sha(path) if Path(path) == c.evidence.CATALOG_PATH else "tool"):
                    if code == 0 and clean:
                        parent.scenario(case)
                        self.assertEqual(json.loads((parent.public / "cases" / (case + ".json")).read_text()), value)
                    else:
                        with self.assertRaises((b.Refused, c.evidence.EvidenceError)):
                            parent.scenario(case)
                        self.assertFalse((parent.public / "cases").exists())
                        self.assertEqual(state["completed_cases"], [])
                args, kwargs = child.execute.call_args
                allocate.assert_called_once_with(str(output))
                self.assertTrue((root / "private/cases/pki.attempt").is_file())
                self.assertTrue((root / "private/pki").is_dir())
                self.assertEqual(Path(args[0][-1]), b.ROOT / "ops/tests/k6/scenarios" / record["script"])
                self.assertEqual(kwargs["timeout"], 47)
                self.assertEqual(kwargs["env"]["K6_CASE_ID"], case)

    def test_catalogue_drift_refuses_before_any_child_or_attempt(self):
        with tempfile.TemporaryDirectory() as directory:
            parent = c.Campaign(Mock(directory=Path(directory), state={}))
            with patch.object(c, "sha", return_value="0" * 64), patch.object(c, "Child") as child:
                with self.assertRaises(b.Refused):
                    parent.scenario("iam-health")
            child.assert_not_called()
            self.assertFalse((parent.root / "private").exists())

    def test_catalogue_dispatches_added_ordinary_case_in_order_without_new_branch(self):
        runtime = Mock(directory=Path("unused"), state=dict(completed_cases=[], restoration="PASS"))
        parent = c.Campaign(runtime)
        parent.admit_services = Mock()
        parent.publish = Mock()
        order = []
        parent.scenario = Mock(side_effect=lambda case: order.append(case))

        def san():
            order.extend(("auth-envoy-san-good", "auth-envoy-san-bad", "auth-envoy-san-restored"))
            return {}, {}, {}

        parent.run_san = Mock(side_effect=san)
        parent.run_oauth = Mock(side_effect=lambda: order.append("auth-oauth-browser"))

        def link_email():
            order.append(c.evidence.LINK_EMAIL_CASE)
            return c.evidence.LINK_EMAIL_CORRELATION

        parent.run_link_email = Mock(side_effect=link_email)
        catalog = copy.deepcopy(c.evidence.CATALOG)
        extra = {**catalog["iam-health"], "id": "ordinary-added", "script": "smoke/ordinary-added.js"}
        catalog = dict(list(catalog.items())[:1] + [(extra["id"], extra)] + list(catalog.items())[1:])
        parent.state["completed_cases"] = order
        with patch.object(c.evidence, "CATALOG", catalog):
            parent.run_cases()
        self.assertEqual(order, list(catalog))
        parent.scenario.assert_any_call("ordinary-added")
        parent.run_san.assert_called_once()
        parent.run_oauth.assert_called_once()
        parent.run_link_email.assert_called_once()
        parent.publish.assert_called_once()
        self.assertEqual(parent.publish.call_args.args[1]["link_email"], c.evidence.LINK_EMAIL_CORRELATION)

    def test_link_correlation_brackets_real_case_and_keeps_observations_private(self):
        state = dict(run_id="a" * 32)
        parent = c.Campaign(Mock(directory=Path("unused"), state=state))
        trace = Mock()
        trace.snapshot.side_effect = [{"edge-before": 0}, {"edge-after": 1}]
        trace.http.side_effect = [(200, b'{"provider-before":0}'), (200, b'{"provider-after":1}')]
        trace.database.side_effect = [{"db-before": 0}, {"db-after": 1}]
        parent.snapshot, parent.http, parent.link_email_state, parent.scenario = (
            trace.snapshot, trace.http, trace.database, trace.scenario)
        with patch.object(c.evidence, "accept_link_email", return_value={"public": 1}) as receiver:
            self.assertEqual(parent.run_link_email(), {"public": 1})
        self.assertEqual([call[0] for call in trace.mock_calls],
                         ["snapshot", "http", "database", "scenario", "http", "database", "snapshot"])
        trace.scenario.assert_called_once_with(c.evidence.LINK_EMAIL_CASE)
        receiver.assert_called_once_with({"edge-before": 0}, {"edge-after": 1},
                                         {"provider-before": 0}, {"provider-after": 1},
                                         {"db-before": 0}, {"db-after": 1}, state["run_id"])

    def test_link_database_projection_is_read_only_and_discriminating(self):
        # In-memory rows exercise the SELECT and its negative witnesses only.
        # The hosted case creates its accounts exclusively through IAM's API.
        run = "a" * 32
        database = "k6_" + run
        state = dict(run_id=run, values=dict(database=database))
        parent = c.Campaign(Mock(directory=Path("unused"), state=state))
        with sqlite3.connect(":memory:") as db:
            self.addCleanup(db.close)
            db.create_function("json_build_object", -1, lambda *parts: json.dumps(dict(zip(parts[::2], parts[1::2]))))
            db.executescript("""
                CREATE TABLE users (id INTEGER PRIMARY KEY, username TEXT);
                CREATE TABLE user_emails (user_id INTEGER, email TEXT, is_primary BOOLEAN, is_verified BOOLEAN);
                CREATE TABLE provider_tokens (user_id INTEGER, provider TEXT, provider_user_id TEXT);
                CREATE TABLE rustycog_outbox_events (id INTEGER);
                INSERT INTO users VALUES (99, 'earlier-browser-account');
            """)

            def read(statement, selected):
                self.assertEqual(selected, database)
                self.assertTrue(statement.lstrip().startswith("WITH examples"))
                db.execute("PRAGMA query_only = ON")
                try:
                    return db.execute(statement).fetchone()[0]
                finally:
                    db.execute("PRAGMA query_only = OFF")

            parent.sql = read
            keys = c.evidence.LINK_EMAIL_STATE.keys()
            self.assertEqual(parent.link_email_state(), dict.fromkeys(keys, 0) | dict(users=1, outbox=0))
            for index, kind in enumerate(("attested", "false", "omitted"), start=1):
                owner, link = f"{run}-owner-{kind}", f"{run}-link-{kind}"
                db.execute("INSERT INTO users VALUES (?, ?)", (index, f"s502-{kind}-{run}"))
                db.executemany("INSERT INTO user_emails VALUES (?, ?, ?, ?)", (
                    (index, owner + "@example.invalid", True, True),
                    (index, link + "@example.invalid", False, kind == "attested"),
                ))
                db.executemany("INSERT INTO provider_tokens VALUES (?, ?, ?)", (
                    (index, "gitlab", owner), (index, "github", link),
                ))
            expected = c.evidence.LINK_EMAIL_STATE | dict(users=4, outbox=0)
            self.assertEqual(parent.link_email_state(), expected)
            for query, changed in (
                ("UPDATE user_emails SET is_verified = FALSE WHERE user_id = 1 AND NOT is_primary", "attested_secondary"),
                ("UPDATE user_emails SET is_verified = TRUE WHERE user_id = 2 AND NOT is_primary", "unproved_secondary"),
                ("UPDATE user_emails SET is_verified = TRUE WHERE user_id = 3 AND NOT is_primary", "unproved_secondary"),
                ("UPDATE user_emails SET is_primary = FALSE WHERE user_id = 2 AND is_primary", "primary_preserved"),
                ("UPDATE user_emails SET user_id = 99 WHERE user_id = 3 AND NOT is_primary", "unproved_secondary"),
                ("UPDATE provider_tokens SET provider_user_id = 'wrong' WHERE user_id = 1 AND provider = 'gitlab'", "existing_method"),
                ("DELETE FROM provider_tokens WHERE user_id = 2 AND provider = 'github'", "new_method"),
                ("INSERT INTO user_emails VALUES (1, 'extra@example.invalid', FALSE, FALSE)", "extra_rows"),
            ):
                with self.subTest(changed=changed):
                    db.execute("SAVEPOINT mutation")
                    db.execute(query)
                    self.assertNotEqual(parent.link_email_state()[changed], expected[changed])
                    db.execute("ROLLBACK TO mutation")
                    db.execute("RELEASE mutation")
        for invalid in ({"run_id": "a" * 31 + "'"}, {"values": {"database": "postgres"}}):
            with patch.object(parent, "sql") as read, patch.dict(state, invalid):
                with self.assertRaisesRegex(b.Refused, "E2E_LINK_EMAIL_DB_SCOPE"):
                    parent.link_email_state()
                read.assert_not_called()

    def test_bad_san_failure_always_attempts_patch_and_positive_restored_stimulus(self):
        parent = self.campaign()
        parent.state["edge_identity"] = ["edge"]
        parent.snapshot = Mock(return_value={})

        def scenario(case):
            if case == "auth-envoy-san-bad":
                raise b.Refused("INJECTED_BAD_FAILURE")

        parent.scenario.side_effect = scenario
        with self.assertRaises(b.Refused):
            parent.run_san()
        self.assertEqual([call.args[0] for call in parent.scenario.call_args_list],
                         ["auth-envoy-san-good", "auth-envoy-san-bad", "auth-envoy-san-restored"])
        self.assertEqual([call.args[0] for call in parent.certificate.call_args_list], ["bad", "restored"])
        self.assertEqual(parent.state["restoration"], "PASS")

    def test_only_validated_summary_enters_fixed_case_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            runtime = Mock(directory=Path(directory), state=dict(run_id="a" * 32))
            parent = c.Campaign(runtime)
            parent.public.mkdir()
            record = c.evidence.CATALOG["iam-health"]
            value = dict(version=1, run_id="a" * 32, phase="HTTP", cases=["iam-health"],
                         expected=1, started=1, completed=1, iterations=1, requests=1,
                         checks={name: dict(passes=1, fails=0) for name in record["checks"]})
            with self.assertRaises(c.evidence.EvidenceError):
                parent.publish_case("iam-health", {**value, "checks": {}})
            self.assertFalse((parent.public / "cases").exists())
            parent.publish_case("iam-health", value)
            self.assertEqual(list((parent.public / "cases").iterdir()), [parent.public / "cases/iam-health.json"])
            with self.assertRaises(b.Refused):
                parent.publish_case("iam-health", value)

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
            (parent.root / "private/cases").mkdir()
            (parent.root / "private/cases/auth-oauth-browser.attempt").write_text("attempted")
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
        self.assertEqual(value["permissions"], {"contents": "read", "actions": "read", "pull-requests": "read"})
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
            if "*" in path:
                self.assertEqual(path, "${{ runner.temp }}/k6-e2e-private/campaign/public/cases/*.json")
        self.assertEqual({Path(path).name for path in steps[clean + 1]["with"]["path"].splitlines()},
                         {"report.json", "provenance.json", "cleanup.json", "admission.json", "startup.json",
                          "processes.json", "browser.json", "browser-docker-resources.jsonl", "correlation.json",
                          "selection.json", "*.json", "docker-resources.jsonl"})
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
