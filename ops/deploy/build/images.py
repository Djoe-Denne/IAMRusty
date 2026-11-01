"""Portable image plan/build; publication and deployment are separate operations."""
import argparse
from datetime import datetime, timezone
from hashlib import sha256
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from image_catalog import FRESHNESS_POLICY, LOCAL_FULL, build_context, select
from image_sources import ROOT, capture, require_unchanged, target_identity


def record_image(image_id, owner, *, opaque=False):
    if not re.fullmatch(r"sha256:[a-f0-9]{64}", image_id) and not (opaque and image_id == "UNKNOWN"):
        raise ValueError("Docker did not return a full image ID")
    now = datetime.now(timezone.utc)
    directory = ROOT / ".tmp"
    directory.mkdir(exist_ok=True)
    with (directory / ("docker-resources-" + now.strftime("%Y%m%d") + ".log")).open("a", encoding="utf-8") as stream:
        stream.write(json.dumps({"timestamp": now.isoformat(), "event": "created", "kind": "image",
            "id": image_id, "owner": owner, "safe_to_delete": "ask" if opaque else "no",
            "reason": "opaque failed build; inspect before any cleanup" if opaque else "retained delivery image/cache"}) + "\n")
        stream.flush()
        os.fsync(stream.fileno())


def command(args, **kwargs):
    result = subprocess.run(args, **kwargs)
    if result.returncode:
        raise RuntimeError("image command failed: " + args[0])
    return result


def inspect(reference):
    result = subprocess.run(["docker", "image", "inspect", reference], capture_output=True, text=True, timeout=30)
    if result.returncode:
        return None
    values = json.loads(result.stdout)
    if len(values) != 1 or not re.fullmatch(r"sha256:[a-f0-9]{64}", values[0]["Id"]):
        raise ValueError("unexpected local image identity")
    return values[0]


def build(context, recipe, reference, arguments, labels, work, owner, *, builder, cold=False):
    iid = work / "image-id"
    if iid.exists():
        iid.unlink()
    args = ["docker", "buildx", "build", "--builder", builder, "--platform", "linux/amd64", "--load", "--provenance=false",
            "--iidfile", str(iid), "--tag", reference, "--file", str(context / recipe)]
    for key, value in arguments.items():
        args.extend(["--build-arg", key + "=" + value])
    for key, value in labels.items():
        args.extend(["--label", key + "=" + value])
    if cold:
        args.extend(["--no-cache-filter", "builder"])
    args.append(str(context))
    result = subprocess.run(args)
    # Register the returned ID before subsequent validation, also on a late CLI
    # failure. No image/cache deletion is part of this tool.
    if iid.exists():
        record_image(iid.read_text().strip(), owner)
    elif result.returncode:
        record_image("UNKNOWN", owner, opaque=True)
    if result.returncode or not iid.exists():
        raise RuntimeError("Linux image build failed; no manifest published")
    value = inspect(reference)
    if value is None or value["Id"] != iid.read_text().strip():
        raise ValueError("built image ID changed before verification")
    return value


