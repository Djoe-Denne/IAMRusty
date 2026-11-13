"""Package verified CI debug binaries using existing Dockerfiles, without Cargo."""
import argparse
from datetime import datetime, timezone
import gzip
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

from binaries import ROOT, ROLES, Refused, command, inventory, require, sha, strict_json, unpack


def runtime_lock(root=ROOT):
    value = strict_json((Path(root) / "ops/tests/k6/ci/images.lock.json").read_bytes())
    require(value["schema"] == "aiforall.k6-runtime/v1" and value["platform"] == "linux/amd64"
            and value["producer_os"] == "ubuntu-24.04" and value["openssl_package"] == "libssl3t64",
            "RUNTIME_CONTRACT")
    require(re.fullmatch(r"ubuntu:24\.04@sha256:[a-f0-9]{64}", value["runtime"]), "RUNTIME_UNPINNED")
    return value


def prepare(acquired, source, context, harness=ROOT):
    acquired, source, context, harness = (Path(path).resolve() for path in (acquired, source, context, harness))
    require(not context.exists(), "PACKAGING_CONTEXT_COLLISION")
    selected = strict_json((acquired / "selection.json").read_bytes())
    require(selected.get("schema") == "aiforall.k6-selection/v1", "SELECTION_SCHEMA")
    target_sha = command(["git", "rev-parse", "HEAD"], source)
    require(target_sha == selected["source_sha"], "TARGET_CHECKOUT_MISMATCH")
    sdk_sha = command(["git", "rev-parse", "HEAD:rustycog"], source)
    harness_sha = command(["git", "rev-parse", "HEAD"], harness)
    require(all(re.fullmatch(r"[a-f0-9]{40}", value) for value in (target_sha, sdk_sha, harness_sha)), "CHECKOUT_SHA")
    context.mkdir(mode=0o700)
    release = context / "release"
    release.mkdir()
    records = {}
    base = {key: selected[key] for key in ("repository", "repository_id", "workflow_id", "run_id", "run_attempt", "source_sha")}
    base.update(sdk_sha=sdk_sha, cargo_lock_sha256=sha(source / "Cargo.lock"),
                inventory_sha256=sha(source / "ops/deploy/build/targets.json"))
    target_inventory = strict_json((source / "ops/deploy/build/targets.json").read_bytes())["variable"]["inventory"]["default"]["targets"]
    harness_inventory = strict_json((harness / "ops/deploy/build/targets.json").read_bytes())["variable"]["inventory"]["default"]["targets"]
    for role in ROLES:
        outputs = inventory(source, role)
        require(outputs == inventory(harness, role) and
                target_inventory[role]["dockerfile"] == harness_inventory[role]["dockerfile"], "HARNESS_TARGET_SEAM_MISMATCH")
        directory = context / (role + "-verified")
        manifest = unpack(acquired / (role + ".tar.gz"), directory, {**base, "role": role}, outputs)
        for name in outputs.values():
            require(not (release / name).exists(), "DUPLICATE_DELIVERY_BINARY")
            shutil.copyfile(directory / name, release / name)
            (release / name).chmod(0o755)
        dockerfile = target_inventory[role]["dockerfile"]
        # Only these closed packaging recipes come from the trusted harness.
        require(dockerfile in ("services/IAMRusty/Dockerfile", "services/OAuthConnect/Dockerfile",
                               "ops/deploy/build/Dockerfile.ext-authz"), "PACKAGING_DOCKERFILE")
        target = context / dockerfile
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(harness / dockerfile, target)
        records[role] = {"manifest": manifest, "dockerfile": dockerfile,
                         "dockerfile_sha256": sha(target), "artifact": selected["artifacts"][role]}
    entrypoint = "services/IAMRusty/docker-entrypoint.sh"
    shutil.copyfile(source / entrypoint, context / entrypoint)
    shutil.copyfile(harness / "ops/tests/k6/ci/Dockerfile.artifacts", context / "Dockerfile.artifacts")
    (context / ".dockerignore").write_text("**\n!release/\n!release/**\n!services/\n!services/IAMRusty/\n"
                                          "!services/IAMRusty/docker-entrypoint.sh\n", encoding="utf-8")
    for name, value in (("source-revision", target_sha), ("sdk-revision", sdk_sha), ("delivery-profile", "images")):
        (release / name).write_text(value + "\n", encoding="ascii")
    return {"schema": "aiforall.k6-images/v1", "harness_sha": harness_sha, "target_sha": target_sha,
            "sdk_sha": sdk_sha, "producer_run_id": selected["run_id"],
            "producer_attempt": selected["run_attempt"], "cargo_profile": "dev", "instrumentation": "none",
            "entrypoint_sha256": sha(context / entrypoint), "roles": records}


