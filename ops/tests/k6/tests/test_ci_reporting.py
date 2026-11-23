"""Automatic trusted-CI admission and fail-closed public test reporting."""
from contextlib import redirect_stdout
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

CI = Path(__file__).resolve().parents[1] / "ci"
sys.path.insert(0, str(CI))
import artifacts as a
import binaries as b
import evidence as e
import image_input as images
import reporting as r
import cleanup as packaging_cleanup
import yaml
from test_ci_campaign_browser import result as browser_result
from test_ci_e2e import provenance

SHA, H, RUN = "a" * 40, "c" * 40, "a" * 32
BASH = os.environ.get("AIFORALL_BASH") or (shutil.which("bash") if os.name != "nt" else None)
EXPECTED = sum(len(record["checks"]) for record in e.CATALOG.values())
HEALTH_CHECKS = len(e.CASES["iam-health"][1])
OAUTH_CHECKS = len(e.OAUTH_CHECKS)


def dump(root, name, value):
    (root / name).write_text(json.dumps(value), encoding="utf-8")


def public_fixture(root):
    root.mkdir()
    (root / "cases").mkdir()
    dump(root, "selection.json", e.selection_receipt(RUN, H, SHA))
    dump(root, "report.json", dict(run_id=RUN, verdict="PASS", cleanup="PASS", restoration="PASS", completed_cases=list(e.CASES)))
    dump(root, "provenance.json", dict(target_sha=SHA, harness_sha=H))
    dump(root, "cleanup.json", dict(status="PASS", owned_running=0, errors=[]))
    dump(root, "correlation.json", r.CORRELATION)
    dump(root, "processes.json", {case: dict(exit=0, cleanup=True) for case in e.CASES})
    dump(root, "browser.json", dict(result=browser_result(), container_exit=0, cleanup="PASS"))
    for case, (phase, checks, requests) in e.CASES.items():
        dump(root / "cases", case + ".json", dict(version=1, run_id=RUN, phase=phase, cases=[case], expected=1,
            started=1, completed=1, iterations=1, requests=requests, checks={name: dict(passes=1, fails=0) for name in checks}))


def project(root, target=None):
    return r.project(root / "source", root / "site", "Djoe-Denne/IAMRusty", "123", "1", H, target or SHA)


def failed_oauth_fixture(root, *, invalid=False):
    public_fixture(root)
    case = "auth-oauth-browser"
    accepted = root / "cases" / (case + ".json")
    partial = json.loads(accepted.read_text(encoding="utf-8"))
    accepted.unlink()
    partial.update(completed=None, checks={"browser-version": dict(passes=1, fails=0),
                                           "registration-callback": dict(passes=0, fails=1)})
    if invalid:
        partial.update(private="SECRET_SENTINEL")
    processes = json.loads((root / "processes.json").read_text(encoding="utf-8"))
    processes[case] = dict(exit=99, cleanup=True, sandbox=True, renderer_observed=True,
                           summary_status="captured", failed_summary=partial)
    dump(root, "processes.json", processes)
    report = json.loads((root / "report.json").read_text(encoding="utf-8"))
    dump(root, "report.json", {**report, "verdict": "FAIL",
                              "completed_cases": [value for value in e.CASES if value != case]})


def results(root):
    values = (json.loads(path.read_text(encoding="utf-8")) for path in (root / "site/allure-results").glob("*-result.json"))
    return {value["fullName"]: value for value in values}


