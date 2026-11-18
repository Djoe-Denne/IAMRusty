"""Separate same-job experimental image admission; never accepted by native delivery."""

import os
from pathlib import Path
import re
import sys

import binaries
import campaign
import cleanup
import package
import runtime

ROOT = binaries.ROOT
sys.path.insert(0, str(ROOT / "ops/ci"))
import iam_compile_once as pilot

SCHEMA = "aiforall.iam-compile-once-images/v1"
INSTRUMENTATION = {"iam": "llvm-source-coverage", "oauth": "none", "ext-authz": "none"}


def proofs():
    runtime.hosted()
    binaries.require(runtime.pilot_hosted(os.environ), "NOT_PILOT_E2E_JOB")
    return pilot.identity()


def pack(acquired, output):
    known = proofs()
    source = known["GITHUB_SHA"]
    output.mkdir(mode=0o700)
    context = output / "context"

    def verify(role, directory, outputs):
        require = binaries.require
        part = "e2e" if role == "iam" else role
        path = acquired / part
        rows = pilot.unpack(path / "bundle.tar.gz", directory, known, part, set(outputs.values()))
        for row in rows:
            inspect = binaries.coverage_elf if role == "iam" else binaries.elf
            inspect(directory / row["name"])
        artifact = binaries.strict_json((path / "github-artifact.json").read_bytes())
        require(re.fullmatch(r"sha256:[a-f0-9]{64}", artifact["digest"]), "PILOT_ARTIFACT_DIGEST")
        return {"role": role, "source_sha": source, "sdk_sha": known["sdk"], "files": rows,
                "instrumentation": INSTRUMENTATION[role]}, artifact

    roles, entrypoint = package.stage_context(ROOT, context, ROOT, source, known["sdk"], verify)
    with (context / roles["iam"]["dockerfile"]).open("a", encoding="utf-8") as dockerfile:
        dockerfile.write("\nENV LLVM_PROFILE_FILE=/tmp/iam-pilot-%m-%p.profraw\n")
    roles["iam"]["dockerfile_sha256"] = binaries.sha(context / roles["iam"]["dockerfile"])
    package.build_images(output, context, {
        "schema": SCHEMA, "harness_sha": source, "target_sha": source, "sdk_sha": known["sdk"],
        "producer_run_id": int(known["GITHUB_RUN_ID"]), "producer_attempt": int(known["GITHUB_RUN_ATTEMPT"]),
        "entrypoint_sha256": entrypoint, "cargo_profile": "dev", "instrumentation": INSTRUMENTATION,
        "roles": roles, "lane": pilot.BRANCH,
    })


def admit(root, value):
    known = proofs()
    require = binaries.require
    require(value.get("schema") == SCHEMA and value.get("lane") == pilot.BRANCH
            and value.get("target_sha") == value.get("harness_sha") == known["GITHUB_SHA"]
            and value.get("sdk_sha") == known["sdk"]
            and value.get("producer_run_id") == int(known["GITHUB_RUN_ID"])
            and value.get("producer_attempt") == int(known["GITHUB_RUN_ATTEMPT"])
            and value.get("instrumentation") == INSTRUMENTATION
            and value.get("qualification") == "PACKAGING_AND_ABI_ONLY_NOT_E2E",
            "PILOT_IMAGE_PROVENANCE")
    cleanup = binaries.strict_json((root / "verified/cleanup.json").read_bytes())
    require(cleanup.get("status") == "PASS" and cleanup.get("owned_running") == 0
            and cleanup.get("run_id") == known["GITHUB_RUN_ID"]
            and cleanup.get("run_attempt") == known["GITHUB_RUN_ATTEMPT"], "PILOT_IMAGE_CLEANUP")
    roles = value.get("roles", {})
    require(set(roles) == set(INSTRUMENTATION), "PILOT_IMAGE_ROLES")
    for role, item in roles.items():
        manifest = item["manifest"]
        require(manifest["instrumentation"] == INSTRUMENTATION[role] and manifest["role"] == role
                and manifest["source_sha"] == known["GITHUB_SHA"] and manifest["sdk_sha"] == known["sdk"]
                and item["abi"]["resolution"] == "PASS"
                and re.fullmatch(r"sha256:[a-f0-9]{64}", item["image_id"])
                and item["tag"] == f"local/k6-{role}:k6-{known['GITHUB_RUN_ID']}-{known['GITHUB_RUN_ATTEMPT']}",
                "PILOT_IMAGE_ROLE_PROVENANCE")


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "pack":
        pack(Path(sys.argv[2]), Path(sys.argv[3]))
    elif len(sys.argv) == 3 and sys.argv[1] == "close-probes":
        proofs()
        cleanup.clean_owned_probes(Path(sys.argv[2]), os.environ)
    else:
        proofs()
        raise SystemExit(campaign.main(admit_images=admit))