def run(args):
    if args.policy != "local" or args.revision or args.cold:
        raise ValueError("commit/cloud release is not implemented: master export, SDK gitlink export and tool pins remain gated; use local --profile release for performance builds")
    names = args.targets.split(",")
    if args.github_openbao:
        names.append("github")
    if args.oauth_proof:
        names.append("gitlab")
    targets = select(names)
    snapshot = capture()
    profile = args.profile or "dev-image"
    context_id = build_context(targets, profile)
    plan = {name: {"source": target["source"], "input": target_identity(snapshot, name)} for name, target in targets.items()}
    if args.action == "plan":
        print(json.dumps({"policy": args.policy, "freshness_policy": FRESHNESS_POLICY, "profile": profile,
                          "build_context": context_id, "snapshot": snapshot["sha256"], "targets": plan}, indent=2))
        return
    if args.output is None or args.output.exists():
        raise ValueError("build requires a new --output manifest path")
    system = command(["docker", "info", "--format", "{{.OSType}}"], capture_output=True, text=True, timeout=30)
    if system.stdout.strip() != "linux":
        raise ValueError("Linux Docker engine required; never use the Windows Cargo target")
    backend = command(["docker", "buildx", "inspect"], capture_output=True, text=True, timeout=30).stdout
    driver = re.search(r"^Driver:\s+(\S+)\s*$", backend, re.MULTILINE)
    name_match = re.search(r"^Name:\s+([A-Za-z0-9_.-]+)\s*$", backend, re.MULTILINE)
    if not driver or driver[1] != "docker" or not name_match:
        raise ValueError("use an existing Docker-driver builder for local artifact images; no global builder change is automatic")
    builder = name_match[1]
    records, pending = {}, []
    for name, target in targets.items():
        suffix = json.dumps([plan[name]["input"], context_id, args.policy])
        reference = target["source"].split(":")[0] + ":inputs-" + sha256(suffix.encode()).hexdigest()
        labels = {"aiforall.dev.source-input": plan[name]["input"], "aiforall.dev.source-policy": args.policy,
                  "aiforall.dev.freshness-policy": FRESHNESS_POLICY,
                  "aiforall.dev.cargo-profile": profile, "aiforall.dev.build-context": context_id,
                  "aiforall.dev.build-targets": ",".join(sorted(targets))}
        value = inspect(reference)
        if value and (value.get("Os") != "linux" or value.get("Architecture") != "amd64"
                      or any(value["Config"].get("Labels", {}).get(k) != v for k, v in labels.items())
                      or any(not re.fullmatch(r"[a-f0-9]{40}", value["Config"].get("Labels", {}).get(k, ""))
                              for k in ("org.opencontainers.image.revision", "aiforall.dev.sdk-revision"))):
            raise ValueError("existing content image disagrees with its provenance")
        records[name] = {"source": target["source"], "local_ref": reference, "source_input": plan[name]["input"],
                         "image_id": value["Id"] if value else None,
                         "build_revision": value["Config"]["Labels"]["org.opencontainers.image.revision"] if value else None,
                         "sdk_revision": value["Config"]["Labels"]["aiforall.dev.sdk-revision"] if value else None}
        if not value:
            pending.append((name, labels))
    scratch = ROOT / ".tmp"
    scratch.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="image-build-", dir=scratch) as temporary:
        work = Path(temporary)
        context = work / "context"
        captured = capture(destination=context)
        if captured != snapshot:
            raise ValueError("source changed during snapshot capture")
        (context / "image-inputs.json").write_text(json.dumps(snapshot), encoding="utf-8")
        # This belongs to the owned snapshot; the operator's ignore file is untouched.
        (context / ".dockerignore").write_text(".git\n**/.env\n**/.env.*\n", encoding="utf-8")
        if pending:
            # Keep Cargo feature unification stable for the requested selection.
            # Cargo decides which artifacts need rebuilding; only pending images
            # are repackaged below.
            arguments = {"DELIVERY_PROFILE": "images", "IMAGE_TARGETS": ",".join(sorted(targets)),
                         "CARGO_PROFILE": profile, "SOURCE_REVISION": snapshot["revision"], "SDK_REVISION": snapshot["sdk_revision"]}
            artifacts_ref = "local/build-artifacts:inputs-" + snapshot["sha256"] + "-" + work.name
            artifacts = build(context, "Dockerfile.build", artifacts_ref, arguments, {}, work, args.owner, builder=builder)
            for name, labels in pending:
                require_unchanged(snapshot)
                labels = {**labels, "org.opencontainers.image.revision": snapshot["revision"],
                          "aiforall.dev.sdk-revision": snapshot["sdk_revision"]}
                if inspect(artifacts_ref)["Id"] != artifacts["Id"]:
                    raise ValueError("owned artifact reference changed before packaging")
                arguments = {"BUILD_IMAGE": artifacts_ref, "SOURCE_REVISION": snapshot["revision"], "SDK_REVISION": snapshot["sdk_revision"]}
                value = build(context, targets[name]["dockerfile"], records[name]["local_ref"], arguments, labels, work, args.owner, builder=builder)
                if any(value["Config"].get("Labels", {}).get(k) != v for k, v in labels.items()):
                    raise ValueError("built image did not retain expected source labels")
                if inspect(artifacts_ref)["Id"] != artifacts["Id"]:
                    raise ValueError("owned artifact reference changed during packaging")
                records[name]["image_id"] = value["Id"]
                records[name]["build_revision"] = snapshot["revision"]
                records[name]["sdk_revision"] = snapshot["sdk_revision"]
        require_unchanged(snapshot)
        with args.output.open("x", encoding="utf-8") as stream:
            json.dump({"schema": "aiforall.image-build/v1", "policy": args.policy, "profile": profile,
                       "freshness_policy": FRESHNESS_POLICY,
                       "cold": args.cold,
                       "build_context": context_id, "build_targets": sorted(targets),
                       "builder": {"name": builder, "driver": "docker"},
                       "revision": snapshot["revision"], "sdk_revision": snapshot["sdk_revision"],
                       "snapshot": snapshot["sha256"], "targets": records}, stream, indent=2)
    print("built/reused " + str(len(records)) + " verified Linux images; no publish, deployment or E2E performed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["plan", "build"])
    parser.add_argument("--policy", choices=["local", "release"], default="local", help="release is explicitly blocked pending commit export and delivery pins")
    parser.add_argument("--revision", help="reserved for the gated commit-release implementation")
    parser.add_argument("--profile", choices=["dev-image", "release"])
    parser.add_argument("--targets", default=",".join(LOCAL_FULL))
    profiles = parser.add_mutually_exclusive_group()
    profiles.add_argument("--github-openbao", action="store_true")
    profiles.add_argument("--oauth-proof", action="store_true")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--owner", default="image-toolchain")
    parser.add_argument("--cold", action="store_true", help="reserved for the gated commit-release implementation")
    args = parser.parse_args()
    if args.cold and args.policy != "release":
        parser.error("--cold is an explicit release-only policy")
    run(args)


if __name__ == "__main__":
    main()
