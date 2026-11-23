"""Exercise the shipped k6 entries and an owned in-process fixture API."""
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
from threading import Thread
import unittest


HERE = Path(__file__).resolve().parents[1]
K6 = os.environ.get("AIFORALL_K6") or shutil.which("k6")
RUN = "0" * 32
CASE = "fixture-probe"
SOURCE = """import http from 'k6/http';
import { scenarioContract } from '../lib/case-contract.js';

const contract = scenarioContract('fixture-probe');
const url = __ENV.K6_ORIGIN + '/fixtures/' + __ENV.K6_RUN_ID;
const mode = __ENV.PROBE_MODE;

export const options = {
  scenarios: { 'fixture-probe': { executor: 'per-vu-iterations', vus: 1, iterations: 1 } },
  systemTags: ['check', 'scenario', 'group'],
  thresholds: contract.thresholds,
  maxRedirects: 0,
};

export function setup() {
  try {
    const created = http.post(url, null);
    if (!contract.check(created, { created: (r) => r.status === 201 })) {
      throw new Error('FIXTURE_CREATE_FAILED');
    }
    if (mode === 'setup-fail') throw new Error('INJECTED_SETUP_FAILURE');
    return { url };
  } catch (error) {
    // The ownership ID is known even if the creation response was lost.
    http.del(url);
    throw error;
  }
}

export default function (data) {
  contract.start();
  const read = http.get(data.url);
  if (mode !== 'missing') {
    const predicate = { 'read-back': (r) => r.status === (mode === 'false' ? 404 : 200) };
    contract.check(read, predicate);
    if (mode === 'duplicate') contract.check(read, predicate);
  }
  contract.finish();
  if (mode === 'undeclared') contract.check(true, { undeclared: (value) => value });
}

export function teardown(data) {
  contract.check(http.del(data.url), { removed: (r) => r.status === 204 });
}

export function handleSummary(data) { return contract.summary(data); }
"""


class Api(BaseHTTPRequestHandler):
    def handle_request(self):
        self.server.requests.append(self.command)
        if self.path != "/fixtures/" + RUN:
            self.send_error(404)
            return
        if self.command == "POST":
            self.server.exists = True
            code = 201
        elif self.command == "GET":
            code = 200 if self.server.exists else 404
        elif self.server.fail_cleanup:
            code = 503
        else:
            self.server.exists = False
            code = 204
        self.send_response(code)
        self.end_headers()

    do_POST = handle_request
    do_GET = handle_request
    do_DELETE = handle_request

    def log_message(self, *_args):
        pass


@unittest.skipUnless(K6 and Path(K6).is_file(), "preinstalled k6 required for the browser-free contract tests")
class K6ContractTests(unittest.TestCase):
    def probe(self, mode, expected_diagnostic=None):
        helper = HERE / "lib/case-contract.js"
        self.assertTrue(helper.is_file(), "STORY-020 requires the reusable k6 scenario contract")
        with tempfile.TemporaryDirectory() as directory, HTTPServer(("127.0.0.1", 0), Api) as server:
            root = Path(directory)
            (root / "lib").mkdir()
            (root / "scenarios").mkdir()
            shutil.copyfile(helper, root / "lib/case-contract.js")
            shutil.copyfile(HERE / "lib/summary.js", root / "lib/summary.js")
            catalogue = {"version": 1, "cases": [
                {"id": CASE, "script": f"{CASE}.js", "phase": "HTTP", "lifecycle": "standard",
                 "timeout_seconds": 30, "requests": 3,
                 "checks": {"created": "functional", "read-back": "functional", "removed": "cleanup"}},
            ]}
            (root / "scenarios/catalog.json").write_text(json.dumps(catalogue), encoding="utf-8")
            script = root / "scenarios" / f"{CASE}.js"
            script.write_text(SOURCE, encoding="utf-8")
            config = root / "options.json"
            config.write_text("{}", encoding="utf-8")
            summary = root / "summary.json"
            env = {key: value for key, value in os.environ.items()
                   if key.upper() in {"SYSTEMROOT", "WINDIR", "TEMP", "TMP", "HOME", "PATH"}}
            env.update(K6_RUN_ID=RUN, K6_ORIGIN=f"http://127.0.0.1:{server.server_port}",
                       K6_SUMMARY_PATH=str(summary), PROBE_MODE=mode)
            server.exists = False
            server.requests = []
            server.fail_cleanup = mode == "cleanup-fail"
            thread = Thread(target=server.serve_forever, kwargs={"poll_interval": 0.05}, daemon=True)
            thread.start()
            try:
                run = subprocess.run(
                    [str(K6), "run", "--no-usage-report", "--quiet", "--address", "127.0.0.1:0",
                     "--config", str(config), str(script)], env=env, capture_output=True, timeout=15,
                )
            finally:
                server.shutdown()
                thread.join(timeout=2)
            self.assertFalse(thread.is_alive(), "owned synthetic API did not stop")
            diagnostic = run.stderr.decode("utf-8", errors="backslashreplace")
            if expected_diagnostic:
                self.assertIn(expected_diagnostic, diagnostic)
            self.assertTrue(summary.is_file(), diagnostic)
            return run.returncode, json.loads(summary.read_text(encoding="utf-8")), server.requests, server.exists

    def test_direct_k6_sets_up_checks_and_removes_fixture(self):
        code, summary, requests, exists = self.probe("success")
        self.assertEqual(code, 0)
        self.assertEqual(requests, ["POST", "GET", "DELETE"])
        self.assertFalse(exists)
        self.assertEqual(summary["requests"], 3)
        self.assertEqual(summary["run_id"], RUN)
        self.assertEqual(summary["checks"], dict.fromkeys(
            ("created", "read-back", "removed"), {"passes": 1, "fails": 0}))

    def test_skipped_duplicate_false_and_cleanup_checks_all_fail(self):
        for mode, name, counts in (
            ("missing", "read-back", None), ("duplicate", "read-back", {"passes": 2, "fails": 0}),
            ("false", "read-back", {"passes": 0, "fails": 1}),
            ("cleanup-fail", "removed", {"passes": 0, "fails": 1}),
        ):
            with self.subTest(mode=mode):
                code, summary, requests, _exists = self.probe(mode)
                self.assertEqual(code, 99)
                self.assertEqual(summary["checks"].get(name), counts)
                self.assertEqual((summary["started"], summary["completed"], summary["iterations"]), (1, 1, 1))
                self.assertEqual(requests, ["POST", "GET", "DELETE"])

    def test_partial_setup_failure_compensates_without_teardown(self):
        code, summary, requests, exists = self.probe("setup-fail")
        self.assertNotEqual(code, 0)
        self.assertEqual(requests, ["POST", "DELETE"])
        self.assertFalse(exists)
        self.assertNotIn("removed", summary["checks"])

    def test_undeclared_assertion_cannot_pass_after_expected_assertions(self):
        code, summary, _requests, _exists = self.probe("undeclared", "CASE_CHECK_NOT_DECLARED")
        self.assertEqual(code, 108)
        self.assertNotIn("undeclared", summary["checks"])