class Packager:
    def __init__(self, output):
        self.output = Path(output)
        self.known_images = set(command(["docker", "image", "ls", "--no-trunc", "--quiet"]).splitlines())
        self.owned_containers = set()
        self.removed = []
        self.registry = ROOT / ".tmp" / ("docker-resources-" + datetime.now(timezone.utc).strftime("%Y%m%d") + ".log")
        self.registry.parent.mkdir(exist_ok=True)

    def record(self, event, kind, identity, safe, **facts):
        require(re.fullmatch(r"(?:sha256:)?[a-f0-9]{64}", identity), "DOCKER_RECEIPT_ID")
        record = dict(timestamp=datetime.now(timezone.utc).isoformat(), event=event, kind=kind, id=identity,
                      safe_to_delete=safe, owner="k6-ci-packaging", run_id=os.environ["GITHUB_RUN_ID"],
                      run_attempt=os.environ["GITHUB_RUN_ATTEMPT"], **facts)
        with self.registry.open("a", encoding="utf-8") as stream:
            stream.write(json.dumps(record, sort_keys=True) + "\n")

    def observe_images(self):
        current = set(command(["docker", "image", "ls", "--no-trunc", "--quiet"]).splitlines())
        for identity in sorted(current - self.known_images):
            self.record("created", "image", identity, "no", reason="active_delivery_or_runtime_base")
        self.known_images = current

    def build(self, context, dockerfile, tag, args):
        iid = self.output / (tag.rsplit(":", 1)[-1] + ".iid")
        argv = ["docker", "build", "--platform", "linux/amd64", "--iidfile", str(iid), "--tag", tag,
                "--file", str(Path(context) / dockerfile)]
        for name, value in args.items():
            argv += ["--build-arg", name + "=" + value]
        try:
            # Public closed build context, no secret inputs; retain Docker build
            # diagnostics in job logs, never in campaign result artifacts.
            result = subprocess.run([*argv, str(context)], timeout=900)
            require(result.returncode == 0, "IMAGE_PACKAGING_FAILED")
        finally:
            self.observe_images()
        identity = iid.read_text(encoding="ascii").strip()
        require(re.fullmatch(r"sha256:[a-f0-9]{64}", identity), "IMAGE_RECEIPT_MISSING")
        return identity

    def abi(self, image, paths):
        identity = command(["docker", "create", "--read-only", "--network", "none", "--cap-drop", "ALL",
                            "--security-opt", "no-new-privileges",
                            "--label", "aiforall.dev/k6-packaging-run=" + os.environ["GITHUB_RUN_ID"],
                            "--label", "aiforall.dev/k6-packaging-attempt=" + os.environ["GITHUB_RUN_ATTEMPT"],
                            "--entrypoint", "/bin/sh", image,
                            "-ec", 'for binary do ldd -v "$binary"; done', "abi-check", *paths])
        self.record("created", "container", identity, "no", purpose="networkless_ELF_resolution")
        self.owned_containers.add(identity)
        try:
            output = command(["docker", "start", "--attach", identity])
            require(not re.search(r"not found|undefined symbol", output, flags=re.I), "RUNTIME_ABI_UNRESOLVED")
            state = strict_json(command(["docker", "inspect", "--format", "{{json .State}}", identity]))
            require(not state["Running"] and state["ExitCode"] == 0, "RUNTIME_ABI_FAILED")
            return {"paths": paths, "resolution": "PASS", "ldd_sha256": hashlib_text(output)}
        finally:
            self.retire(identity)

    def retire(self, identity):
        require(identity in self.owned_containers, "FOREIGN_CONTAINER")
        state = strict_json(command(["docker", "inspect", "--format", "{{json .State}}", identity]))
        if state["Running"]:
            command(["docker", "stop", "--time", "10", identity])
        state = strict_json(command(["docker", "inspect", "--format", "{{json .State}}", identity]))
        require(not state["Running"], "PACKAGING_CONTAINER_RUNNING")
        self.record("stopped", "container", identity, "no")
        # Networkless, stateless probe owned by this run; remove only that ID.
        self.record("reclassified", "container", identity, "yes", reason="owned_stateless_probe_stopped")
        command(["docker", "rm", identity])
        self.record("removed", "container", identity, "yes")
        self.owned_containers.remove(identity)
        self.removed.append(identity)


