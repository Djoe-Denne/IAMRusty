"""Review command, producer identity, and hosted execution boundaries."""
from copy import deepcopy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch
import zipfile

import yaml

CI = Path(__file__).resolve().parents[1] / "ci"
sys.path.insert(0, str(CI))
import artifacts
import image_input
import pr_qualification as pr
import runtime
from binaries import Refused

REPO = "test/AIForAll"
BASE, HEAD, MERGE, NEXT = (letter * 40 for letter in "abcd")
REF = "refs/heads/master"
PR_BRANCH = "feature/current"
REPOSITORY = {"id": 123, "full_name": REPO}
ACTOR = {"login": "operator", "id": 1, "type": "User"}


def environment():
    return dict(GITHUB_ACTIONS="true", GITHUB_JOB="e2e", GITHUB_EVENT_NAME="issue_comment",
                GITHUB_REPOSITORY=REPO, GITHUB_REPOSITORY_ID="123", GITHUB_ACTOR=ACTOR["login"],
                GITHUB_ACTOR_ID="1", GITHUB_SHA=BASE, GITHUB_REF=REF, RUNNER_ENVIRONMENT="github-hosted",
                GITHUB_WORKFLOW_REF=f"{REPO}/.github/workflows/k6-e2e-pr.yml@{REF}",
                K6_PR_MODE="true")


def event():
    return deepcopy(dict(action="created", repository=REPOSITORY, sender=ACTOR,
                         issue=dict(number=42, state="open", pull_request={}),
                         comment=dict(body="/k6-e2e", user=ACTOR)))


def pull():
    return deepcopy(dict(number=42, state="open", merged=False, mergeable=True, merge_commit_sha=MERGE,
                         head=dict(sha=HEAD, ref=PR_BRANCH, repo=REPOSITORY),
                         base=dict(sha=BASE, ref="master", repo=REPOSITORY)))


def run(number=92, attempt=2):
    return dict(id=number + 700, run_attempt=attempt, run_number=number, workflow_id=987,
                path=".github/workflows/ci.yml", event="pull_request", head_branch=PR_BRANCH, head_sha=HEAD,
                repository=REPOSITORY.copy(), head_repository=REPOSITORY.copy(), pull_requests=[pull()])


def mock_api(*, access="write", runs=None):
    api = Mock()
    selected = run()

    def respond(path):
        if path == "pulls/42":
            return pull()
        if path == "collaborators/operator/permission":
            return dict(permission=access, user=ACTOR)
        if path == "actions/workflows/ci.yml":
            return dict(id=987, path=".github/workflows/ci.yml", state="active")
        if path == "actions/runs/792/attempts/2":
            return selected
        raise AssertionError(path)

    api.json.side_effect = respond
    api.pages.side_effect = lambda path, key, **_kwargs: (
        [run(91, 1), run(92, 2)] if runs is None else runs
    ) if key == "workflow_runs" else [
        dict(name=name, status="completed", conclusion="success", run_id=792, run_attempt=2, head_sha=HEAD)
        for name in artifacts.ROLES.values()
    ]
    return api


class RequestTests(unittest.TestCase):
    def test_exact_command_and_live_write_permission(self):
        self.assertEqual(pr.request_number(event(), environment(), ""), 42)
        for template, field, replacement in (
            ("comment", "body", "/K6-E2E"),
            ("comment", "body", "/k6-e2e 100"),
            ("comment", "user", {**ACTOR, "id": 100}),
            ("issue", "state", "closed"),
            ("issue", "pull_request", None),
        ):
            payload = event()
            payload[template][field] = replacement
            with self.subTest(field=field), self.assertRaises(Refused):
                pr.request_number(payload, environment(), "")
        with self.assertRaises(Refused):
            pr.request_number({**event(), "action": "edited"}, environment(), "")
        for permission in ("read", "none"):
            with self.subTest(permission=permission), self.assertRaisesRegex(Refused, "PR_E2E_WRITE_REQUIRED"):
                pr.authorize(mock_api(access=permission), environment())
        pr.authorize(mock_api(), environment())

    def test_comment_only_allowed_on_exact_hosted_job(self):
        self.assertTrue(artifacts.premerge_job(environment()))
        for changes in (
            {"GITHUB_EVENT_NAME": "pull_request_target"},
            {"RUNNER_ENVIRONMENT": "self-hosted"},
            {"GITHUB_JOB": "lint"},
            {"GITHUB_REF": "refs/heads/feature"},
            {"GITHUB_WORKFLOW_REF": f"{REPO}/.github/workflows/ci.yml@{REF}"},
        ):
            with self.subTest(changes=changes):
                self.assertFalse(artifacts.premerge_job({**environment(), **changes}))
                with self.assertRaises(Refused):
                    runtime.hosted({**environment(), **changes})

    def test_dispatch_preview_must_still_target_the_selected_pr(self):
        ref = "refs/heads/" + PR_BRANCH
        env = {**environment(), "GITHUB_EVENT_NAME": "workflow_dispatch", "GITHUB_REF": ref,
               "GITHUB_WORKFLOW_REF": f"{REPO}/.github/workflows/k6-e2e.yml@{ref}",
               "GITHUB_SHA": HEAD}
        self.assertTrue(artifacts.premerge_job(env))
        self.assertEqual(pr.request_number({"repository": REPOSITORY}, env, "42"), 42)
        with self.assertRaises(Refused):
            pr.request_number({"repository": REPOSITORY}, env, "42\n")
        pr.check_dispatch_ref(env, pr.revision(pull(), 42, REPO, 123))
        with self.assertRaisesRegex(Refused, "PR_E2E_PREVIEW_REF"):
            pr.check_dispatch_ref({**env, "GITHUB_SHA": NEXT}, pr.revision(pull(), 42, REPO, 123))


