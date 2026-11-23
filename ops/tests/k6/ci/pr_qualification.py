"""Resolve an authorized PR request to exact saved binaries from ordinary CI."""
import argparse
from dataclasses import dataclass
import os
from pathlib import Path
import re

from artifacts import GitHub, positive, premerge_job, save_bundles, validate_jobs
from binaries import Refused, command, require, strict_json

SHA = re.compile(r"[a-f0-9]{40}")
WORKFLOW = ".github/workflows/ci.yml"
LOGIN = re.compile(r"[A-Za-z0-9-]{1,39}")
NUMBER = re.compile(r"[1-9][0-9]{0,9}")


def same_repo(repository, repo, repo_id):
    return isinstance(repository, dict) and repository.get("id") == repo_id and repository.get("full_name") == repo


def read_event(environment):
    with Path(environment["GITHUB_EVENT_PATH"]).open("rb") as stream:
        raw = stream.read(1024 * 1024 + 1)
    require(len(raw) <= 1024 * 1024, "PR_E2E_EVENT")
    return strict_json(raw)


def actor(environment):
    login = environment.get("GITHUB_ACTOR", "")
    require(LOGIN.fullmatch(login), "PR_E2E_ACTOR")
    return login, positive(environment.get("GITHUB_ACTOR_ID", ""))


def human(user, environment):
    login, identity = actor(environment)
    return (isinstance(user, dict) and user.get("login") == login
            and user.get("id") == identity and user.get("type") == "User")


def authorize(api, environment):
    login, _identity = actor(environment)
    permission = api.json(f"collaborators/{login}/permission")
    require(permission.get("permission") in ("write", "maintain", "admin")
            and human(permission.get("user"), environment), "PR_E2E_WRITE_REQUIRED")


def request_number(event, environment, dispatch_number):
    require(premerge_job(environment), "PR_E2E_JOB_SCOPE")
    repo, identity = environment["GITHUB_REPOSITORY"], positive(environment["GITHUB_REPOSITORY_ID"])
    require(same_repo(event.get("repository"), repo, identity), "PR_E2E_REPOSITORY")
    if environment["GITHUB_EVENT_NAME"] == "workflow_dispatch":
        require(NUMBER.fullmatch(dispatch_number), "PR_E2E_NUMBER")
        return int(dispatch_number)
    comment, issue = event.get("comment", {}), event.get("issue", {})
    require(event.get("action") == "created" and comment.get("body") == "/k6-e2e"
            and human(event.get("sender"), environment) and human(comment.get("user"), environment)
            and isinstance(issue.get("pull_request"), dict) and issue.get("state") == "open", "PR_E2E_COMMAND")
    number = issue.get("number")
    require(type(number) is int and NUMBER.fullmatch(str(number)), "PR_E2E_NUMBER")
    return number


@dataclass(frozen=True)
class Revision:
    number: int
    head: str
    base: str
    merge: str
    branch: str


def revision(pull, number, repo, repo_id):
    head, base = pull.get("head", {}), pull.get("base", {})
    require(pull.get("number") == number and pull.get("state") == "open" and pull.get("merged") is False
            and base.get("ref") == "master" and same_repo(head.get("repo"), repo, repo_id)
            and same_repo(base.get("repo"), repo, repo_id), "PR_E2E_OPEN_INTERNAL_PR_REQUIRED")
    commits = (head.get("sha"), base.get("sha"), pull.get("merge_commit_sha"))
    require(pull.get("mergeable") is True and all(isinstance(value, str) and SHA.fullmatch(value) for value in commits)
            and len(set(commits)) == 3, "PR_E2E_MERGE_UNAVAILABLE")
    branch = head.get("ref")
    require(isinstance(branch, str) and branch and branch != "master", "PR_E2E_HEAD_BRANCH")
    return Revision(number, *commits, branch)


def check_dispatch_ref(environment, target):
    if environment["GITHUB_EVENT_NAME"] == "workflow_dispatch":
        require(environment["GITHUB_REF"] == f"refs/heads/{target.branch}" and environment["GITHUB_SHA"] == target.head,
                "PR_E2E_PREVIEW_REF")


def check_current(api, environment, number, expected_merge):
    current = revision(api.json(f"pulls/{number}"), number, environment["GITHUB_REPOSITORY"],
                       positive(environment["GITHUB_REPOSITORY_ID"]))
    require(current.merge == expected_merge, "PR_E2E_REVISION_MOVED")


def matches(run, workflow, target, repository, repo_id):
    if not (run.get("workflow_id") == workflow and run.get("path") == WORKFLOW and run.get("event") == "pull_request"
            and run.get("head_branch") == target.branch and run.get("head_sha") == target.head
            and all(same_repo(run.get(field), repository, repo_id) for field in ("repository", "head_repository"))):
        return False
    associated = [item for item in run.get("pull_requests", []) if item.get("number") == target.number]
    if len(associated) != 1:
        return False
    head, base = associated[0]["head"], associated[0]["base"]
    return ((head.get("ref"), head.get("sha"), head.get("repo", {}).get("id")) == (target.branch, target.head, repo_id)
            and (base.get("ref"), base.get("sha"), base.get("repo", {}).get("id")) == ("master", target.base, repo_id))