def hashlib_text(text):
    import hashlib
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def save_images(tags, path):
    with path.open("xb") as target:
        process = subprocess.Popen(["docker", "image", "save", *tags], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        try:
            with gzip.GzipFile(fileobj=target, mode="wb", compresslevel=1, mtime=0) as compressed:
                shutil.copyfileobj(process.stdout, compressed, 1024 * 1024)
            require(process.wait(timeout=60) == 0, "IMAGE_EXPORT_FAILED")
        finally:
            process.stdout.close()
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=10)


def package(acquired, source, output):
    output = Path(output)
    require(output.is_absolute() and not output.exists(), "PACKAGING_OUTPUT_EXISTS")
    output.mkdir(mode=0o700)
    context = output / "context"
    provenance = prepare(acquired, source, context)
    lock = runtime_lock()
    provenance["runtime"] = lock["runtime"]
    suffix = "k6-" + os.environ["GITHUB_RUN_ID"] + "-" + os.environ["GITHUB_RUN_ATTEMPT"]
    require(re.fullmatch(r"k6-[1-9][0-9]*-[1-9][0-9]*", suffix), "PACKAGE_RUN_ID")
    packager = Packager(output)
    try:
        artifact_image = "local/k6-artifacts:" + suffix
        packager.build(context, "Dockerfile.artifacts", artifact_image, {"RUNTIME_IMAGE": lock["runtime"]})
        tags = []
        for role, record in provenance["roles"].items():
            tag = "local/k6-" + role + ":" + suffix
            identity = packager.build(context, record["dockerfile"], tag,
                                     {"BUILD_IMAGE": artifact_image, "RUNTIME_IMAGE": lock["runtime"],
                                      "RUNTIME_OPENSSL_PACKAGE": lock["openssl_package"],
                                      "SOURCE_REVISION": provenance["target_sha"], "SDK_REVISION": provenance["sdk_sha"]})
            paths = ["/app/iam-service", "/app/migration"] if role == "iam" else [
                "/app/oauth-connect-service" if role == "oauth" else "/app/ext-authz"]
            record.update(image_id=identity, tag=tag, abi=packager.abi(tag, paths))
            tags.append(tag)
        archive = output / "images.tar.gz"
        save_images(tags, archive)
        provenance["archive_sha256"] = sha(archive)
        provenance["qualification"] = "PACKAGING_AND_ABI_ONLY_NOT_E2E"
    except Refused as error:
        provenance["failure"] = str(error)
        raise
    finally:
        for identity in list(packager.owned_containers):
            packager.retire(identity)
        require(not packager.owned_containers, "PACKAGING_CLEANUP_INCOMPLETE")
        provenance["cleanup"] = {"owned_running": 0, "removed_probes": packager.removed,
                                 "images": "retained_on_disposable_host_and_exported_on_success"}
        if packager.registry.exists():
            shutil.copyfile(packager.registry, output / "docker-resources.jsonl")
        (output / "provenance.json").write_text(json.dumps(provenance, sort_keys=True), encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--acquired", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        package(args.acquired, args.source, args.output)
    except Refused as error:
        raise SystemExit("Verified image packaging failed: " + str(error)) from None
    except Exception:
        raise SystemExit("Verified image packaging failed; no compilation fallback") from None
    print("Three images exported; four ELF binaries resolved; not an E2E verdict")


if __name__ == "__main__":
    main()
