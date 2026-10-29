"""Tests of the runner, not evidence of an HTTP target or Chromium."""
import copy
import importlib.util
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "lib"))
SPEC = importlib.util.spec_from_file_location("k6_runner", ROOT / "run-linux.py")
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


def summary(phase="HTTP"):
    checks = runner.CHECKS[phase]
    return {"version": 1, "run_id": "a" * 32, "phase": phase,
            "cases": [runner.CASES[phase]], "expected": 1, "started": 1,
            "completed": 1, "iterations": 1, "requests": 1 if phase == "HTTP" else 0,
            "checks": {name: {"passes": 1, "fails": 0} for name in checks}}


class SummaryTests(unittest.TestCase):
    def test_exact_current_summary(self):
        runner.verify_summary(summary(), "a" * 32, "HTTP")

    def test_missing_metrics_never_become_zero(self):
        for key in summary():
            value = summary()
            del value[key]
            with self.subTest(key=key), self.assertRaises(runner.Rejected):
                runner.verify_summary(value, "a" * 32, "HTTP")

    def test_stale_empty_extra_and_nonfinite_rejected(self):
        for key, value in [("run_id", "b" * 32), ("cases", []),
                           ("requests", 0), ("completed", 0), ("started", True),
                           ("iterations", float("nan")), ("expected", 2)]:
            data = summary()
            data[key] = value
            with self.subTest(key=key), self.assertRaises(runner.Rejected):
                runner.verify_summary(data, "a" * 32, "HTTP")
        data = summary()
        data["extra"] = 1
        with self.assertRaises(runner.Rejected):
            runner.verify_summary(data, "a" * 32, "HTTP")

    def test_each_check_required_exactly_once(self):
        for replacement in [{}, {"passes": 0, "fails": 0},
                            {"passes": 1, "fails": 1}, {"passes": 2, "fails": 0},
                            {"passes": True, "fails": 0}]:
            data = summary()
            data["checks"]["iam-status"] = replacement
            with self.assertRaises(runner.Rejected):
                runner.verify_summary(data, "a" * 32, "HTTP")

    def test_browser_is_distinct_and_has_no_http(self):
        runner.verify_summary(summary("Chromium"), "a" * 32, "Chromium")
        with self.assertRaises(runner.Rejected):
            runner.verify_summary(summary("Chromium"), "a" * 32, "HTTP")

    def test_duplicate_truncated_and_nan_json(self):
        for raw in [b'{"version":1,"version":1}', b'{', b'{"a":NaN}', b'\xff']:
            with self.subTest(raw=raw), self.assertRaises(runner.Rejected):
                runner.strict_json(raw)


class InputAndPrivacyTests(unittest.TestCase):
    def test_schema_fixed_target_and_missing_input(self):
        good = {"version": 1, "profile": "local-full", "cluster": "aiforall-local-full",
                "context": "kind-aiforall-local-full", "task_id": "c" * 32,
                "target_id": "d" * 32, "origin": "http://127.0.0.1:18080",
                "lease": "/private/lease.json", "observations": "/private/observations.json"}
        runner.config_schema(good)
        for field in good:
            data = copy.deepcopy(good)
            del data[field]
            with self.subTest(field=field), self.assertRaises(runner.Rejected):
                runner.config_schema(data)
        for origin in ["https://foreign.example", "http://127.0.0.1:18080/",
                       "http://user:secret@127.0.0.1:18080", "http://127.0.0.1:18080?code=secret"]:
            data = {**good, "origin": origin}
            with self.assertRaises(runner.Rejected):
                runner.config_schema(data)

    def test_closed_child_environment(self):
        ambient = {"K6_OUT": "cloud", "K6_HTTP_DEBUG": "full", "HTTPS_PROXY": "secret",
                   "ALL_PROXY": "secret", "LD_PRELOAD": "secret", "NODE_OPTIONS": "secret",
                   "KUBECONFIG": "secret", "HOME": "foreign"}
        env = runner.child_environment(Path("/private/run"), ambient)
        self.assertFalse(set(ambient) & set(env) - {"HOME"})
        self.assertEqual(env["HOME"], str(Path("/private/run") / "home"))
        self.assertEqual(env["K6_NO_USAGE_REPORT"], "true")

    def test_sentinels_and_secret_shapes_on_every_surface(self):
        for payload in [b"CONFIDENTIAL_SENTINEL_test", b"https://example.invalid/?code=abc",
                        b"Authorization: Bearer abcd", b"cookie=abc", b"verifier=abc",
                        b"eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJhIn0.signature"]:
            for surface in ["stdout", "stderr", "summary", "result", "provenance"]:
                with self.subTest(surface=surface), self.assertRaises(runner.Rejected):
                    runner.confidential(payload)

    def test_failure_messages_are_closed_codes(self):
        error = runner.Rejected("INPUT_INVALID")
        self.assertEqual(str(error), "INPUT_INVALID")
        with self.assertRaises(ValueError):
            runner.Rejected("https://secret.invalid")


if __name__ == "__main__":
    unittest.main()