class RevisionTests(unittest.TestCase):
    def test_rejects_closed_unmergeable_foreign_or_unresolved_targets(self):
        for delta in (dict(state="closed"), dict(mergeable=None), dict(merge_commit_sha=HEAD),
                      dict(head={**pull()["head"], "repo": {"id": 456, "full_name": "fork/x"}}),
                      dict(base={**pull()["base"], "ref": "feature"})):
            with self.subTest(delta=delta), self.assertRaises(Refused):
                pr.revision({**pull(), **delta}, 42, REPO, 123)

    def test_merge_tree_and_freshness_are_independent(self):
        def git(args, *, cwd):
            self.assertEqual(cwd, Path.cwd())
            return MERGE if args[1] == "rev-parse" else f"{BASE} {HEAD}"

        with patch.object(pr, "command", side_effect=git):
            pr.verify_tree(Path.cwd(), MERGE, BASE, HEAD)
            with self.assertRaisesRegex(Refused, "PR_E2E_MERGE_IDENTITY"):
                pr.verify_tree(Path.cwd(), MERGE, HEAD, BASE)
        pr.check_current(mock_api(), environment(), 42, MERGE)
        with self.assertRaisesRegex(Refused, "PR_E2E_REVISION_MOVED"):
            pr.check_current(mock_api(), environment(), 42, NEXT)