@unittest.skipUnless(K6 and Path(K6).is_file(), "preinstalled k6 required for script inspection")
class ShippedScenarioTests(unittest.TestCase):
    target = dict(version=1, profile="iam-native-parity", run_id=RUN,
                  origin="https://127.0.0.1:18443", provider="https://localhost:18445/oauth/authorize",
                  issuer="https://127.0.0.1:18443/iam", audience="aiforall")

    def invoke(self, directory, args):
        config = directory / "options.json"
        config.write_text("{}", encoding="utf-8")
        env = {key: value for key, value in os.environ.items()
               if key.upper() in {"SYSTEMROOT", "WINDIR", "TEMP", "TMP", "HOME", "PATH"}}
        return subprocess.run([K6, *args[:1], "--config", str(config), *args[1:]],
                              env=env, capture_output=True, text=True, encoding="utf-8", timeout=20)

    def inspect(self, case, target):
        with tempfile.TemporaryDirectory() as scratch:
            directory = Path(scratch)
            path = directory / "target.json"
            path.write_text(json.dumps(target), encoding="utf-8")
            phase = case["id"].removeprefix("auth-envoy-san-") if case["lifecycle"] == "san" else ""
            return self.invoke(directory, ["inspect", "-e", f"K6_CAMPAIGN_FILE={path}", "-e", f"K6_RUN_ID={RUN}",
                                           "-e", f"K6_AUTH_SAN_PHASE={phase}", "-e", f"K6_CASE_ID={case['id']}",
                                           str(HERE / "scenarios" / case["script"])])

    def test_shipped_health_oauth_and_san_are_bounded_and_keep_named_check_tags(self):
        catalogue = json.loads((HERE / "scenarios/catalog.json").read_text(encoding="utf-8"))
        baseline = {"smoke/iam-health.js", "auth/oauth-browser.js", "auth/envoy-san.js"}
        for case in (item for item in catalogue["cases"] if item["script"] in baseline):
            with self.subTest(case=case["id"]):
                result = self.inspect(case, self.target)
                self.assertEqual(result.returncode, 0, result.stderr)
                options = json.loads(result.stdout)
                self.assertEqual(list(options["scenarios"]), [case["id"]])
                self.assertEqual(options["scenarios"][case["id"]]["vus"], 1)
                self.assertEqual(options["scenarios"][case["id"]]["iterations"], 1)
        for relative in sorted(baseline):
            source = (HERE / "scenarios" / relative).read_text(encoding="utf-8")
            tags = re.search(r"systemTags:\s*(\[[^\]]+\])", source).group(1)
            with self.subTest(script=relative), tempfile.TemporaryDirectory() as scratch:
                directory = Path(scratch)
                output = directory / "summary.json"
                script = directory / "probe.js"
                summary = json.dumps((HERE / "lib/summary.js").as_uri())
                script.write_text(
                    f"import {{started,completed,project}} from {summary};\n"
                    "import {check} from 'k6';\n"
                    f"export const options={{vus:1,iterations:1,systemTags:{tags}}};\n"
                    "export default function(){started.add(1);check(true,{pass:v=>v});"
                    "check(false,{fail:v=>v});completed.add(1);}\n"
                    "export function handleSummary(data){return project(data,'HTTP','probe');}\n", encoding="utf-8")
                result = self.invoke(directory, ["run", "--quiet", "--no-usage-report", "--address", "127.0.0.1:0",
                                                 "-e", f"K6_RUN_ID={RUN}", "-e", f"K6_SUMMARY_PATH={output}", str(script)])
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(json.loads(output.read_text(encoding="utf-8"))["checks"],
                                 {"pass": {"passes": 1, "fails": 0}, "fail": {"passes": 0, "fails": 1}})

    def test_refuses_insecure_foreign_malformed_or_stale_target_metadata(self):
        case = {"id": "iam-health", "script": "smoke/iam-health.js", "lifecycle": "standard"}
        for key, value in (("origin", "http://127.0.0.1:18443"), ("provider", "https://127.0.0.1:18445/oauth/authorize"),
                           ("issuer", "http://127.0.0.1:8080/iam"), ("audience", "foreign"), ("version", 2),
                           ("profile", "legacy"), ("run_id", "b" * 32), ("insecure", True)):
            with self.subTest(key=key):
                result = self.inspect(case, {**self.target, key: value})
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("NATIVE_CAMPAIGN_INVALID", result.stderr)