class ReportingTests(unittest.TestCase):
    def test_missing_completion_with_all_assertions_passing_is_incomplete(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            public_fixture(source)
            case = list(e.CASES)[-1]
            path = source / "cases" / (case + ".json")
            partial = json.loads(path.read_text(encoding="utf-8"))
            path.unlink()
            processes = json.loads((source / "processes.json").read_text(encoding="utf-8"))
            processes[case] = dict(exit=99, cleanup=True, summary_status="captured",
                                   failed_summary={**partial, "completed": None})
            dump(source, "processes.json", processes)
            report = json.loads((source / "report.json").read_text(encoding="utf-8"))
            dump(source, "report.json", {**report, "verdict": "FAIL", "stage": "scenario:" + case})
            metric = project(root)
            self.assertEqual((metric["status"], metric["failure_categories"]), ("failed", ["incomplete"]))

    def test_restoration_failure_and_readiness_failure_keep_their_categories(self):
        for stage in ("restoration", "readiness"):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                source = root / "source"
                public_fixture(source)
                report = json.loads((source / "report.json").read_text(encoding="utf-8"))
                report.update(verdict="FAIL", stage=stage)
                if stage == "restoration":
                    report["restoration"] = "FAIL"
                    expected = ["cleanup"]
                else:
                    for path in (source / "cases").glob("*.json"):
                        path.unlink()
                    dump(source, "processes.json", {})
                    expected = ["incomplete", "infrastructure"]
                dump(source, "report.json", report)
                metric = project(root)
                self.assertEqual((metric["status"], metric["failure_categories"]), ("failed", expected))

    def test_cleanup_infrastructure_and_missing_execution_have_distinct_categories(self):
        for failure in ("cleanup", "infrastructure", "incomplete"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                public_fixture(root / "source")
                if failure == "cleanup":
                    dump(root / "source", "cleanup.json", dict(status="FAIL", owned_running=1, errors=[]))
                elif failure == "infrastructure":
                    dump(root / "source", "correlation.json", {**r.CORRELATION, "iam_delta": 8})
                else:
                    (root / "source/cases/iam-health.json").unlink()
                metric = project(root)
                self.assertEqual(metric["failure_categories"], [failure])

    def test_same_run_summary_never_overrides_actual_nonzero_exit_or_unclean_child(self):
        for change in (dict(exit=99), dict(cleanup=False), dict(exit=False)):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                public_fixture(root / "source")
                processes = json.loads((root / "source/processes.json").read_text())
                processes["iam-health"].update(change)
                dump(root / "source", "processes.json", processes)
                metric = project(root)
                self.assertEqual((metric["status"], metric["passed"]), ("failed", EXPECTED - HEALTH_CHECKS))
                if change.get("cleanup") is False:
                    self.assertIn("cleanup", metric["failure_categories"])

    def test_selection_receipt_is_required_exact_ordered_run_and_source_bound(self):
        for change in (None, dict(run_id="b" * 32), dict(catalog_sha256="0" * 64),
                       dict(cases=list(reversed(e.CASES))), dict(harness_sha=SHA),
                       dict(target_sha=H), dict(unknown="SECRET_SENTINEL")):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                public_fixture(root / "source")
                path = root / "source/selection.json"
                if change is None:
                    path.unlink()
                else:
                    dump(root / "source", "selection.json", {**json.loads(path.read_text()), **change})
                metric = project(root)
                self.assertEqual((metric["status"], metric["passed"], metric["expected"]), ("incomplete", 0, EXPECTED))

    def test_old_root_case_files_are_not_accepted_and_unknown_files_never_project(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            public_fixture(root / "source")
            case = root / "source/cases/iam-health.json"
            case.rename(root / "source/iam-health.json")
            dump(root / "source/cases", "unknown.json", dict(private="SECRET_SENTINEL"))
            metric = project(root)
            self.assertEqual((metric["status"], metric["passed"], metric["expected"]),
                             ("failed", EXPECTED - HEALTH_CHECKS, EXPECTED))
            for path in (root / "site").rglob("*"):
                if path.is_file():
                    self.assertNotIn("SECRET_SENTINEL", path.read_text(encoding="utf-8"))

    def test_added_declaration_changes_report_denominator_and_missing_case_stays_unproven(self):
        record = {**e.CATALOG["iam-health"], "id": "ordinary-added"}
        catalog = {**e.CATALOG, "ordinary-added": record}
        cases = {**e.CASES, "ordinary-added": ("HTTP", frozenset(record["checks"]), 1)}
        expanded = EXPECTED + len(record["checks"])
        with patch.object(e, "CATALOG", catalog), patch.object(e, "CASES", cases):
            for missing in (False, True):
                with self.subTest(missing=missing), tempfile.TemporaryDirectory() as directory:
                    root = Path(directory)
                    public_fixture(root / "source")
                    if missing:
                        (root / "source/cases/ordinary-added.json").unlink()
                    metric = project(root)
                    self.assertEqual(metric["expected"], expanded)
                    self.assertEqual(metric["passed"], EXPECTED if missing else expanded)
                    self.assertEqual(metric["status"], "failed" if missing else "passed")
                    self.assertEqual(len(results(root)), expanded + 1)
                    self.assertIn(f"{expanded} declared checks", (root / "site/index.html").read_text())

    def test_declared_checks_and_distinct_parent_contract_are_not_code_coverage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            public_fixture(root / "source")
            metric = project(root)
            self.assertEqual((metric["status"], metric["passed"], metric["expected"]), ("passed", EXPECTED, EXPECTED))
            files = list((root / "site/allure-results").glob("*-result.json"))
            self.assertEqual(len(files), EXPECTED + 1)
            for file in files:
                value = json.loads(file.read_text())
                self.assertEqual(value["status"], "passed")
                self.assertNotIn("start", value)
                self.assertNotIn("stop", value)
                self.assertNotIn("attachments", value)
            self.assertEqual(json.loads((root / "site/badge.json").read_text())["message"], f"{EXPECTED}/{EXPECTED}")

    def test_parent_failure_cannot_leave_green_badge_even_with_all_passed_checks(self):
        for name, change in (("cleanup.json", dict(status="FAIL")), ("cleanup.json", dict(owned_running=False)),
                             ("correlation.json", dict(iam_delta=8)), ("report.json", dict(verdict="FAIL"))):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                public_fixture(root / "source")
                value = json.loads((root / "source" / name).read_text())
                dump(root / "source", name, {**value, **change})
                metric = project(root)
                self.assertNotEqual(metric["status"], "passed")
                self.assertEqual(metric["passed"], EXPECTED)
                self.assertEqual(json.loads((root / "site/badge.json").read_text())["color"], "red")

    def test_missing_or_stale_phase_is_unproven_not_a_fake_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            public_fixture(root / "source")
            (root / "source/cases/auth-oauth-browser.json").unlink()
            metric = project(root)
            self.assertEqual((metric["passed"], metric["unproven"]), (EXPECTED - OAUTH_CHECKS, OAUTH_CHECKS))
            self.assertNotEqual(metric["status"], "passed")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            public_fixture(root / "source")
            metric = project(root, "d" * 40)
            self.assertEqual((metric["status"], metric["passed"]), ("incomplete", 0))

    def test_no_private_inputs_unknown_diagnostics_or_sentinels_are_projected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            public_fixture(root / "source")
            dump(root / "source", "credentials.json", dict(token="SECRET_SENTINEL"))
            report = json.loads((root / "source/report.json").read_text())
            dump(root / "source", "report.json", {**report, "diagnostic": "SECRET_SENTINEL"})
            project(root)
            for file in (root / "site").rglob("*"):
                if file.is_file():
                    self.assertNotIn("SECRET_SENTINEL", file.read_text(encoding="utf-8"))

    def test_missing_run_still_produces_incomplete_red_report(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            metric = project(root)
            self.assertEqual((metric["status"], metric["passed"], metric["unproven"]), ("incomplete", 0, EXPECTED))
            self.assertTrue((root / "site/index.html").is_file())

    def test_threshold_failure_names_observed_checks_without_counting_partial_success(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            failed_oauth_fixture(root / "source")
            job_summary = root / "summary.md"
            with redirect_stdout(io.StringIO()) as log:
                metric = r.project(root / "source", root / "site", "Djoe-Denne/IAMRusty", "123", "1", H, SHA, job_summary)
            self.assertEqual((metric["status"], metric["passed"], metric["unproven"]),
                             ("failed", EXPECTED - OAUTH_CHECKS, OAUTH_CHECKS))
            self.assertIn("functional", metric["failure_categories"])
            self.assertIn("incomplete", metric["failure_categories"])
            report = results(root)
            failed = report["auth-oauth-browser.registration-callback"]
            self.assertEqual(failed["status"], "failed")
            self.assertIn("0 passed, 1 failed", failed["statusDetails"]["message"])
            partial_pass = report["auth-oauth-browser.browser-version"]
            self.assertEqual(partial_pass["status"], "broken")
            self.assertIn("1 passed, 0 failed", partial_pass["statusDetails"]["message"])
            not_observed = report["auth-oauth-browser.login-nonce"]
            self.assertEqual(not_observed["status"], "broken")
            self.assertIn("no observation", not_observed["statusDetails"]["message"])
            parent = report["E2E.parent-contract"]
            message = parent["statusDetails"]["message"]
            summary = job_summary.read_text(encoding="utf-8")
            for marker in ("k6 exit 99", "completed=unavailable", "registration-callback", f"observed checks: 2/{OAUTH_CHECKS}",
                           "renderer_observed=yes"):
                self.assertIn(marker, message)
                self.assertIn(marker, summary)
                self.assertIn(marker, log.getvalue())
            self.assertIn("`browser-version`: 1 passed, 0 failed", summary)
            self.assertIn("auth-oauth-browser.registration-callback: 0 passed, 1 failed", log.getvalue())
            self.assertEqual(parent["links"][0]["url"], metric["run_url"])

    def test_invalid_failure_diagnostic_emits_exit_but_never_private_content(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            failed_oauth_fixture(root / "source", invalid=True)
            with redirect_stdout(io.StringIO()) as log:
                self.assertEqual(project(root)["passed"], EXPECTED - OAUTH_CHECKS)
            self.assertNotIn("SECRET_SENTINEL", log.getvalue())
            parent = results(root)["E2E.parent-contract"]
            self.assertIn("k6 exit 99 (thresholds not met); diagnostic summary invalid", parent["statusDetails"]["message"])
            for file in (root / "site").rglob("*"):
                if file.is_file():
                    self.assertNotIn("SECRET_SENTINEL", file.read_text(encoding="utf-8"))

    def test_failed_stage_remains_visible_without_a_child_summary(self):
        case = "auth-oauth-browser"
        for process, expected in (
            (dict(exit=99, cleanup=True), "k6 exit 99 (thresholds not met); diagnostic summary unavailable"),
            (None, "scenario did not complete; no process evidence available"),
        ):
            with self.subTest(process=process), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                failed_oauth_fixture(root / "source")
                report = json.loads((root / "source/report.json").read_text(encoding="utf-8"))
                dump(root / "source", "report.json", {**report, "stage": "scenario:" + case})
                processes = json.loads((root / "source/processes.json").read_text(encoding="utf-8"))
                if process:
                    processes[case] = process
                else:
                    del processes[case]
                dump(root / "source", "processes.json", processes)
                with redirect_stdout(io.StringIO()) as log:
                    self.assertEqual(project(root)["passed"], EXPECTED - OAUTH_CHECKS)
                self.assertIn(expected, log.getvalue())
                parent = results(root)["E2E.parent-contract"]
                self.assertIn(f"{case}: {expected}", parent["statusDetails"]["message"])


def ci_env():
    return dict(GITHUB_ACTIONS="true", GITHUB_EVENT_NAME="push", GITHUB_REF="refs/heads/master", GITHUB_JOB="e2e", GITHUB_RUN_ID="123",
                GITHUB_RUN_ATTEMPT="1", GITHUB_SHA=SHA)


class AutomaticAdmissionTests(unittest.TestCase):
    def test_packaging_cleanup_accepts_only_trusted_auto_job_and_never_adopts_a_label(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            with patch.object(packaging_cleanup, "ROOT", root), patch.object(packaging_cleanup, "command", return_value="") as command:
                packaging_cleanup.cleanup(root / "packaging", ci_env())
                self.assertTrue(all(call.args[0][1:3] == ["container", "ls"] for call in command.call_args_list))
            value = json.loads((root / "packaging/cleanup.json").read_text())
            self.assertEqual((value["status"], value["owned_running"]), ("PASS", 0))
            with patch.object(packaging_cleanup, "command") as command, self.assertRaises(b.Refused):
                packaging_cleanup.cleanup(root / "forbidden", {**ci_env(), "GITHUB_EVENT_NAME": "pull_request"})
            command.assert_not_called()

    def test_auto_admission_is_bound_to_own_push_master_run_attempt_and_sha(self):
        env = ci_env()
        self.assertTrue(a.current_ci_source(env, "123", "1", SHA))
        for change in (dict(GITHUB_EVENT_NAME="pull_request"), dict(GITHUB_REF="refs/pull/1/merge"),
                       dict(GITHUB_JOB="arbitrary"), dict(GITHUB_RUN_ID="999"), dict(GITHUB_RUN_ATTEMPT="2"), dict(GITHUB_SHA=H)):
            self.assertFalse(a.current_ci_source({**env, **change}, "123", "1", SHA))
            with patch.object(a, "GitHub") as api, self.assertRaises(b.Refused):
                a.acquire(Path("unused"), "123", "1", SHA, {**env, **change})
            api.assert_not_called()

    def test_local_images_require_current_ci_and_exact_closed_packaging_provenance(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            source = root / "packaged"
            source.mkdir()
            (source / "images.tar.gz").write_bytes(b"payload")
            value = provenance()
            value.update(harness_sha=SHA, archive_sha256=b.sha(source / "images.tar.gz"))
            dump(source, "provenance.json", value)
            dump(source, "cleanup.json", dict(schema="aiforall.k6-packaging-cleanup/v1", status="PASS", owned_running=0,
                                              run_id="123", run_attempt="1"))
            (source / "docker-resources.jsonl").write_text("{}\n")
            (source / "private-secret").write_text("SENTINEL")
            images.from_packaging(source, root / "images", "123", "1", SHA, ci_env())
            self.assertEqual({p.name for p in (root / "images/verified").iterdir()}, images.FILES)
            with self.assertRaises(b.Refused):
                images.from_packaging(source, root / "foreign", "999", "1", SHA, ci_env())
            (source / "images.tar.gz").write_bytes(b"changed")
            with self.assertRaises(b.Refused):
                images.from_packaging(source, root / "changed", "123", "1", SHA, ci_env())


class AllureToolTests(unittest.TestCase):
    def test_hash_mismatch_refuses_before_extracting_any_tool(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            raw = b"not-an-archive"
            dump(root, "allure.lock.json", dict(version="2.0.0", bytes=len(raw), sha256="0" * 64,
                url="https://github.com/allure-framework/allure2/releases/download/2.0.0/allure-2.0.0.tgz"))
            with patch.object(r, "HERE", root), patch.object(r.urllib.request, "urlopen", return_value=io.BytesIO(raw)), self.assertRaises(b.Refused):
                r.install_allure(root / "tool")
            self.assertFalse((root / "tool/allure-2.0.0").exists())

    def test_verified_archive_only_no_links_or_unsafe_extraction(self):
        for unsafe in (False, True):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory).resolve()
                stream = io.BytesIO()
                with tarfile.open(fileobj=stream, mode="w:gz") as archive:
                    member = tarfile.TarInfo("allure-2.0.0/bin/allure")
                    data = b"verified-tool"
                    if unsafe:
                        member.type = tarfile.SYMTYPE
                        member.linkname = "/private/foreign"
                        archive.addfile(member)
                    else:
                        member.size = len(data)
                        archive.addfile(member, io.BytesIO(data))
                raw = stream.getvalue()
                dump(root, "allure.lock.json", dict(version="2.0.0", bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest(),
                    url="https://github.com/allure-framework/allure2/releases/download/2.0.0/allure-2.0.0.tgz"))
                with patch.object(r, "HERE", root), patch.object(r.urllib.request, "urlopen", return_value=io.BytesIO(raw)):
                    if unsafe:
                        with self.assertRaises(b.Refused):
                            r.install_allure(root / "tool")
                    else:
                        self.assertEqual(r.install_allure(root / "tool").read_bytes(), data)


class WorkflowTests(unittest.TestCase):
    def test_e2e_runs_after_build_alongside_not_after_coverage_and_pages_is_separate(self):
        value = yaml.load((b.ROOT / ".github/workflows/ci.yml").read_text(), Loader=yaml.BaseLoader)
        job = value["jobs"]["k6-e2e"]
        self.assertEqual(job["needs"], ["build-iam", "build-oauth", "build-ext-authz"])
        self.assertEqual(job["uses"], "./.github/workflows/k6-e2e.yml")
        self.assertIn("'push'", job["if"])
        self.assertIn("refs/heads/master", job["if"])
        self.assertEqual(job["permissions"], dict(contents="read", actions="read"))
        self.assertEqual(value["jobs"]["sonar"]["needs"], ["merge-coverage", "clippy", "plan"])
        stage, pages = (value["jobs"][name] for name in ("e2e-pages-stage", "e2e-pages"))
        self.assertEqual(stage["needs"], ["k6-e2e"])
        self.assertEqual(stage["permissions"], dict(contents="read", actions="read"))
        self.assertNotIn("environment", stage)
        self.assertIn("always()", stage["if"])
        for job in (stage, pages):
            self.assertIn("'push'", job["if"])
            self.assertIn("refs/heads/master", job["if"])
        self.assertEqual(pages["needs"], ["e2e-pages-stage"])
        self.assertEqual(pages["permissions"], {"contents": "read", "actions": "read", "pages": "write", "id-token": "write"})
        self.assertEqual(pages["concurrency"], {"group": "e2e-github-pages", "cancel-in-progress": "false"})
        self.assertEqual(stage["steps"][1]["with"]["name"], "k6-e2e-report-${{ github.run_id }}-${{ github.run_attempt }}")
        self.assertEqual(stage["outputs"]["artifact_id"], "${{ steps.stage.outputs.artifact_id }}")
        artifact_name = "github-pages-${{ github.run_id }}-${{ github.run_attempt }}"
        self.assertEqual(stage["steps"][2]["with"]["name"], artifact_name)
        self.assertEqual(pages["env"]["PAGES_ARTIFACT_NAME"], artifact_name)
        visible, current, deploy = pages["steps"]
        self.assertEqual(visible["env"]["PAGES_ARTIFACT_ID"], "${{ needs.e2e-pages-stage.outputs.artifact_id }}")
        self.assertEqual(visible["env"]["STAGED_ATTEMPT"], "${{ needs.e2e-pages-stage.outputs.attempt }}")
        self.assertEqual(current["run"], stage["steps"][0]["run"])
        self.assertIn("steps.visible.outputs.ready == 'true'", current["if"])
        self.assertIn("steps.visible.outputs.ready == 'true' && steps.current.outputs.publish == 'true'", deploy["if"])
        self.assertEqual(deploy["with"]["artifact_name"], "${{ env.PAGES_ARTIFACT_NAME }}")
        self.assertNotIn("continue-on-error", deploy)
        self.assertNotIn("continue-on-error", pages)

    def test_reusable_manual_split_and_readme_badges_do_not_use_fake_static_results(self):
        value = yaml.load((b.ROOT / ".github/workflows/k6-e2e.yml").read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(set(value["on"]), {"workflow_call", "workflow_dispatch"})
        steps = value["jobs"]["e2e"]["steps"]
        automatic = [step for step in steps if "artifacts.py" in step.get("run", "")]
        self.assertEqual(len(automatic), 1)
        self.assertIn('"$GITHUB_RUN_ID"', automatic[0]["run"])
        self.assertIn('"$GITHUB_SHA"', automatic[0]["run"])
        packaging = next(i for i, step in enumerate(steps) if "ci/package.py" in step.get("run", ""))
        admission = next(i for i, step in enumerate(steps) if "--packaged-directory" in step.get("run", ""))
        self.assertIn("ci/cleanup.py", steps[packaging + 1]["run"])
        self.assertLess(packaging + 1, admission)
        self.assertIn("always()", steps[packaging + 1]["if"])
        for step in steps:
            self.assertNotIn("${{", step.get("run", ""))
        readme = (b.ROOT / "README.md").read_text()
        for metric in ("alert_status", "coverage", "security_rating", "reliability_rating", "sqale_rating", "duplicated_lines_density"):
            self.assertIn("project=Djoe-Denne_IAMRusty&metric=" + metric, readme)
        self.assertIn("img.shields.io/endpoint?url=", readme)
        self.assertNotIn("img.shields.io/badge/E2E-34", readme)


@unittest.skipUnless(BASH, "existing Bash required; set AIFORALL_BASH explicitly on Windows")
class PagesArtifactTests(unittest.TestCase):
    ARTIFACT = "42\tgithub-pages-123-1\tfalse\n"

    def run_step(self, step, stub, *, listings=(), **overrides):
        workflow = yaml.load((b.ROOT / ".github/workflows/ci.yml").read_text(), Loader=yaml.BaseLoader)
        command = next(cmd["run"] for cmd in workflow["jobs"]["e2e-pages"]["steps"] if cmd.get("id") == step)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for i, row in enumerate(listings, 1):
                (root / str(i)).write_text(row, encoding="utf-8", newline="\n")
            env = dict(os.environ, GH_TOKEN="", BASH_ENV="", ENV="",
                       GITHUB_OUTPUT=str(root / "output"), GITHUB_REPOSITORY="fixture/repo", GITHUB_RUN_ID="123",
                       GITHUB_RUN_ATTEMPT="1", GITHUB_SHA=SHA, PAGES_ARTIFACT_NAME="github-pages-123-1",
                       PAGES_ARTIFACT_ID="42", STAGED_ATTEMPT="1", FIXTURE=str(root))
            env.update(overrides)
            result = subprocess.run([BASH, "--noprofile", "--norc", "-eo", "pipefail", "-c", stub + "\n" + command],
                                    env=env, capture_output=True, text=True, timeout=12)
            recorded = [path.read_text(encoding="utf-8") if path.is_file() else ""
                        for path in (root / "output", root / "calls", root / "delays")]
            return result, *recorded

    def visibility(self, listings, **env):
        return self.run_step("visible", r"""
gh() {
    [[ "$*" == "api --method GET repos/fixture/repo/actions/runs/123/artifacts -f name=github-pages-123-1 --jq .artifacts[] | [.id, .name, .expired] | @tsv" ]] || return 64
    printf x >> "$FIXTURE/calls"
    cat "$FIXTURE/$(wc -c < "$FIXTURE/calls")"
}
sleep() { printf '%s\n' "$1" >> "$FIXTURE/delays"; }
""", listings=listings, **env)

    def test_delayed_visibility_unblocks_only_the_exact_upload(self):
        result, output, calls, delays = self.visibility(("", "", self.ARTIFACT))
        self.assertEqual((result.returncode, output, calls, delays), (0, "ready=true\n", "xxx", "2\n4\n"))
        self.assertIn("Pages artifact 42 is visible.", result.stdout)

    def test_missing_artifact_has_a_five_lookup_budget(self):
        result, output, calls, delays = self.visibility(("",) * 5)
        self.assertEqual((result.returncode, output, calls, delays), (1, "", "xxxxx", "2\n4\n8\n16\n"))
        self.assertIn("still missing after five lookups", result.stdout)

    def test_mismatched_expired_or_ambiguous_upload_is_not_retried(self):
        for row in (self.ARTIFACT.replace("42", "43"), self.ARTIFACT.replace("false", "true"), self.ARTIFACT * 2):
            with self.subTest(row=row):
                result, output, calls, delays = self.visibility((row,))
                self.assertEqual((result.returncode, output, calls, delays), (1, "", "x", ""))
                self.assertIn("does not match the staged identity", result.stdout)

    def test_an_old_attempt_or_missing_upload_receipt_never_looks_up_artifacts(self):
        for env in (dict(STAGED_ATTEMPT="0"), dict(PAGES_ARTIFACT_ID="")):
            with self.subTest(env=env):
                result, output, calls, delays = self.visibility((), **env)
                self.assertEqual((result.returncode, output, calls, delays), (1, "", "", ""))

    def test_visibility_cannot_override_a_stale_commit_or_attempt(self):
        stub = r"""
gh() {
    case "$2" in
        */git/ref/heads/master) printf '%s\n' "$MASTER_SHA" ;;
        */actions/runs/123) printf '%s\n' "$LATEST_ATTEMPT" ;;
        *) return 64 ;;
    esac
}
"""
        for master, attempt, expected in ((SHA, "1", "publish=true\n"), (H, "1", ""), (SHA, "2", "")):
            with self.subTest(master=master, attempt=attempt):
                result, output, _, _ = self.run_step("current", stub, MASTER_SHA=master, LATEST_ATTEMPT=attempt)
                self.assertEqual((result.returncode, output), (0, expected))


if __name__ == "__main__":
    unittest.main()
