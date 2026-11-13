"""Workflow wiring contracts, using the repository's existing PyYAML dependency."""
import json
from pathlib import Path
import re
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[4]


def workflow(name):
    return yaml.load((ROOT / ".github/workflows" / name).read_text(encoding="utf-8"), Loader=yaml.BaseLoader)


class WorkflowTests(unittest.TestCase):
    def test_producer_reuses_exactly_one_existing_build_per_role(self):
        value = workflow("ci.yml")["jobs"]["build"]
        self.assertEqual(value["runs-on"], "ubuntu-24.04")
        roles = {item["e2e_role"]: item for item in value["strategy"]["matrix"]["include"] if "e2e_role" in item}
        self.assertEqual(set(roles), {"iam", "oauth", "ext-authz"})
        self.assertIn("--package iam-migration", roles["iam"]["package_args"])
        self.assertEqual(roles["oauth"]["package_args"], "--package oauth-connect-service")
        commands = [step.get("run", "") for step in value["steps"]]
        self.assertEqual(sum("cargo build " in command for command in commands), 1)
        collect = next(step for step in value["steps"] if "binaries.py" in step.get("run", ""))
        self.assertIn("github.event_name == 'push'", collect["if"])
        self.assertIn("refs/heads/master", collect["if"])
        artifact = next(step for step in value["steps"] if "upload-artifact@" in step.get("uses", ""))
        self.assertEqual(artifact["with"]["retention-days"], "7")
        self.assertEqual(artifact["with"]["if-no-files-found"], "error")
        self.assertEqual(artifact["with"]["compression-level"], "0")
        self.assertIn("github.run_attempt", artifact["with"]["name"])
        self.assertTrue(artifact["with"]["path"].endswith("/bundle.tar.gz"))

    def test_manual_consumer_cannot_restart_coverage_or_build(self):
        value = workflow("k6-images.yml")
        self.assertEqual(set(value["on"]), {"workflow_dispatch"})
        self.assertEqual(value["permissions"], {"contents": "read", "actions": "read"})
        inputs = value["on"]["workflow_dispatch"]["inputs"]
        self.assertEqual(set(inputs), {"producer_run_id", "producer_attempt", "target_sha"})
        job = value["jobs"]["package-images"]
        self.assertNotIn("needs", job)
        self.assertNotIn("continue-on-error", job)
        self.assertEqual(job["runs-on"], "ubuntu-24.04")
        for step in job["steps"]:
            self.assertNotIn("continue-on-error", step)
            if "uses" in step:
                self.assertRegex(step["uses"], r"^[A-Za-z0-9_./-]+@[a-f0-9]{40}$")
            if "run" in step:
                self.assertNotIn("${{", step["run"])
                self.assertNotIn("cargo ", step["run"])
            if step.get("uses", "").startswith("actions/checkout@"):
                self.assertEqual(step["with"]["persist-credentials"], "false")

    def test_only_allowlisted_delivery_outputs_are_uploaded(self):
        value = workflow("k6-images.yml")
        for step in value["jobs"]["package-images"]["steps"]:
            if not step.get("uses", "").startswith("actions/upload-artifact@"):
                continue
            paths = step["with"]["path"].splitlines()
            self.assertTrue(paths)
            for path in paths:
                self.assertIn(path, ["${{ runner.temp }}/k6-packaged/" + name for name in
                                    ("images.tar.gz", "provenance.json", "docker-resources.jsonl", "cleanup.json")])
            self.assertEqual(step["with"]["retention-days"], "7")

    def test_cancellation_has_independent_cleanup_before_success_publication(self):
        steps = workflow("k6-images.yml")["jobs"]["package-images"]["steps"]
        cleanup = next(index for index, step in enumerate(steps) if "cleanup.py" in step.get("run", ""))
        self.assertEqual(steps[cleanup]["if"], "${{ always() }}")
        uploads = [index for index, step in enumerate(steps) if step.get("uses", "").startswith("actions/upload-artifact@")]
        self.assertTrue(all(index > cleanup for index in uploads))
        self.assertIn("cancelled()", steps[uploads[-1]]["if"])

    def test_existing_packaging_recipes_keep_defaults_and_fixed_runtime_users(self):
        recipes = ["services/IAMRusty/Dockerfile", "services/OAuthConnect/Dockerfile",
                   "ops/deploy/build/Dockerfile.ext-authz"]
        for name in recipes:
            text = (ROOT / name).read_text(encoding="utf-8")
            self.assertIn("ARG RUNTIME_IMAGE=debian:bookworm-slim", text)
            self.assertIn("FROM ${RUNTIME_IMAGE} AS runtime", text)
            self.assertIn("RUNTIME_OPENSSL_PACKAGE", text)
            self.assertIn("COPY --from=", text)
            self.assertNotRegex(text, r"RUN[^\n]*cargo")
            self.assertIn("USER 65534:65534" if "ext-authz" in name else "USER 1000:1000", text)
        oauth = (ROOT / recipes[1]).read_text(encoding="utf-8")
        self.assertIn("getent passwd 1000", oauth)
        self.assertNotIn("userdel", oauth)


if __name__ == "__main__":
    unittest.main()
