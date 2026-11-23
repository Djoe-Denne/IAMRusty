"""The opt-in PR lane never admits fork code or images from another source."""
import copy
from pathlib import Path
import sys
import unittest
import yaml


CI = Path(__file__).resolve().parents[1] / "ci"
sys.path.insert(0, str(CI))
from binaries import Refused
import runtime


REPO = "test/AIForAll"
MERGE, BASE, HEAD = "a" * 40, "b" * 40, "c" * 40
REF = "refs/pull/42/merge"


def environment():
    return {
        "GITHUB_ACTIONS": "true", "GITHUB_JOB": "e2e", "GITHUB_EVENT_NAME": "pull_request",
        "GITHUB_REPOSITORY": REPO, "GITHUB_REPOSITORY_ID": "123",
        "GITHUB_WORKFLOW_REF": f"{REPO}/.github/workflows/k6-e2e-pr.yml@{REF}",
        "GITHUB_REF": REF, "GITHUB_SHA": MERGE, "RUNNER_ENVIRONMENT": "github-hosted",
    }


def event():
    repo = {"id": 123, "full_name": REPO}
    return {"action": "labeled", "number": 42, "label": {"name": "k6-e2e:456:2"},
            "repository": repo.copy(), "pull_request": {
                "state": "open", "base": {"ref": "master", "sha": BASE, "repo": repo.copy()},
                "head": {"sha": HEAD, "repo": repo.copy()},
            }}


class PullRequestQualificationTests(unittest.TestCase):
    def admit(self, payload, env=None, commit=MERGE, parents=(BASE, HEAD), paths=None):
        try:
            from pr_qualification import admit
        except ModuleNotFoundError as error:
            if error.name != "pr_qualification":
                raise
            self.fail("the dedicated k6 PR qualification admission is missing")
        return admit(payload, environment() if env is None else env, commit, parents,
                     ["ops/tests/k6/scenarios/catalog.json"] if paths is None else paths)

    def test_uses_exact_merge_and_base_with_explicit_packaging_run(self):
        self.assertEqual(self.admit(event()), ("456", "2", BASE))

    def test_checks_fork_label_producer_and_job_scope(self):
        mutations = [
            (("action",), "synchronize"),
            (("label", "name"), "k6-e2e:456:0"),
            (("label", "name"), "k6-e2e:456:2; other"),
            (("pull_request", "head", "repo", "id"), 456),
            (("pull_request", "base", "ref"), "other"),
        ]
        for keys, replacement in mutations:
            with self.subTest(keys=keys, replacement=replacement):
                payload = copy.deepcopy(event())
                target = payload
                for key in keys[:-1]:
                    target = target[key]
                target[keys[-1]] = replacement
                with self.assertRaises(Refused):
                    self.admit(payload)
        for key, replacement in [
            ("GITHUB_WORKFLOW_REF", f"{REPO}/.github/workflows/ci.yml@{REF}"),
            ("GITHUB_REF", "refs/heads/master"),
            ("GITHUB_EVENT_NAME", "pull_request_target"),
            ("RUNNER_ENVIRONMENT", "self-hosted"),
        ]:
            with self.subTest(key=key), self.assertRaises(Refused):
                self.admit(event(), env={**environment(), key: replacement})

    def test_rejects_other_revisions_and_app_changes(self):
        for kwargs in [
            {"commit": HEAD}, {"parents": (HEAD, BASE)}, {"parents": (BASE,)},
            {"paths": []}, {"paths": ["ops/tests/k6/../../Cargo.lock"]},
            {"paths": ["ops/tests/k6/lib/summary.js", "services/IAMRusty/src/main.rs"]},
            {"paths": [".github/actions/build-service/action.yml"]},
            {"paths": ["ops/tests/k6/ci/Dockerfile.artifacts"]},
            {"paths": ["ops/tests/k6/ci/images.lock.json"]},
            {"paths": ["ops/tests/k6/ci/package.py"]},
        ]:
            with self.subTest(kwargs=kwargs), self.assertRaises(Refused):
                self.admit(event(), **kwargs)

    def test_runtime_does_not_admit_arbitrary_pr_jobs(self):
        matcher = getattr(runtime, "premerge_job", None)
        self.assertTrue(callable(matcher), "the retained runtime needs its own PR job admission")
        self.assertTrue(matcher(environment()))
        self.assertFalse(matcher({**environment(), "GITHUB_JOB": "lint"}))
        self.assertFalse(matcher({**environment(), "GITHUB_REF": "refs/heads/arbitrary"}))

    def test_opt_in_is_separate_read_only_and_reuses_the_actual_campaign(self):
        root = CI.parents[3] / ".github/workflows"
        wrapper = yaml.load((root / "k6-e2e-pr.yml").read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(wrapper["on"], {"pull_request": {"types": ["labeled"], "branches": ["master"]}})
        self.assertEqual(wrapper["permissions"], {"actions": "read", "contents": "read"})
        job = wrapper["jobs"]["e2e"]
        self.assertIn("head.repo.full_name", job["if"])
        self.assertIn("github.event.label.name", job["if"])
        self.assertNotIn("secrets", job)
        self.assertEqual(job["uses"], "./.github/workflows/k6-e2e.yml")
        self.assertEqual(job["with"], {"pr_qualification": "true"})
        reused = yaml.load((root / "k6-e2e.yml").read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(set(reused["on"]), {"workflow_call", "workflow_dispatch"})
        self.assertEqual(reused["on"]["workflow_call"]["inputs"]["pr_qualification"]["default"], "false")
        steps = reused["jobs"]["e2e"]["steps"]
        admission = next(step for step in steps if "pr_qualification.py" in step.get("run", ""))
        self.assertEqual(admission["if"], "${{ inputs.pr_qualification }}")
        self.assertNotIn("${{", admission["run"])
        self.assertTrue(any("campaign.py run" in step.get("run", "") for step in steps))
        self.assertTrue(any("campaign.py clean" in step.get("run", "") and "always()" in step["if"]
                            for step in steps))
