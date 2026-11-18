"""Verify the only compiler-cache executable admitted by native E2E producers."""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import urllib.request


ROOT = Path(__file__).resolve().parents[2]
POLICY = json.loads((ROOT / ".github/ci/sccache-policy.json").read_text(encoding="utf-8"))


def expected_attestation(policy=POLICY):
    return {key: policy[key] for key in ("version", "platform", "executable_sha256")}


def attest_executable(executable, policy=POLICY):
    digest = hashlib.sha256()
    with open(executable, "rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    if digest.hexdigest() != policy["executable_sha256"]:
        raise ValueError("Unapproved sccache executable digest")
    result = subprocess.run(
        [executable, "--version"], capture_output=True, text=True, check=True,
    )
    if result.stdout.strip() != "sccache " + policy["version"]:
        raise ValueError("Unapproved sccache version")
    return expected_attestation(policy)


def attest():
    executable = shutil.which("sccache")
    if not executable:
        raise ValueError("Pinned sccache executable missing")
    return attest_executable(executable)


def install_archive(raw, directory, policy=POLICY, inspect=attest_executable):
    """Authenticate before parsing; expose only the authenticated executable."""
    if hashlib.sha256(raw).hexdigest() != policy["archive_sha256"]:
        raise ValueError("Unapproved sccache archive digest")
    name = f"sccache-v{policy['version']}-{policy['platform']}/sccache"
    with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as archive:
        matches = [item for item in archive.getmembers() if item.name == name]
        if len(matches) != 1 or not matches[0].isfile() or matches[0].size > 64 * 1024 * 1024:
            raise ValueError("Invalid sccache release executable")
        binary = archive.extractfile(matches[0]).read()
    if hashlib.sha256(binary).hexdigest() != policy["executable_sha256"]:
        raise ValueError("Unapproved sccache executable digest")
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=False)
    executable = directory / "sccache"
    with executable.open("xb") as stream:
        stream.write(binary)
    executable.chmod(0o755)
    value = inspect(executable, policy)
    # Only the reviewed binary's otherwise-empty directory enters PATH. No
    # release-supplied Python, Cargo or shell can intercept verification.
    return value, executable


def install(directory):
    asset = f"sccache-v{POLICY['version']}-{POLICY['platform']}.tar.gz"
    url = f"https://github.com/mozilla/sccache/releases/download/v{POLICY['version']}/{asset}"
    with urllib.request.urlopen(url, timeout=120) as response:
        raw = response.read(64 * 1024 * 1024 + 1)
    if len(raw) > 64 * 1024 * 1024:
        raise ValueError("Oversized sccache release archive")
    value, executable = install_archive(raw, directory)
    with open(os.environ["GITHUB_PATH"], "a", encoding="utf-8") as stream:
        stream.write(str(executable.parent) + "\n")
    return value


def statistics():
    # There is no upstream post hook. Even a failure-path statistics request
    # authenticates the executable before it can be invoked.
    executable = shutil.which("sccache")
    if not executable:
        raise ValueError("Pinned sccache executable missing")
    attest_executable(executable)
    result = subprocess.run([executable, "--show-stats"], check=False)
    if result.returncode:
        print("Compiler-cache statistics unavailable; build outcome is unchanged.")
    return 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("verify", "install", "stats"), nargs="?", default="verify")
    parser.add_argument("--directory", type=Path)
    args = parser.parse_args()
    if args.command == "stats":
        statistics()
    else:
        value = install(args.directory) if args.command == "install" else attest()
        print(json.dumps(value, sort_keys=True))
