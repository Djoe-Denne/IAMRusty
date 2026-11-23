"""Admit saved master images for a harness-only, explicitly labelled PR."""
import argparse
import os
from pathlib import Path
import re
import subprocess

from binaries import ROOT, Refused, require, strict_json
from image_input import acquire_saved
from runtime import hosted, premerge_job


SHA = re.compile(r"[a-f0-9]{40}")
LABEL = re.compile(r"k6-e2e:([1-9][0-9]{0,19}):([1-9][0-9]{0,8})")
WORKFLOWS = {".github/workflows/k6-e2e.yml", ".github/workflows/k6-e2e-pr.yml"}


def allowed_path(path):
    if not isinstance(path, str) or not re.fullmatch(r"[A-Za-z0-9_./-]+", path):
        return False
    if any(part in {"", ".", ".."} for part in path.split("/")):
        return False
    return (path in WORKFLOWS or path == "README.md"
            or path.startswith("ops/tests/k6/")
            or path.startswith(("docs/adr/", "docs/contracts/", ".serena/memories/architecture/"))
            and path.endswith(".md"))


def admit(event, env, commit, parents, paths):
    require(premerge_job(env), "PR_E2E_JOB_SCOPE")
    try:
        request = event["pull_request"]
        base, head = request["base"], request["head"]
        source, revision = base["sha"], head["sha"]
        label = LABEL.fullmatch(event["label"]["name"])
        number = event["number"]
        repository_id = int(env["GITHUB_REPOSITORY_ID"])
        repositories = (event["repository"], base["repo"], head["repo"])
        same_repo = all(repo["id"] == repository_id and repo["full_name"] == env["GITHUB_REPOSITORY"]
                        for repo in repositories)
        requested = event["action"] == "labeled" and request["state"] == "open" and base["ref"] == "master"
    except (KeyError, TypeError, ValueError):
        raise Refused("PR_E2E_EVENT") from None
    require(requested and type(number) is int and number > 0
            and env["GITHUB_REF"] == f"refs/pull/{number}/merge" and same_repo and label is not None, "PR_E2E_EVENT")
    require(all(isinstance(value, str) and SHA.fullmatch(value) for value in (commit, source, revision))
            and commit == env["GITHUB_SHA"] and tuple(parents) == (source, revision), "PR_E2E_MERGE_IDENTITY")
    require(bool(paths) and all(allowed_path(path) for path in paths), "PR_E2E_APPLICATION_DIFF")
    return *label.groups(), source


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, check=True, timeout=30).stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    env = dict(os.environ)
    try:
        hosted(env)
        require(premerge_job(env), "PR_E2E_JOB_SCOPE")
        with Path(env["GITHUB_EVENT_PATH"]).open("rb") as stream:
            raw = stream.read(1024 * 1024 + 1)
        require(len(raw) <= 1024 * 1024, "PR_E2E_EVENT")
        event = strict_json(raw)
        commit = git("rev-parse", "HEAD").decode("ascii").strip()
        parents = git("show", "-s", "--format=%P", "HEAD").decode("ascii").split()
        require(len(parents) == 2 and all(SHA.fullmatch(sha) for sha in parents), "PR_E2E_MERGE_IDENTITY")
        paths = git("diff", "--name-only", "--no-renames", "-z", parents[0], "HEAD", "--").decode("utf-8")
        run, attempt, base = admit(event, env, commit, parents, paths.removesuffix("\0").split("\0"))
        acquire_saved(args.output, run, attempt, base, env)
    except Refused as error:
        print(str(error))
        return 1
    except (KeyError, OSError, ValueError, subprocess.SubprocessError):
        print("PR_E2E_INPUT_UNAVAILABLE")
        return 1
    print("Verified saved images from the exact application base for this PR merge commit")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
