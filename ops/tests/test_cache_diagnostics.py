"""Credential-safety and classification checks for hosted cache diagnostics."""

import json
from pathlib import Path
import sys
import unittest

import yaml


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ops/ci"))
import cache_diagnostics as diagnostics


class CacheDiagnosticsTests(unittest.TestCase):
    def test_multiline_backend_errors_are_classified_without_raw_content(self):
        lines = [
            "[2026-11-18T14:00:00Z ERROR sccache::compiler::compiler] [crate]: Cache read error: "
            "RateLimited (temporary) at read => http response error\n",
            "Context: service: ghac; status: 429\n",
            "headers: authorization: Bearer SUPER_SECRET_TOKEN\n",
            "url: https://private.example/cache?sig=SIGNED_SECRET\n",
            "Too many requests; retries exhausted\n",
            "[2026-11-18T14:00:01Z DEBUG sccache::server] Error executing cache write: "
            "AlreadyExists at write => status: 409\n",
            "cache already exists; path: PRIVATE_CACHE_KEY\n",
            "[2026-11-18T14:00:02Z DEBUG sccache::server] unrelated request\n",
        ]
        summary = diagnostics.summarize(lines)
        self.assertEqual(summary["events"], 2)
        self.assertEqual(summary["counts"]["lookup.http.429"], 1)
        self.assertEqual(summary["counts"]["lookup.kind.RateLimited"], 1)
        self.assertEqual(summary["counts"]["write.http.409"], 1)
        self.assertEqual(summary["counts"]["write.signal.reservation_conflict"], 1)
        encoded = json.dumps(summary)
        for secret in ("SUPER_SECRET_TOKEN", "SIGNED_SECRET", "private.example", "PRIVATE_CACHE_KEY"):
            self.assertNotIn(secret, encoded)

    def test_status_occurrences_count_once_per_event_and_unknown_errors_stay_visible(self):
        summary = diagnostics.summarize([
            "[2026-11-18T14:00:00Z DEBUG sccache::server] Error executing cache write: "
            "status: 503; status_code=503\n",
            "[2026-11-18T14:00:01Z ERROR sccache::compiler::compiler] Cache read error: "
            "unclassified secret content\n",
        ])
        self.assertEqual(summary["counts"]["write.http.503"], 1)
        self.assertEqual(summary["counts"]["lookup.events"], 1)
        self.assertNotIn("secret", json.dumps(summary))
        self.assertEqual(diagnostics.summarize(())["events"], 0)

    def test_oversized_event_is_bounded_without_dropping_later_events(self):
        summary = diagnostics.summarize([
            "Cache read error: Unexpected at read\n",
            "x" * 100000 + "\n",
            "x\n",
            "[2026-11-18T14:00:01Z DEBUG sccache::server] Error executing cache write: "
            "PermissionDenied at write: status: 403\n",
        ])
        self.assertEqual(summary["events"], 2)
        self.assertEqual(summary["truncated_events"], 1)
        self.assertEqual(summary["counts"]["write.http.403"], 1)

    def test_only_iam_captures_raw_log_and_only_summary_is_published(self):
        workflow = yaml.load((ROOT / ".github/workflows/ci.yml").read_text(), Loader=yaml.BaseLoader)
        jobs = workflow["jobs"]
        self.assertEqual([name for name, job in jobs.items()
                          if "SCCACHE_LOG" in job.get("env", {})], ["build-iam"])
        iam = jobs["build-iam"]
        self.assertIn("sccache::server=debug", iam["env"]["SCCACHE_LOG"])
        self.assertNotEqual(iam["env"]["SCCACHE_LOG"], "debug")
        private_logs = [step for step in iam["steps"] if "SCCACHE_ERROR_LOG=" in step.get("run", "")]
        self.assertEqual(len(private_logs), 1)
        self.assertIn("$RUNNER_TEMP/iam-sccache-private.log", private_logs[0]["run"])
        summaries = [step for step in iam["steps"] if "cache_diagnostics.py" in step.get("run", "")]
        self.assertEqual(len(summaries), 1)
        self.assertEqual(summaries[0]["if"], "always()")
        uploads = [step for step in iam["steps"] if step.get("with", {}).get("name") == "iam-cache-diagnostics"]
        self.assertEqual(len(uploads), 1)
        self.assertTrue(uploads[0]["with"]["path"].endswith("/iam-cache-diagnostics.json"))
        self.assertNotIn(".log", uploads[0]["with"]["path"])


if __name__ == "__main__":
    unittest.main()