class ProducerTests(unittest.TestCase):
    def setUp(self):
        self.target = pr.revision(pull(), 42, REPO, 123)

    def test_selects_newest_exact_pr_run_and_attempt(self):
        api = mock_api()
        self.assertEqual(pr.pick_producer(api, self.target, REPO, 123), (987, 792, 2))
        api.pages.assert_any_call("actions/workflows/ci.yml/runs", "workflow_runs",
                                  parameters={"event": "pull_request", "head_sha": HEAD})

    def test_no_same_head_other_pr_base_or_fallback_to_successful_old_attempt(self):
        for candidate in (
            {**run(), "event": "push"},
            {**run(), "pull_requests": [{**pull(), "number": 43}]},
            {**run(), "pull_requests": [{**pull(), "base": {**pull()["base"], "sha": NEXT}}]},
            {**run(), "head_repository": {"id": 99, "full_name": "fork/x"}},
        ):
            with self.subTest(candidate=candidate), self.assertRaises(Refused):
                pr.pick_producer(mock_api(runs=[candidate]), self.target, REPO, 123)
        api = mock_api()
        original = api.pages.side_effect
        api.pages.side_effect = lambda *args, **kwargs: original(*args, **kwargs)[:-1] if args[1] == "jobs" else original(*args, **kwargs)
        with self.assertRaisesRegex(Refused, "BUILD_JOB_NOT_SUCCESSFUL"):
            pr.pick_producer(api, self.target, REPO, 123)

    def test_miss_is_not_success_and_keeps_outer_head_distinct_from_build_sha(self):
        api = mock_api()
        env = environment()
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "acquired"
            env["GITHUB_OUTPUT"] = str(Path(tmp) / "github-output")
            env["GITHUB_STEP_SUMMARY"] = str(Path(tmp) / "summary")
            with patch.object(pr, "GitHub", return_value=api), patch.object(pr, "read_event", return_value=event()), \
                 patch.object(pr, "save_bundles", side_effect=Refused("BINARY_ARTIFACT_MISSING_OR_AMBIGUOUS")) as save:
                with self.assertRaisesRegex(Refused, "BINARY_ARTIFACT_MISSING"):
                    pr.acquire(output, "", env)
                self.assertFalse(Path(env["GITHUB_OUTPUT"]).exists())
                save.assert_called_once_with(output, api, REPO, 123, 987, 792, 2, MERGE, artifact_sha=HEAD)

    def test_master_image_admission_is_not_used_for_pr(self):
        with self.assertRaisesRegex(Refused, "MANUAL_MASTER_E2E_REQUIRED"):
            image_input.acquire(Path.cwd(), "792", "2", MERGE,
                                {**environment(), "GITHUB_EVENT_NAME": "workflow_dispatch"})

    def test_binary_zip_selection_never_mixes_attempts_heads_or_repositories(self):
        items = [dict(id=i, name=f"k6-binaries-{role}-792-2", digest="sha256:" + "e" * 64, size_in_bytes=256,
                      expired=False, expires_at="2099-01-01T00:00:00Z",
                      workflow_run=dict(id=792, head_sha=HEAD, repository_id=123, head_repository_id=123))
                 for i, role in enumerate(artifacts.ROLES, start=1)]

        def download(item, path):
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("bundle.tar.gz", str(item["id"]).encode())

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            api = Mock()
            api.download.side_effect = download
            for index, bad in enumerate((dict(name="k6-binaries-iam-792-1"),
                                         dict(expired=True),
                                         dict(workflow_run={**items[0]["workflow_run"], "head_sha": NEXT}),
                                         dict(workflow_run={**items[0]["workflow_run"], "head_repository_id": 7}))):
                output = root / str(index)
                api.pages.return_value = [{**items[0], **bad}, *items[1:]]
                with self.subTest(bad=bad), self.assertRaises(Refused):
                    artifacts.save_bundles(output, api, REPO, 123, 987, 792, 2, MERGE, artifact_sha=HEAD)
                self.assertFalse(output.exists())
                api.download.assert_not_called()
            api.pages.return_value = items
            output = root / "accepted"
            artifacts.save_bundles(output, api, REPO, 123, 987, 792, 2, MERGE, artifact_sha=HEAD)
            receipt = json.loads((output / "selection.json").read_bytes())
            self.assertEqual((receipt["source_sha"], receipt["run_id"], receipt["run_attempt"]), (MERGE, 792, 2))
            self.assertEqual({role: info["id"] for role, info in receipt["artifacts"].items()}, {role: i for i, role in enumerate(artifacts.ROLES, start=1)})


class WorkflowTests(unittest.TestCase):
    def test_comment_workflow_only_routes_the_command_to_read_only_jobs(self):
        root = CI.parents[3] / ".github/workflows"
        wrapper = yaml.load((root / "k6-e2e-pr.yml").read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(wrapper["on"], {"issue_comment": {"types": ["created"]}})
        self.assertEqual(wrapper["permissions"], {"actions": "read", "contents": "read", "pull-requests": "read"})
        self.assertEqual(wrapper["cache-mode"], "none")
        job = wrapper["jobs"]["e2e"]
        self.assertIn("github.event.issue.pull_request", job["if"])
        self.assertIn("github.event.comment.body == '/k6-e2e'", job["if"])
        self.assertNotIn("secrets", job)
        self.assertEqual(job["uses"], "./.github/workflows/k6-e2e.yml")
        reused = yaml.load((root / "k6-e2e.yml").read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(reused["permissions"], wrapper["permissions"])
        self.assertEqual(reused["cache-mode"], "none")
        self.assertNotIn("workflow_dispatch", wrapper["on"])
        steps = reused["jobs"]["e2e"]["steps"]
        admission = next(index for index, step in enumerate(steps) if "pr_qualification.py acquire" in step.get("run", ""))
        checkout = next(index for index, step in enumerate(steps) if step.get("with", {}).get("path") == ".k6-pr-source")
        self.assertLess(admission, checkout)
        self.assertEqual(steps[checkout]["with"]["ref"], "${{ steps.pr.outputs.target_sha }}")
        self.assertEqual(steps[checkout]["with"]["persist-credentials"], "false")
        for step in steps[checkout:]:
            self.assertNotIn("GH_TOKEN", step.get("env", {}))
            self.assertNotIn("cargo ", step.get("run", ""))
        self.assertTrue(any("campaign.py run" in step.get("run", "") for step in steps))
        self.assertTrue(any("campaign.py clean" in step.get("run", "") and "always()" in step["if"]
                            for step in steps))
        self.assertEqual(reused["jobs"]["verify-pr-current"]["needs"], "e2e")
