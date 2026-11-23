"""Verify native bundles and acquire automatic/manual master build inputs."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import urllib.parse
import urllib.request
import zipfile

from binaries import ROLES, Refused, require, strict_json

MAX_DOWNLOAD = 4 * 1024 ** 3


def positive(value):
    require(isinstance(value, str) and re.fullmatch(r"[1-9][0-9]*", value), "INVALID_RUN_OR_ATTEMPT")
    return int(value)


def validate_run(run, repository, repository_id, workflow_id, run_id, attempt, source_sha):
    require(isinstance(source_sha, str) and re.fullmatch(r"[a-f0-9]{40}", source_sha), "INVALID_TARGET_SHA")
    require(run.get("id") == run_id and run.get("run_attempt") == attempt, "RUN_ATTEMPT")
    require(run.get("workflow_id") == workflow_id and run.get("path") == ".github/workflows/ci.yml", "RUN_WORKFLOW")
    require(run.get("event") == "push" and run.get("head_branch") == "master", "RUN_TRUST_BOUNDARY")
    require(run.get("head_sha") == source_sha, "RUN_SOURCE_SHA")
    for key in ("repository", "head_repository"):
        value = run.get(key, {})
        require(value.get("id") == repository_id and value.get("full_name") == repository, "RUN_REPOSITORY")


def validate_jobs(jobs, *, producer=None):
    for name in ROLES.values():
        matched = [job for job in jobs if job.get("name") == name]
        require(len(matched) == 1 and matched[0].get("status") == "completed"
                and matched[0].get("conclusion") == "success", "BUILD_JOB_NOT_SUCCESSFUL")
        if producer is not None:
            run_id, attempt, head_sha = producer
            job = matched[0]
            require(job.get("run_id") == run_id and job.get("run_attempt") == attempt
                    and job.get("head_sha") == head_sha, "BUILD_JOB_WRONG_ATTEMPT")


def validate_artifact(item, run_id, source_sha, now=None):
    now = datetime.now(timezone.utc) if now is None else now
    try:
        expiry = datetime.fromisoformat(item["expires_at"].replace("Z", "+00:00"))
        require(expiry > now and item.get("expired") is False, "BINARY_ARTIFACT_EXPIRED")
    except (KeyError, ValueError, TypeError):
        raise Refused("BINARY_ARTIFACT_EXPIRY") from None
    require(type(item.get("id")) is int and item["id"] > 0, "BINARY_ARTIFACT_ID")
    require(isinstance(item.get("digest"), str) and re.fullmatch(r"sha256:[a-f0-9]{64}", item["digest"]), "BINARY_ARTIFACT_DIGEST")
    require(type(item.get("size_in_bytes")) is int and 0 < item["size_in_bytes"] <= MAX_DOWNLOAD, "BINARY_ARTIFACT_SIZE")
    origin = item.get("workflow_run", {})
    require(origin.get("id") == run_id and origin.get("head_sha") == source_sha, "BINARY_ARTIFACT_ORIGIN")
    return {key: item[key] for key in ("id", "name", "digest", "size_in_bytes")}


def select_artifacts(artifacts, run_id, attempt, source_sha, now=None):
    selected = {}
    for role in ROLES:
        name = f"k6-binaries-{role}-{run_id}-{attempt}"
        matched = [artifact for artifact in artifacts if artifact.get("name") == name]
        require(len(matched) == 1, "BINARY_ARTIFACT_MISSING_OR_AMBIGUOUS")
        selected[role] = validate_artifact(matched[0], run_id, source_sha, now)
    require(len({item["id"] for item in selected.values()}) == len(ROLES), "BINARY_ARTIFACT_IDS")
    return selected


class SafeRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, url):
        parsed = urllib.parse.urlsplit(url)
        require(parsed.scheme == "https" and parsed.hostname and
                (parsed.hostname == "api.github.com" or parsed.hostname.endswith(".githubusercontent.com")
                 or parsed.hostname.endswith(".blob.core.windows.net")), "ARTIFACT_REDIRECT_HOST")
        redirected = super().redirect_request(request, fp, code, message, headers, url)
        if redirected is not None and parsed.hostname != "api.github.com":
            redirected.remove_header("Authorization")
            redirected.remove_header("Cookie")
        return redirected


class GitHub:
    def __init__(self, repository, token):
        require(re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository), "REPOSITORY_INPUT")
        require(isinstance(token, str) and bool(token), "ACTIONS_READ_TOKEN_REQUIRED")
        self.repository = repository
        self.token = token
        self.opener = urllib.request.build_opener(SafeRedirect())

    def open(self, suffix):
        require(suffix.startswith("actions/") or re.fullmatch(
            r"pulls/[1-9][0-9]*|collaborators/[A-Za-z0-9-]{1,39}/permission", suffix), "API_SCOPE")
        request = urllib.request.Request("https://api.github.com/repos/" + self.repository + "/" + suffix,
                                        headers={"Authorization": "Bearer " + self.token,
                                                 "Accept": "application/vnd.github+json",
                                                 "X-GitHub-Api-Version": "2022-11-28",
                                                 "User-Agent": "aiforall-k6-binary-seam"})
        return self.opener.open(request, timeout=60)

    def json(self, suffix):
        with self.open(suffix) as response:
            raw = response.read(4 * 1024 ** 2 + 1)
        require(len(raw) <= 4 * 1024 ** 2, "API_OUTPUT_LIMIT")
        return strict_json(raw)

    def pages(self, suffix, key, *, parameters=None):
        values = []
        for page in range(1, 21):
            query = urllib.parse.urlencode({**(parameters or {}), "per_page": 100, "page": page})
            result = self.json(suffix + "?" + query)
            require(type(result.get("total_count")) is int and isinstance(result.get(key), list), "API_LIST_SCHEMA")
            require(result["total_count"] <= 2000, "API_PAGINATION_LIMIT")
            values.extend(result[key])
            if len(values) >= result["total_count"]:
                require(len(values) == result["total_count"], "API_LIST_CHANGED")
                return values
            require(bool(result[key]), "API_LIST_INCOMPLETE")
        raise Refused("API_LIST_INCOMPLETE")

    def download(self, item, path):
        digest = hashlib.sha256()
        count = 0
        with self.open(f"actions/artifacts/{item['id']}/zip") as source, Path(path).open("xb") as target:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                count += len(chunk)
                require(count <= MAX_DOWNLOAD, "ARTIFACT_DOWNLOAD_LIMIT")
                digest.update(chunk)
                target.write(chunk)
        require("sha256:" + digest.hexdigest() == item["digest"], "ARTIFACT_API_DIGEST_MISMATCH")


def unwrap_zip(path, destination):
    destination = Path(destination)
    require(not destination.exists(), "ARTIFACT_EXTRACTION_COLLISION")
    try:
        with zipfile.ZipFile(path) as archive:
            members = archive.infolist()
            require(len(members) == 1 and members[0].filename == "bundle.tar.gz", "ARTIFACT_ZIP_MEMBERS")
            member = members[0]
            require(stat.S_IFMT(member.external_attr >> 16) in (0, stat.S_IFREG), "ARTIFACT_ZIP_TYPE")
            require(0 < member.file_size <= MAX_DOWNLOAD and not member.flag_bits & 1, "ARTIFACT_ZIP_SIZE")
            with archive.open(member) as source, destination.open("xb") as target:
                shutil.copyfileobj(source, target, 1024 * 1024)
            require(destination.stat().st_size == member.file_size, "ARTIFACT_ZIP_TRUNCATED")
    except (zipfile.BadZipFile, OSError, EOFError):
        raise Refused("ARTIFACT_ZIP_INVALID") from None


def current_ci_source(environment, run_id, attempt, source_sha):
    return (isinstance(source_sha, str) and re.fullmatch(r"[a-f0-9]{40}", source_sha) is not None
            and environment.get("GITHUB_EVENT_NAME") == "push" and environment.get("GITHUB_REF") == "refs/heads/master"
            and environment.get("GITHUB_JOB") == "e2e" and environment.get("GITHUB_RUN_ID") == str(run_id)
            and environment.get("GITHUB_RUN_ATTEMPT") == str(attempt) and environment.get("GITHUB_SHA") == source_sha)


def premerge_job(env):
    ref = env.get("GITHUB_REF", "")
    repository = env.get("GITHUB_REPOSITORY", "")
    workflow = env.get("GITHUB_WORKFLOW_REF", "")
    if not (env.get("GITHUB_ACTIONS") == "true" and env.get("RUNNER_ENVIRONMENT") == "github-hosted"
            and env.get("GITHUB_JOB") == "e2e" and env.get("K6_PR_MODE") == "true"
            and re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository)):
        return False
    if env.get("GITHUB_EVENT_NAME") == "issue_comment":
        return ref == "refs/heads/master" and workflow == f"{repository}/.github/workflows/k6-e2e-pr.yml@{ref}"
    return (env.get("GITHUB_EVENT_NAME") == "workflow_dispatch" and ref.startswith("refs/heads/")
            and ref != "refs/heads/master" and workflow == f"{repository}/.github/workflows/k6-e2e.yml@{ref}")


def acquire(output, run_id, attempt, source_sha, environment=None):
    env = dict(os.environ if environment is None else environment)
    require(env.get("GITHUB_REF") == "refs/heads/master" and (env.get("GITHUB_EVENT_NAME") == "workflow_dispatch"
             or current_ci_source(env, run_id, attempt, source_sha)),
            "MANUAL_MASTER_HARNESS_REQUIRED")
    repo = env["GITHUB_REPOSITORY"]
    repo_id = positive(env["GITHUB_REPOSITORY_ID"])
    run_id, attempt = positive(run_id), positive(attempt)
    require(re.fullmatch(r"[a-f0-9]{40}", source_sha), "INVALID_TARGET_SHA")
    api = GitHub(repo, env.get("GH_TOKEN", ""))
    workflow = api.json("actions/workflows/ci.yml")
    require(workflow.get("path") == ".github/workflows/ci.yml" and workflow.get("state") == "active", "PRODUCER_WORKFLOW")
    run = api.json(f"actions/runs/{run_id}/attempts/{attempt}")
    validate_run(run, repo, repo_id, workflow["id"], run_id, attempt, source_sha)
    validate_jobs(api.pages(f"actions/runs/{run_id}/attempts/{attempt}/jobs", "jobs"))
    save_bundles(output, api, repo, repo_id, workflow["id"], run_id, attempt, source_sha)
    with open(env["GITHUB_OUTPUT"], "a", encoding="utf-8") as target:
        target.write("target_sha=" + source_sha + "\n")


def save_bundles(output, api, repo, repo_id, workflow_id, run_id, attempt, source_sha, *, artifact_sha=None):
    # For PR CI, GitHub's artifact metadata names the PR head, while the
    # producer's checked-out source (and binary manifest) names the merge SHA.
    items = api.pages(f"actions/runs/{run_id}/artifacts", "artifacts")
    selected = select_artifacts(items, run_id, attempt, source_sha if artifact_sha is None else artifact_sha)
    if artifact_sha is not None:
        selected_ids = {item["id"] for item in selected.values()}
        for item in items:
            if item.get("id") in selected_ids:
                origin = item["workflow_run"]
                require(origin.get("repository_id") == repo_id and origin.get("head_repository_id") == repo_id,
                        "BINARY_ARTIFACT_REPOSITORY")
    output = Path(output)
    require(output.is_absolute() and not output.exists(), "ACQUISITION_OUTPUT_EXISTS")
    output.mkdir(mode=0o700)
    for role, item in selected.items():
        path = output / (role + ".zip")
        api.download(item, path)
        unwrap_zip(path, output / (role + ".tar.gz"))
        path.unlink()
    selection = dict(schema="aiforall.k6-selection/v1", repository=repo, repository_id=repo_id,
                     workflow_id=workflow_id, run_id=run_id, run_attempt=attempt,
                     source_sha=source_sha, artifacts=selected)
    (output / "selection.json").write_text(json.dumps(selection, sort_keys=True), encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--attempt", required=True)
    parser.add_argument("--source-sha", required=True)
    args = parser.parse_args()
    try:
        acquire(args.output, args.run_id, args.attempt, args.source_sha)
    except Refused as error:
        raise SystemExit("CI binary acquisition refused: " + str(error)) from None
    except Exception:
        raise SystemExit("CI binary acquisition refused; no cache or rebuild fallback") from None
    print("Three trusted CI binary artifacts acquired by immutable ID and verified digest")


if __name__ == "__main__":
    main()
