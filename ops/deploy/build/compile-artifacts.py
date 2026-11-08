"""Build selected Linux deliverables once, and export only confirmed Cargo outputs."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from image_catalog import LOCAL_FULL, select


def cached_source_times(source_root=Path("."), target=None):
    """Synchronize inputs before Cargo, including after a failed partial build.

    The journal describes source timestamps, never compilation success. Writing
    it after Cargo would lose B on failure and make an A->B->A rollback unsafe.
    """
    inputs = source_root / "image-inputs.json"
    if not inputs.exists():
        return None
    target = Path(target or os.environ["CARGO_TARGET_DIR"])
    target.mkdir(parents=True, exist_ok=True)
    cache = target / ".aiforall-source-times.json"
    previous = json.loads(cache.read_text(encoding="utf-8")) if cache.exists() else {}
    # A rollback timestamp must be newer than retained fingerprints, not just
    # newer than the previous synchronization (Cargo may have failed later).
    floor = max([cache.stat().st_mtime_ns if cache.exists() else 0,
                 *(item["mtime_ns"] for item in previous.values())])
    for fingerprints in [*target.glob("*/.fingerprint"), *target.glob("*/*/.fingerprint")]:
        for directory, _, names in os.walk(fingerprints):
            for name in names:
                floor = max(floor, (Path(directory) / name).stat().st_mtime_ns)
    now = time.time_ns()
    if now <= floor:
        raise ValueError("Cargo cache clock is ahead of this build; repair the clock or select a fresh cache namespace")
    current = {}
    for item in json.loads(inputs.read_text(encoding="utf-8"))["files"]:
        path = item["path"]
        old = previous.get(path, {})
        changed = old.get("sha256") != item["sha256"]
        stamp = now if changed else old["mtime_ns"]
        os.utime(source_root / path, ns=(stamp, stamp))
        actual = (source_root / path).stat().st_mtime_ns
        if (changed and not floor < actual <= stamp) or (not changed and actual != stamp):
            raise ValueError("source filesystem timestamp precision cannot safely reuse this Cargo cache")
        current[path] = {"sha256": item["sha256"], "mtime_ns": actual}
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=target, prefix=".source-times-", delete=False) as stream:
            temporary = Path(stream.name)
            json.dump(current, stream)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, cache)
        if os.name != "nt":
            directory = os.open(target, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(directory)
            finally:
                os.close(directory)
    finally:
        if temporary is not None and temporary.exists():
            temporary.unlink()
    return current


def compile_artifacts():
    delivery = os.environ.get("DELIVERY_PROFILE", "workspace")
    if delivery == "openbao":
        names = ["github", "lazaret"]
        if any(not re.fullmatch(r"[a-f0-9]{40}", os.environ.get(key, "")) for key in ("SOURCE_REVISION", "SDK_REVISION")):
            raise ValueError("attested OpenBao delivery requires exact source and SDK revisions")
    elif delivery == "workspace":
        names = [*LOCAL_FULL, "github", "gitlab", "oauth"]
    elif delivery == "images":
        names = os.environ.get("IMAGE_TARGETS", "").split(",")
    else:
        raise ValueError("unknown delivery profile")
    if delivery == "images" and not Path("image-inputs.json").is_file():
        raise ValueError("snapshot manifest missing from the effective Docker build context")
    targets = select(names)
    profile = os.environ.get("CARGO_PROFILE", "release")
    if profile not in {"release", "dev-image"}:
        raise ValueError("unknown Linux compilation profile")
    expected = {}
    features = set()
    for item in targets.values():
        expected.update(item["outputs"])
        features.update(item.get("features", []))
    if delivery == "workspace":
        expected["oodhive-monolith"] = "oodhive-monolith"
    if len(expected) != len(set(expected.values())):
        raise ValueError("colliding executable names require separate build/export batches")
    command = ["cargo", "build", "--locked", "--profile", profile, "--message-format=json-render-diagnostics"]
    for package, binary in expected.items():
        command.extend(["-p", package, "--bin", binary])
    if features:
        command.extend(["--features", ",".join(sorted(features))])
    cached_source_times()
    outputs = {}
    process = subprocess.Popen(command, stdout=subprocess.PIPE, text=True)
    for line in process.stdout:
        record = json.loads(line)
        if record.get("reason") == "compiler-artifact" and record.get("executable"):
            binary = record["target"]["name"]
            if binary in expected.values():
                outputs[binary] = Path(record["executable"])
    if process.wait():
        raise RuntimeError("Linux Cargo build failed; no artifacts exported")
    if set(outputs) != set(expected.values()):
        raise RuntimeError("Cargo did not confirm every selected executable")
    destination = Path("/opt/release")
    destination.mkdir(parents=True, exist_ok=False)
    for binary, path in outputs.items():
        shutil.copy2(path, destination / binary)
    for name, value in {"delivery-profile": delivery, "source-revision": os.environ.get("SOURCE_REVISION", "unattested"),
                        "sdk-revision": os.environ.get("SDK_REVISION", "unattested")}.items():
        (destination / name).write_text(value, encoding="utf-8")
    (destination / "artifacts.json").write_text(json.dumps({"targets": names, "profile": profile,
        "binaries": sorted(outputs)}), encoding="utf-8")


if __name__ == "__main__":
    compile_artifacts()
