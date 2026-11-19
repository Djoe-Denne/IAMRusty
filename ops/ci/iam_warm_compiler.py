"""Restore one pinned prior sccache snapshot in the warm benchmark only."""
from datetime import datetime
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tarfile
import tempfile

import iam_compile_once as iam

sys.path.insert(0, str(iam.ci.ROOT / "ops/tests/k6/ci"))
from artifacts import GitHub, Refused, unwrap_zip, validate_artifact

REPO = "Djoe-Denne/IAMRusty"
REPO_ID = 989204416
SEED_RUN = 44518086358
SEED_ATTEMPT = 1
SEED_BRANCH = "ci-iam-cd"
SEED_SOURCE = "743ebc0edc16bbd6570e6ea7334c94b09fb62775"
SEED_SDK = "1db6c9de8708f6451d91a56f482b053026487c8f"
SEED_JOB = 135539506878
SEED_ARTIFACT = dict(id=14527570584, name="iam-cd-d-compiler-1", size_in_bytes=311967169,
                     digest="sha256:d1e9e35d23f206b11a3eecb08af6d7f25336a1cd96ed6d48929c97c5b4dfea8c")
MAX_BYTES = 2 * 1024**3
MAX_ENTRIES = 2048


def entry_name(name):
    parts = PurePosixPath(name).parts
    return (len(parts) == 3 and re.fullmatch(r"[0-9a-f]{64}", parts[-1])
            and parts[:2] == tuple(parts[-1][:2]) and "/".join(parts) == name)


def checked_entries(entries):
    seen, total = set(), 0
    for entry in entries:
        total += entry.size
        iam.require(entry_name(entry.name) and entry.isfile() and not entry.issparse()
                    and entry.name not in seen and entry.size > 0
                    and len(seen) < MAX_ENTRIES and total <= MAX_BYTES, "Unexpected compiler-cache member")
        seen.add(entry.name)
        yield entry
    iam.require(seen, "Empty compiler snapshot")
    print(f"Compiler snapshot: {len(seen)} entries, {total} bytes", flush=True)


def require_fresh(cache):
    iam.require(cache.is_dir() and not cache.is_symlink() and not any(cache.iterdir()),
                "Restore requires a fresh compiler cache")


def restore_snapshot(archive, cache):
    require_fresh(cache)
    with tarfile.open(archive, "r|gz") as bundle:
        for entry in checked_entries(bundle):
            bundle.extract(entry, cache, filter="data")


def select_seed(api, now=None):
    # A failed downstream unit replay did not invalidate the successful server producer.
    run = api.json(f"actions/runs/{SEED_RUN}/attempts/{SEED_ATTEMPT}")
    iam.require(run.get("id") == SEED_RUN and run.get("run_attempt") == SEED_ATTEMPT
                and run.get("path") == iam.AC_WORKFLOW and run.get("event") == "push"
                and run.get("head_branch") == SEED_BRANCH and run.get("head_sha") == SEED_SOURCE
                and run.get("status") == "completed", "Foreign compiler seed run")
    for key in ("repository", "head_repository"):
        repo = run.get(key) or {}
        iam.require(repo.get("id") == REPO_ID and repo.get("full_name") == REPO,
                    "Foreign compiler seed repository")
    job = api.json(f"actions/jobs/{SEED_JOB}")
    iam.require(job.get("id") == SEED_JOB and job.get("name") == "D instrumented IAM with cache export"
                and job.get("run_id") == SEED_RUN and job.get("run_attempt") == SEED_ATTEMPT
                and job.get("head_sha") == SEED_SOURCE and job.get("status") == "completed"
                and job.get("conclusion") == "success", "Compiler seed producer not successful")
    uploads = [step for step in job["steps"] if step.get("name") == "Upload only the compiler cache"]
    iam.require(len(uploads) == 1 and uploads[0].get("conclusion") == "success",
                "Compiler seed upload not successful")
    item = api.json(f"actions/artifacts/{SEED_ARTIFACT['id']}")
    origin = item.get("workflow_run") or {}
    iam.require(origin.get("repository_id") == origin.get("head_repository_id") == REPO_ID
                and origin.get("head_branch") == SEED_BRANCH, "Foreign compiler seed artifact origin")
    selected = validate_artifact(item, SEED_RUN, SEED_SOURCE, now)
    iam.require(selected == SEED_ARTIFACT, "Unexpected compiler seed artifact")
    created = datetime.fromisoformat(item["created_at"])
    iam.require(datetime.fromisoformat(uploads[0]["started_at"]) <= created
                <= datetime.fromisoformat(uploads[0]["completed_at"]), "Compiler seed outside producer upload")
    return selected


def require_consumer(known, env):
    iam.require(known["lane"] == "ac" and known["repository_id"] == str(REPO_ID)
                and known["sdk"] == SEED_SDK and env.get("GITHUB_JOB") in {"build-d-server", "build-d-tests"},
                "Compiler seed is for the warm producers only")


def restore(env=os.environ):
    require_consumer(iam.identity(env), env)
    temp = Path(env["RUNNER_TEMP"])
    cache = Path(env["SCCACHE_DIR"])
    iam.require(cache == temp / "iam-ac-sccache" and cache.is_dir() and not cache.is_symlink(),
                "Unexpected compiler cache directory")
    require_fresh(cache)
    api = GitHub(REPO, env["GH_TOKEN"])
    item = select_seed(api)
    with tempfile.TemporaryDirectory(prefix="iam-warm-compiler-", dir=temp) as directory:
        raw = Path(directory) / "seed.zip"
        archive = Path(directory) / "bundle.tar.gz"
        api.download(item, raw)
        iam.require(raw.stat().st_size == item["size_in_bytes"], "Compiler seed download size mismatch")
        unwrap_zip(raw, archive)
        restore_snapshot(archive, cache)
    print(f"Pinned compiler seed restored: run {SEED_RUN}/{SEED_ATTEMPT}, producer {SEED_JOB}, "
          f"artifact {item['id']}, {item['digest']}, {item['size_in_bytes']} ZIP bytes", flush=True)


def main():
    os.umask(0o077)
    try:
        restore()
    except Refused as error:
        raise SystemExit(f"Compiler seed refused: {error}") from None
    except (KeyError, OSError, TypeError, ValueError, tarfile.TarError, subprocess.SubprocessError):
        raise SystemExit("Compiler seed provenance, download or extraction failed; no cold fallback") from None


if __name__ == "__main__":
    main()
