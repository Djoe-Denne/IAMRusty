"""Automatic trusted-CI admission and fail-closed public test reporting."""
import copy
import hashlib
import io
import json
from pathlib import Path
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


def dump(root, name, value):
    (root / name).write_text(json.dumps(value), encoding="utf-8")


def public_fixture(root):
    root.mkdir()
    dump(root, "report.json", dict(run_id=RUN, verdict="PASS", cleanup="PASS", restoration="PASS", completed_cases=list(e.CASES)))
    dump(root, "provenance.json", dict(target_sha=SHA, harness_sha=H))
    dump(root, "cleanup.json", dict(status="PASS", owned_running=0, errors=[]))
    dump(root, "correlation.json", r.CORRELATION)
    dump(root, "processes.json", {case: dict(exit=0, cleanup=True) for case in e.CASES})
    dump(root, "browser.json", dict(result=browser_result(), container_exit=0, cleanup="PASS"))
    for case, (phase, checks, requests) in e.CASES.items():
        dump(root, case + ".json", dict(version=1, run_id=RUN, phase=phase, cases=[case], expected=1,
            started=1, completed=1, iterations=1, requests=requests, checks={name: dict(passes=1, fails=0) for name in checks}))


def project(root, target=None):
    return r.project(root / "source", root / "site", "Djoe-Denne/IAMRusty", "123", "1", H, target or SHA)


class ReportingTests(unittest.TestCase):
    def test_34_checks_and_distinct_parent_contract_are_not_code_coverage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            public_fixture(root / "source")
            metric = project(root)
            self.assertEqual((metric["status"], metric["passed"], metric["expected"]), ("passed", 34, 34))
            files = list((root / "site/allure-results").glob("*-result.json"))
            self.assertEqual(len(files), 35)
            for file in files:
                value = json.loads(file.read_text())
                self.assertEqual(value["status"], "passed")
                self.assertNotIn("start", value)
                self.assertNotIn("stop", value)
                self.assertNotIn("attachments", value)
            self.assertEqual(json.loads((root / "site/badge.json").read_text())["message"], "34/34")

    def test_parent_failure_cannot_leave_green_badge_even_with_34_passed_checks(self):
        for name, change in (("cleanup.json", dict(status="FAIL")), ("cleanup.json", dict(owned_running=False)),
                             ("correlation.json", dict(iam_delta=8)), ("report.json", dict(verdict="FAIL"))):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                public_fixture(root / "source")
                value = json.loads((root / "source" / name).read_text())
                dump(root / "source", name, {**value, **change})
                metric = project(root)
                self.assertNotEqual(metric["status"], "passed")
                self.assertEqual(metric["passed"], 34)
                self.assertEqual(json.loads((root / "site/badge.json").read_text())["color"], "red")

    def test_missing_or_stale_phase_is_unproven_not_a_fake_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            public_fixture(root / "source")
            (root / "source/auth-oauth-browser.json").unlink()
            metric = project(root)
            self.assertEqual((metric["passed"], metric["unproven"]), (8, 26))
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
            self.assertEqual((metric["status"], metric["passed"], metric["unproven"]), ("incomplete", 0, 34))
            self.assertTrue((root / "site/index.html").is_file())


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
        self.assertEqual(job["needs"], ["build"])
        self.assertEqual(job["uses"], "./.github/workflows/k6-e2e.yml")
        self.assertIn("'push'", job["if"])
        self.assertIn("refs/heads/master", job["if"])
        self.assertEqual(job["permissions"], dict(contents="read", actions="read"))
        self.assertEqual(value["jobs"]["sonar"]["needs"], ["merge-coverage"])
        pages = value["jobs"]["e2e-pages"]
        self.assertEqual(pages["needs"], ["k6-e2e"])
        self.assertEqual(pages["permissions"]["pages"], "write")
        self.assertIn("always()", pages["if"])
        self.assertIn('"$current" = "$GITHUB_SHA"', pages["steps"][0]["run"])

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


if __name__ == "__main__":
    unittest.main()
