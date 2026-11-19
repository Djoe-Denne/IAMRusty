"""Same-attempt prebuilt runtime base for the hosted A/C pair only."""
import os
from pathlib import Path
import re
import subprocess
import sys

import binaries
import package

ROOT = binaries.ROOT
sys.path.insert(0, str(ROOT / "ops/ci"))
import iam_compile_once as iam

INSTALL = {
    "iam": "RUN apt-get update && apt-get install -y --no-install-recommends \\\n"
           "    ca-certificates \\\n    ${RUNTIME_OPENSSL_PACKAGE} \\\n    curl \\\n"
           "    && rm -rf /var/lib/apt/lists/*",
    "oauth": "RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates "
             "${RUNTIME_OPENSSL_PACKAGE} curl \\\n    && rm -rf /var/lib/apt/lists/* \\\n    && ",
    "ext-authz": "RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates "
                 "${RUNTIME_OPENSSL_PACKAGE} curl \\\n    && rm -rf /var/lib/apt/lists/*",
}
FRONTEND = "# syntax=docker/dockerfile:1\n"


def base_tag(known):
    return f"local/iam-ac-runtime:ci-{known['GITHUB_RUN_ID']}-{known['GITHUB_RUN_ATTEMPT']}"


def prepare(output):
    known = iam.identity()
    binaries.require(known["lane"] == "ac" and os.environ.get("GITHUB_JOB") == iam.AC_PRODUCERS["runtime"][0]
                     and sys.platform == "linux" and os.getuid() != 0, "NOT_AC_BASE_PRODUCER")
    os.umask(0o077)
    output.mkdir(mode=0o700)
    context = output / "empty-context"
    context.mkdir(mode=0o700)
    base = base_tag(known)
    ident = package.Packager(output).build(context, ROOT / "ops/tests/k6/ci/Dockerfile.iam-ac-base", base,
                                           {"RUNTIME_IMAGE": package.runtime_lock()["runtime"]})
    package.save_images([base], output / "bundle.tar.gz")
    print("A/C runtime base image:", ident)


def stage_recipe(role, source):
    original = INSTALL[role]
    binaries.require(source.count(original) == 1, "AC_RUNTIME_RECIPE_DRIFT")
    result = source.removeprefix(FRONTEND).replace(original, "RUN " if role == "oauth" else "RUN command -v curl", 1)
    binaries.require("apt-get" not in result, "AC_RUNTIME_RECIPE_NETWORK")
    return result


def load(acquired, output, context, known, roles):
    archive = acquired / "runtime"
    artifact = binaries.strict_json((archive / "github-artifact.json").read_bytes())
    tag = base_tag(known)
    tracker = package.Packager(output)
    try:
        subprocess.run(["docker", "image", "load", "--input", str(archive / "bundle.tar.gz")],
                       check=True, timeout=120)
    finally:
        tracker.observe_images()
    image = binaries.command(["docker", "image", "inspect", "--format", "{{.Id}}", tag])
    binaries.require(re.fullmatch(r"sha256:[a-f0-9]{64}", image), "AC_RUNTIME_IMAGE_ID")
    for role, item in roles.items():
        dockerfile = context / item["dockerfile"]
        dockerfile.write_text(stage_recipe(role, dockerfile.read_text(encoding="utf-8")), encoding="utf-8")
        item["dockerfile_sha256"] = binaries.sha(dockerfile)
    artifacts = context / "Dockerfile.artifacts"
    artifacts.write_text(artifacts.read_text(encoding="utf-8").removeprefix(FRONTEND), encoding="utf-8")
    return tag, {"image_id": image, **{key: artifact[key] for key in ("id", "digest", "producer_job_id")}}


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Expected runtime output directory")
    prepare(Path(sys.argv[1]))