def pick_producer(api, target, repository, repo_id):
    workflow = api.json("actions/workflows/ci.yml")
    require(workflow.get("path") == WORKFLOW and workflow.get("state") == "active", "PRODUCER_WORKFLOW")
    workflow_id = positive(str(workflow.get("id")))
    runs = api.pages("actions/workflows/ci.yml/runs", "workflow_runs",
                     parameters={"event": "pull_request", "head_sha": target.head})
    eligible = [run for run in runs if matches(run, workflow_id, target, repository, repo_id)]
    require(eligible, "PR_E2E_NO_BUILD_FOR_CURRENT_REVISION")
    latest = max(eligible, key=lambda run: positive(str(run.get("run_number"))))
    require(sum(run["run_number"] == latest["run_number"] for run in eligible) == 1, "PR_E2E_AMBIGUOUS_PRODUCER")
    run_id, attempt = (positive(str(latest.get(field))) for field in ("id", "run_attempt"))
    selected = api.json(f"actions/runs/{run_id}/attempts/{attempt}")
    require(selected.get("id") == run_id and selected.get("run_attempt") == attempt
            and matches(selected, workflow_id, target, repository, repo_id), "PR_E2E_PRODUCER_MOVED")
    validate_jobs(api.pages(f"actions/runs/{run_id}/attempts/{attempt}/jobs", "jobs"),
                  producer=(run_id, attempt, target.head))
    return workflow_id, run_id, attempt


def acquire(output, dispatch_number, env):
    number = request_number(read_event(env), env, dispatch_number)
    repo, repo_id = env["GITHUB_REPOSITORY"], positive(env["GITHUB_REPOSITORY_ID"])
    api = GitHub(repo, env.get("GH_TOKEN", ""))
    authorize(api, env)
    target = revision(api.json(f"pulls/{number}"), number, repo, repo_id)
    check_dispatch_ref(env, target)
    workflow_id, run_id, attempt = pick_producer(api, target, repo, repo_id)
    save_bundles(output, api, repo, repo_id, workflow_id, run_id, attempt, target.merge, artifact_sha=target.head)
    check_current(api, env, number, target.merge)
    with open(env["GITHUB_OUTPUT"], "a", encoding="utf-8") as stream:
        stream.write(f"pr_number={number}\ntarget_sha={target.merge}\nbase_sha={target.base}\nhead_sha={target.head}\n")
    with open(env["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as stream:
        stream.write(f"PR #{number}, merge `{target.merge}`; binary producer run `{run_id}`, attempt `{attempt}`.\n")


def verify_tree(source, merge, base, head):
    require(all(SHA.fullmatch(sha) for sha in (merge, base, head))
            and command(["git", "rev-parse", "HEAD"], cwd=source) == merge
            and command(["git", "show", "-s", "--format=%P", "HEAD"], cwd=source).split() == [base, head],
            "PR_E2E_MERGE_IDENTITY")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest="action", required=True)
    receive = modes.add_parser("acquire")
    receive.add_argument("--output", type=Path, required=True)
    receive.add_argument("--pr-number", default="")
    tree = modes.add_parser("verify-tree")
    tree.add_argument("--source", type=Path, required=True)
    tree.add_argument("--merge", required=True)
    tree.add_argument("--base", required=True)
    tree.add_argument("--head", required=True)
    current = modes.add_parser("verify-current")
    current.add_argument("--pr-number", required=True)
    current.add_argument("--merge", required=True)
    args = parser.parse_args()
    env = os.environ
    try:
        if args.action == "acquire":
            acquire(args.output, args.pr_number, env)
        elif args.action == "verify-tree":
            require(premerge_job(env), "PR_E2E_JOB_SCOPE")
            verify_tree(args.source, args.merge, args.base, args.head)
        else:
            require(NUMBER.fullmatch(args.pr_number) and SHA.fullmatch(args.merge), "PR_E2E_CURRENT_IDENTITY")
            check_current(GitHub(env["GITHUB_REPOSITORY"], env.get("GH_TOKEN", "")), env, int(args.pr_number), args.merge)
    except Refused as error:
        raise SystemExit(f"PR E2E refused: {error}. No rebuild or older artifact fallback.") from None
    except (KeyError, AttributeError, OSError, TypeError, ValueError):
        raise SystemExit("PR E2E input unavailable; no rebuild or older artifact fallback.") from None
    print("Verified PR E2E " + args.action)


if __name__ == "__main__":
    main()
