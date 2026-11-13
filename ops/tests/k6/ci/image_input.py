"""Acquire an immutable image bundle produced by the manual packaging workflow."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import stat
import zipfile

from artifacts import GitHub, MAX_DOWNLOAD, positive, validate_artifact, current_ci_source
from binaries import ROLES, Refused, require, sha, strict_json

FILES = {"images.tar.gz", "provenance.json", "cleanup.json", "docker-resources.jsonl"}
PATHS = {"iam": ["/app/iam-service", "/app/migration"], "oauth": ["/app/oauth-connect-service"],
         "ext-authz": ["/app/ext-authz"]}


def admit(run, workflow_id, repository, repository_id, run_id, attempt):
    require(run.get("id") == run_id and run.get("run_attempt") == attempt, "IMAGE_RUN_ATTEMPT")
    require(run.get("workflow_id") == workflow_id and run.get("path") == ".github/workflows/k6-images.yml", "IMAGE_WORKFLOW")
    require(run.get("event") == "workflow_dispatch" and run.get("head_branch") == "master"
            and run.get("status") == "completed" and run.get("conclusion") == "success", "IMAGE_RUN_NOT_TRUSTED_SUCCESS")
    for field in ("repository", "head_repository"):
        require(run.get(field, {}).get("id") == repository_id
                and run[field].get("full_name") == repository, "IMAGE_RUN_REPOSITORY")


def provenance(value, cleanup, source_sha, packaging_sha):
    require(value.get("schema") == "aiforall.k6-images/v1" and value.get("target_sha") == source_sha
            and value.get("harness_sha") == packaging_sha, "IMAGE_PROVENANCE")
    require(value.get("cargo_profile") == "dev" and value.get("instrumentation") == "none"
            and value.get("qualification") == "PACKAGING_AND_ABI_ONLY_NOT_E2E", "IMAGE_QUALIFICATION")
    require(isinstance(value.get("sdk_sha"), str) and re.fullmatch(r"[a-f0-9]{40}", value["sdk_sha"]), "IMAGE_SDK")
    require(cleanup.get("schema") == "aiforall.k6-packaging-cleanup/v1"
            and cleanup.get("status") == "PASS" and type(cleanup.get("owned_running")) is int
            and cleanup["owned_running"] == 0, "IMAGE_PACKAGING_CLEANUP")
    require(isinstance(value.get("roles"), dict) and set(value["roles"]) == set(ROLES), "IMAGE_ROLES")
    for role, item in value["roles"].items():
        require(re.fullmatch(r"sha256:[a-f0-9]{64}", item.get("image_id", "")), "IMAGE_ID")
        require(re.fullmatch(r"local/k6-" + re.escape(role) + r":k6-[1-9][0-9]*-[1-9][0-9]*", item.get("tag", "")), "IMAGE_TAG")
        require(item.get("abi", {}).get("resolution") == "PASS" and item["abi"].get("paths") == PATHS[role], "IMAGE_ABI")
        manifest = item.get("manifest", {})
        require(manifest.get("source_sha") == source_sha and manifest.get("sdk_sha") == value["sdk_sha"]
                and manifest.get("instrumentation") == "none" and manifest.get("role") == role, "IMAGE_BINARY_PROVENANCE")
    require(re.fullmatch(r"[a-f0-9]{64}", value.get("archive_sha256", "")), "IMAGE_ARCHIVE_HASH")
    return value


def extract(path, output):
    output = Path(output)
    require(not output.exists(), "IMAGE_EXTRACTION_COLLISION")
    with zipfile.ZipFile(path) as archive:
        members = archive.infolist()
        require(len(members) == 4 and {m.filename for m in members} == FILES, "IMAGE_ARCHIVE_MEMBERS")
        require(all(stat.S_IFMT(m.external_attr >> 16) in (0, stat.S_IFREG) and not m.flag_bits & 1 for m in members), "IMAGE_ARCHIVE_TYPE")
        require(all(0 < m.file_size <= (MAX_DOWNLOAD if m.filename == "images.tar.gz" else 1024 ** 2)
                    for m in members), "IMAGE_ARCHIVE_SIZE")
        output.mkdir(mode=0o700)
        for member in members:
            with archive.open(member) as source, (output / member.filename).open("xb") as target:
                shutil.copyfileobj(source, target, 1024 * 1024)


def acquire(directory, run_id, attempt, source_sha, environment=None):
    env = dict(os.environ if environment is None else environment)
    require(env.get("GITHUB_REF") == "refs/heads/master" and env.get("GITHUB_EVENT_NAME") == "workflow_dispatch", "MANUAL_MASTER_E2E_REQUIRED")
    run_id, attempt = positive(run_id), positive(attempt)
    require(re.fullmatch(r"[a-f0-9]{40}", source_sha), "IMAGE_TARGET_SHA")
    directory = Path(directory)
    require(directory.is_absolute() and not directory.exists(), "IMAGE_ACQUISITION_COLLISION")
    directory.mkdir(mode=0o700)
    api = GitHub(env["GITHUB_REPOSITORY"], env.get("GH_TOKEN", ""))
    workflow = api.json("actions/workflows/k6-images.yml")
    require(workflow.get("path") == ".github/workflows/k6-images.yml" and workflow.get("state") == "active", "IMAGE_WORKFLOW")
    run = api.json(f"actions/runs/{run_id}/attempts/{attempt}")
    admit(run, workflow["id"], env["GITHUB_REPOSITORY"], positive(env["GITHUB_REPOSITORY_ID"]), run_id, attempt)
    artifacts = api.pages(f"actions/runs/{run_id}/artifacts", "artifacts")
    name = f"k6-images-{run_id}-{attempt}"
    matches = [a for a in artifacts if a.get("name") == name]
    require(len(matches) == 1, "IMAGE_ARTIFACT_MISSING_OR_AMBIGUOUS")
    validate_artifact(matches[0], run_id, run["head_sha"])
    archive = directory / "artifact.zip"
    api.download(matches[0], archive)
    output = directory / "verified"
    extract(archive, output)
    archive.unlink()
    value = provenance(strict_json((output / "provenance.json").read_bytes()),
                       strict_json((output / "cleanup.json").read_bytes()), source_sha, run["head_sha"])
    closure = strict_json((output / "cleanup.json").read_bytes())
    require(closure.get("run_id") == str(run_id) and closure.get("run_attempt") == str(attempt), "IMAGE_CLEANUP_ORIGIN")
    require(all(item["tag"] == f"local/k6-{role}:k6-{run_id}-{attempt}" for role, item in value["roles"].items()), "IMAGE_TAG_ORIGIN")
    require(sha(output / "images.tar.gz") == value["archive_sha256"], "IMAGE_PAYLOAD_HASH")
    (directory / "admission.json").write_text(json.dumps({"run_id": run_id, "run_attempt": attempt,
        "artifact_id": matches[0]["id"], "artifact_digest": matches[0]["digest"], "packaging_sha": run["head_sha"],
        "target_sha": source_sha, "sdk_sha": value["sdk_sha"]}, sort_keys=True), encoding="utf-8")
    return value


def from_packaging(source, directory, run_id, attempt, source_sha, environment=None):
    """Same trusted CI job only; never admit unrelated local/saved images."""
    env = dict(os.environ if environment is None else environment)
    require(re.fullmatch(r"[a-f0-9]{40}", source_sha) and current_ci_source(env, run_id, attempt, source_sha), "IMAGE_CURRENT_CI_ONLY")
    run_id, attempt = positive(run_id), positive(attempt)
    source, directory = Path(source), Path(directory)
    require(source.is_absolute() and directory.is_absolute() and not directory.exists(), "IMAGE_ACQUISITION_COLLISION")
    for name in FILES:
        path = source / name
        require(path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= (MAX_DOWNLOAD if name == "images.tar.gz" else 1024**2),
                "IMAGE_LOCAL_FILE")
    closure = strict_json((source / "cleanup.json").read_bytes())
    value = provenance(strict_json((source / "provenance.json").read_bytes()), closure, source_sha, env["GITHUB_SHA"])
    require(closure.get("run_id") == str(run_id) and closure.get("run_attempt") == str(attempt), "IMAGE_CLEANUP_ORIGIN")
    require(all(item["tag"] == f"local/k6-{role}:k6-{run_id}-{attempt}" for role, item in value["roles"].items()), "IMAGE_TAG_ORIGIN")
    require(sha(source / "images.tar.gz") == value["archive_sha256"], "IMAGE_PAYLOAD_HASH")
    directory.mkdir(mode=0o700)
    verified = directory / "verified"
    verified.mkdir(mode=0o700)
    for name in FILES:
        shutil.copyfile(source / name, verified / name)
    require(sha(verified / "images.tar.gz") == value["archive_sha256"], "IMAGE_PAYLOAD_HASH")
    (directory / "admission.json").write_text(json.dumps(dict(mode="same-ci-job", run_id=run_id,
        run_attempt=attempt, target_sha=source_sha, packaging_sha=env["GITHUB_SHA"], sdk_sha=value["sdk_sha"]), sort_keys=True), encoding="utf-8")
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--attempt", required=True)
    parser.add_argument("--target-sha", required=True)
    parser.add_argument("--packaged-directory", type=Path)
    args = parser.parse_args()
    try:
        if args.packaged_directory:
            from_packaging(args.packaged_directory, args.output, args.run_id, args.attempt, args.target_sha)
        else:
            acquire(args.output, args.run_id, args.attempt, args.target_sha)
    except Refused as error:
        raise SystemExit("E2E image acquisition refused: " + str(error)) from None
    except Exception:
        raise SystemExit("E2E image acquisition failed; no rebuild fallback") from None
    print("Verified reusable image bundle acquired; no compilation or runtime started")


if __name__ == "__main__":
    main()
