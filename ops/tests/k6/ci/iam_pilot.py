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
CI_SCHEMA = "aiforall.k6-current-ci-iam-images/v1"
AC_SCHEMA = "aiforall.k6-iam-ac-images/v1"
INSTRUMENTATION = {"iam": "llvm-source-coverage", "oauth": "none", "ext-authz": "none"}


def proofs():
    runtime.hosted()
    known = pilot.identity()
    current = known["lane"] == "ci" and os.environ.get("GITHUB_JOB") == "e2e" \
        and os.environ.get("GITHUB_EVENT_NAME") == "push" and os.environ.get("GITHUB_REF") == "refs/heads/master"
    benchmark = known["lane"] == "ac" and runtime.benchmark_arm(os.environ)
    binaries.require(runtime.pilot_hosted(os.environ) or current or benchmark, "NOT_INSTRUMENTED_E2E_JOB")
    return known


def mode(known):
    if known["lane"] == "ac":
        arm = runtime.benchmark_arm(os.environ)
        binaries.require(arm is not None, "BENCHMARK_ARM")
        return (AC_SCHEMA, "benchmark-" + arm, {**INSTRUMENTATION, "iam": "none"} if arm == "a" else INSTRUMENTATION,
                "native-e2e" if arm == "a" else "e2e")
    schema, lane = (CI_SCHEMA, "current-ci") if known["lane"] == "ci" else (SCHEMA, pilot.BRANCH)
    return schema, lane, INSTRUMENTATION, "e2e"


def pack(acquired, output):
    known = proofs()
    schema, lane, instrumentation, iam_part = mode(known)
    source = known["GITHUB_SHA"]
    output.mkdir(mode=0o700)
    context = output / "context"

    def verify(role, directory, outputs):
        require = binaries.require
        part = iam_part if role == "iam" else role
        path = acquired / part
        artifact = binaries.strict_json((path / "github-artifact.json").read_bytes())
        require(re.fullmatch(r"sha256:[a-f0-9]{64}", artifact["digest"]), "PILOT_ARTIFACT_DIGEST")
        if known["lane"] == "ci" and role != "iam":
            provenance = {
                "repository": known["repository"], "repository_id": int(known["repository_id"]),
                "workflow_id": artifact["workflow_id"], "run_id": int(known["GITHUB_RUN_ID"]),
                "run_attempt": int(known["GITHUB_RUN_ATTEMPT"]), "source_sha": source, "sdk_sha": known["sdk"],
                "role": role, "cargo_lock_sha256": known["lock"],
                "inventory_sha256": binaries.sha(ROOT / "ops/deploy/build/targets.json"),
            }
            return binaries.unpack(path / "bundle.tar.gz", directory, provenance, outputs), artifact
        rows = pilot.unpack(path / "bundle.tar.gz", directory, pilot.part_identity(known, part), part,
                            set(outputs.values()))
        for row in rows:
            inspect = binaries.coverage_elf if instrumentation[role] != "none" else binaries.elf
            inspect(directory / row["name"])
        return {"role": role, "source_sha": source, "sdk_sha": known["sdk"], "files": rows,
                "instrumentation": instrumentation[role]}, artifact

    roles, entrypoint = package.stage_context(ROOT, context, ROOT, source, known["sdk"], verify)
    if instrumentation["iam"] != "none":
        with (context / roles["iam"]["dockerfile"]).open("a", encoding="utf-8") as dockerfile:
            dockerfile.write("\nENV LLVM_PROFILE_FILE=/tmp/iam-pilot-%m-%p.profraw\n")
        roles["iam"]["dockerfile_sha256"] = binaries.sha(context / roles["iam"]["dockerfile"])
    prebuilt = None
    base_receipt = {}
    if known["lane"] == "ac":
        import iam_ac_runtime
        prebuilt, base = iam_ac_runtime.load(acquired, output, context, known, roles)
        base_receipt["benchmark_base"] = base
    package.build_images(output, context, {
        "schema": schema,
        "harness_sha": source, "target_sha": source, "sdk_sha": known["sdk"],
        "producer_run_id": int(known["GITHUB_RUN_ID"]), "producer_attempt": int(known["GITHUB_RUN_ATTEMPT"]),
        "entrypoint_sha256": entrypoint, "cargo_profile": "dev", "instrumentation": instrumentation,
        "roles": roles, "lane": lane, **base_receipt,
    }, prebuilt_base=prebuilt)


def admit(root, value):
    known = proofs()
    require = binaries.require
    schema, lane, instrumentation, _ = mode(known)
    require(value.get("schema") == schema and value.get("lane") == lane
            and value.get("target_sha") == value.get("harness_sha") == known["GITHUB_SHA"]
            and value.get("sdk_sha") == known["sdk"]
            and value.get("producer_run_id") == int(known["GITHUB_RUN_ID"])
            and value.get("producer_attempt") == int(known["GITHUB_RUN_ATTEMPT"])
            and value.get("instrumentation") == instrumentation
            and value.get("qualification") == "PACKAGING_AND_ABI_ONLY_NOT_E2E",
             "PILOT_IMAGE_PROVENANCE")
    if known["lane"] == "ac":
        base = value.get("benchmark_base", {})
        require(set(base) == {"id", "digest", "image_id", "producer_job_id"}
                and all(type(base[key]) is int and base[key] > 0 for key in ("id", "producer_job_id"))
                and all(re.fullmatch(r"sha256:[a-f0-9]{64}", base[key]) for key in ("digest", "image_id")),
                "AC_RUNTIME_BASE_PROVENANCE")
    cleanup = binaries.strict_json((root / "verified/cleanup.json").read_bytes())
    require(cleanup.get("status") == "PASS" and cleanup.get("owned_running") == 0
            and cleanup.get("run_id") == known["GITHUB_RUN_ID"]
            and cleanup.get("run_attempt") == known["GITHUB_RUN_ATTEMPT"], "PILOT_IMAGE_CLEANUP")
    roles = value.get("roles", {})
    require(set(roles) == set(instrumentation), "PILOT_IMAGE_ROLES")
    for role, item in roles.items():
        manifest = item["manifest"]
        require(manifest["instrumentation"] == instrumentation[role] and manifest["role"] == role
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
