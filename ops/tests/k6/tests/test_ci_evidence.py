"""Portable refusal tests for the CI receiver; never an E2E qualification."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[4]
SPEC = importlib.util.spec_from_file_location("ci_evidence", ROOT / "ops/tests/k6/ci/evidence.py")
e = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(e)
RUN = "a" * 32
K6 = Path(os.environ.get("AIFORALL_K6") or shutil.which("k6") or "C:/Program Files/k6/k6.exe")


def summary(case):
    phase, checks, requests = e.CASES[case]
    return dict(version=1, run_id=RUN, phase=phase, cases=[case], expected=1,
                started=1, completed=1, iterations=1, requests=requests,
                checks={name: {"passes": 1, "fails": 0} for name in checks})


def receive(value, case="iam-health", code=0):
    return e.accept_summary(json.dumps(value).encode(), RUN, case, code)


class SummaryTests(unittest.TestCase):
    def test_existing_stimuli_have_the_same_closed_checks(self):
        files = {"iam-health": "smoke/iam-health.js",
                 "auth-oauth-browser": "auth/oauth-browser.js",
                 "auth-envoy-san-good": "auth/envoy-san.js"}
        for case, path in files.items():
            source = (ROOT / "ops/tests/k6/scenarios" / path).read_text(encoding="utf-8")
            for name in e.CASES[case][1]:
                self.assertIn("'" + name + "'", source, (case, name))
            self.assertIn("'group'", source)
        auth = (ROOT / "ops/tests/k6/lib/windows-auth.ps1").read_text(encoding="utf-8")
        checks = re.search(r"Checks = @\((.*?)\)", auth, re.S).group(1)
        self.assertEqual(set(re.findall(r"'([^']+)'", checks)), e.OAUTH_CHECKS)
        self.assertEqual(sum(len(spec[1]) for spec in e.CASES.values()), 34)

    def test_all_five_phases_are_accepted(self):
        values = {case: summary(case) for case in e.CASES}
        results = {case: (json.dumps(value).encode(), 0) for case, value in values.items()}
        self.assertEqual(e.accept_campaign(results, RUN), values)
        for case in e.CASES:
            self.assertEqual(receive(values[case], case), values[case])
            results[case] = (json.dumps(values[case]).encode(), 1)
            with self.assertRaises(e.EvidenceError):
                e.accept_campaign(results, RUN)
            results[case] = (json.dumps(values[case]).encode(), 0)

    def test_no_vacuous_or_stale_summary(self):
        changes = {"run_id": "b" * 32, "version": True, "phase": "browser",
                   "cases": [], "expected": 0, "started": None, "completed": 0,
                   "iterations": 2, "requests": True, "checks": {}}
        for field, value in changes.items():
            with self.subTest(field=field), self.assertRaises(e.EvidenceError):
                data = summary("iam-health")
                data[field] = value
                receive(data)
        for field in e.SUMMARY_FIELDS:
            data = summary("iam-health")
            del data[field]
            with self.subTest(missing=field), self.assertRaises(e.EvidenceError):
                receive(data)

    def test_exit_code_and_case_are_not_inferred_from_a_summary(self):
        for code in (1, -15, True, None):
            with self.subTest(code=code), self.assertRaises(e.EvidenceError):
                receive(summary("iam-health"), code=code)
        with self.assertRaises(e.EvidenceError):
            receive(summary("iam-health"), case="unknown")

    def test_closed_checks_and_fields(self):
        for change in (lambda d: d.update(secret="not-for-publication"),
                       lambda d: d["checks"].update(unknown={"passes": 1, "fails": 0}),
                       lambda d: d["checks"]["iam-body"].update(passes=2),
                       lambda d: d["checks"]["iam-body"].update(fails=1),
                       lambda d: d["checks"]["iam-body"].update(passes=True),
                       lambda d: d["checks"]["iam-body"].update(extra=0)):
            data = summary("iam-health")
            change(data)
            with self.assertRaises(e.EvidenceError):
                receive(data)

    def test_invalid_duplicate_nonfinite_truncated_and_oversized_json(self):
        for raw in (b"", b"[]", b"{", b"{}garbage", b"\xff", b" " * 65537,
                    b'{"version":1,"version":1}', b'{"n":NaN}', b'{"n":Infinity}'):
            with self.subTest(raw=raw[:30]), self.assertRaises(e.EvidenceError):
                e.accept_summary(raw, RUN, "iam-health", 0)

    def test_campaign_rejects_missing_or_extra_phase(self):
        for case in e.CASES:
            values = {name: summary(name) for name in e.CASES if name != case}
            with self.assertRaises(e.EvidenceError):
                e.accept_campaign(values, RUN)
        values = {case: summary(case) for case in e.CASES}
        values["extra"] = summary("iam-health")
        with self.assertRaises(e.EvidenceError):
            e.accept_campaign(values, RUN)
        values = {case: summary(case) for case in e.CASES}
        with self.assertRaises(e.EvidenceError):
            e.accept_campaign(values, RUN)  # No implicit zero exit status.


def counters():
    value = {}
    for cluster in ("iam", "ext_authz"):
        for name in ("upstream_rq_total", "upstream_rq_5xx", "upstream_cx_connect_fail",
                     "upstream_cx_connect_timeout", "ssl.fail_verify_san", "ssl.fail_verify_error",
                     "ssl.handshake"):
            value[f"cluster.{cluster}.{name}"] = 10
    return value


class ParentTests(unittest.TestCase):
    def setUp(self):
        self.before = counters()
        self.after = counters()
        self.after["cluster.iam.upstream_rq_total"] += 7
        self.after["cluster.ext_authz.upstream_rq_total"] += 4
        self.provider = dict(run_id=RUN, authorize=0, token=0, profile=0, pkce_validated=0)
        self.provider_after = dict(run_id=RUN, authorize=3, token=3, profile=3, pkce_validated=3)
        self.db = dict(users=1, outbox=0)
        self.db_after = dict(users=2, outbox=0)

    def accept(self):
        e.accept_oauth(self.before, self.after, self.provider, self.provider_after,
                       self.db, self.db_after, RUN)

    def test_exact_parent_deltas(self):
        self.accept()

    def test_each_transport_or_cardinality_mutation_is_rejected(self):
        for name in self.after:
            if name.endswith("ssl.handshake") or name == "cluster.ext_authz.upstream_rq_5xx":
                continue
            with self.subTest(name=name):
                self.after[name] += 1
                with self.assertRaises(e.EvidenceError):
                    self.accept()
                self.after[name] -= 1
        for name in ("authorize", "token", "profile", "pkce_validated"):
            self.provider_after[name] -= 1
            with self.assertRaises(e.EvidenceError):
                self.accept()
            self.provider_after[name] += 1

    def test_stale_provider_missing_counter_and_database_effects(self):
        self.provider_after["run_id"] = "b" * 32
        with self.assertRaises(e.EvidenceError):
            self.accept()
        self.provider_after["run_id"] = RUN
        for field in ("users", "outbox"):
            self.db_after[field] += 1
            with self.assertRaises(e.EvidenceError):
                self.accept()
            self.db_after[field] -= 1
        del self.after["cluster.iam.upstream_cx_connect_fail"]
        with self.assertRaises(e.EvidenceError):
            self.accept()

    def test_counter_reset_boolean_and_absence_are_not_zero(self):
        for before, after in (({}, {}), ({"n": 1}, {"n": 0}),
                              ({"n": True}, {"n": 2}), ({"n": 0}, {"n": 1.0})):
            with self.assertRaises(e.EvidenceError):
                e.delta(before, after, "n")

    def test_san_requires_same_incarnation_cause_and_restoration(self):
        good = counters()
        bad = counters()
        bad["cluster.iam.ssl.fail_verify_san"] += 1
        restored = copy.deepcopy(bad)
        restored["cluster.iam.ssl.handshake"] += 1
        e.accept_san(good, bad, restored, ["pod-uid/restart-0"] * 3)
        for ids in (["old", "new", "new"], ["", "", ""], ["uid", "uid"]):
            with self.assertRaises(e.EvidenceError):
                e.accept_san(good, bad, restored, ids)
        for name in ("cluster.iam.ssl.fail_verify_san", "cluster.iam.ssl.fail_verify_error"):
            restored[name] += 1
            with self.assertRaises(e.EvidenceError):
                e.accept_san(good, bad, restored, ["uid"] * 3)
            restored[name] -= 1
        with self.assertRaises(e.EvidenceError):
            e.accept_san(good, bad, bad, ["uid"] * 3)


@unittest.skipUnless(K6.is_file(), "existing pinned k6 required; not browser qualification")
class ProjectorTests(unittest.TestCase):
    def test_real_projector_output_is_consumable_and_failed_check_is_refused(self):
        source = (ROOT / "ops/tests/k6/scenarios/auth/oauth-browser.js").read_text(encoding="utf-8")
        tags = re.search(r"systemTags:\s*(\[[^\]]+\])", source).group(1)
        for passed in (True, False):
            with self.subTest(passed=passed), tempfile.TemporaryDirectory(prefix="k6-ci-project-") as directory:
                path = Path(directory)
                output = path / "summary.json"
                script = (
                    "import { check } from 'k6';\n"
                    "import { started, completed, project } from "
                    + json.dumps((ROOT / "ops/tests/k6/lib/summary.js").as_uri()) + ";\n"
                    "export const options={vus:1,iterations:1,systemTags:" + tags + "};\n"
                    "export default function(){started.add(1);"
                    + "".join("check(" + str(passed).lower() + ",{" + json.dumps(name) + ":v=>v});"
                              for name in sorted(e.OAUTH_CHECKS))
                    + "completed.add(1);}\n"
                    "export function handleSummary(d){return project(d,'Chromium','auth-oauth-browser');}\n")
                (path / "probe.js").write_text(script, encoding="utf-8")
                (path / "options.json").write_text("{}", encoding="utf-8")
                env = {key: value for key, value in os.environ.items()
                       if key.upper() in {"SYSTEMROOT", "WINDIR", "TEMP", "TMP", "HOME", "PATH"}}
                result = subprocess.run([str(K6), "run", "--quiet", "--no-usage-report", "--config",
                                         str(path / "options.json"), "-e", "K6_RUN_ID=" + RUN,
                                         "-e", "K6_SUMMARY_PATH=" + str(output), str(path / "probe.js")],
                                        env=env, capture_output=True, timeout=20)
                self.assertEqual(result.returncode, 0, "projector probe failed (private output omitted)")
                raw = output.read_bytes()
                if passed:
                    e.accept_summary(raw, RUN, "auth-oauth-browser", result.returncode)
                else:
                    with self.assertRaises(e.EvidenceError):
                        e.accept_summary(raw, RUN, "auth-oauth-browser", result.returncode)


if __name__ == "__main__":
    unittest.main()
