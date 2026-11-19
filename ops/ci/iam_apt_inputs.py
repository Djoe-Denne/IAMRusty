"""Transfer an authenticated Noble native-package set only within the benchmark attempt."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile

import iam_compile_once as iam

REQUIRED = {"libcurl4-openssl-dev", "libcurl4t64"}
MAX_BYTES = 64 * 1024**2
MAX_PACKAGES = 32
DEB = re.compile(r"[a-z0-9][a-z0-9+.-]*_[a-zA-Z0-9.+~%:-]+_(amd64|all)\.deb")


def checked_entries(entries):
    seen, total = set(), 0
    for entry in entries:
        total += entry.size
        iam.require(DEB.fullmatch(entry.name) and entry.name not in seen and entry.isfile()
                    and not entry.issparse() and 0 < entry.size and total <= MAX_BYTES
                    and len(seen) < MAX_PACKAGES, "Unexpected Ubuntu package archive member")
        seen.add(entry.name)
        yield entry
    iam.require(seen, "Empty Ubuntu package seed")


def version_set(paths):
    versions = {}
    for path in paths:
        package, version, arch = subprocess.check_output(
            ["dpkg-deb", "--showformat=${Package}\t${Version}\t${Architecture}\n", "--show", str(path)],
            text=True, timeout=10,
        ).strip().split("\t")
        iam.require(path.name.startswith(package + "_") and path.name.endswith("_" + arch + ".deb")
                    and package not in versions and re.fullmatch(r"[0-9][a-zA-Z0-9.+~:-]*", version),
                    "Unexpected Ubuntu package record")
        versions[package] = version
    iam.require(REQUIRED <= versions.keys()
                and versions["libcurl4-openssl-dev"] == versions["libcurl4t64"],
                "Missing or mismatched libcurl packages")
    return versions


def restore_packages(archive, destination):
    destination.mkdir(mode=0o700)
    with tarfile.open(archive, "r|gz") as bundle:
        for entry in checked_entries(bundle):
            with bundle.extractfile(entry) as source, (destination / entry.name).open("xb") as out:
                shutil.copyfileobj(source, out, 1024 * 1024)
            iam.require((destination / entry.name).stat().st_size == entry.size, "Truncated Ubuntu package")
    return sorted(destination.iterdir())


def produce(root, output):
    output.mkdir(mode=0o700)
    with tempfile.TemporaryDirectory(prefix="iam-packages-", dir=output.parent) as directory:
        staging = Path(directory)
        subprocess.run(["sudo", "bash", str(root / "ops/ci/iam_benchmark_apt.sh"), "host", str(staging)],
                       check=True, timeout=270)
        paths = sorted(staging.glob("*.deb"))
        with tarfile.open(output / "bundle.tar.gz", "x:gz", compresslevel=0) as archive:
            for member in checked_entries(archive.gettarinfo(p, p.name) for p in paths):
                with (staging / member.name).open("rb") as source:
                    archive.addfile(member, source)
        print("Prepared IAM Ubuntu packages:", json.dumps(version_set(paths), sort_keys=True), flush=True)


def install(output):
    iam.fetch("apt", output)
    paths = restore_packages(output / "bundle.tar.gz", output / "packages")
    versions = version_set(paths)
    subprocess.run(["sudo", "unshare", "--net", "--", "apt-get", "-o", "Dir::Etc::sourcelist=/dev/null",
                    "-o", "Dir::Etc::sourceparts=-", "install", "-y", "--no-install-recommends", "--no-remove",
                    *map(str, paths)], check=True, timeout=90)
    for name, version in versions.items():
        actual = subprocess.check_output(
            ["dpkg-query", "-W", "-f=${db:Status-Status} ${Version}", name], text=True, timeout=10,
        )
        iam.require(actual == f"installed {version}", "Unexpected installed Ubuntu package version")
    receipt = json.loads((output / "github-artifact.json").read_bytes())
    print(f"IAM Ubuntu seed: {receipt['digest']} {json.dumps(versions, sort_keys=True)}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("prepare", "install"))
    args = parser.parse_args()
    os.umask(0o077)
    known = iam.identity()
    iam.require(known["lane"] == "ac", "Ubuntu package seed is benchmark-only")
    output = Path(os.environ["RUNNER_TEMP"]) / "iam-warm-apt"
    if args.operation == "prepare":
        iam.require(os.environ.get("GITHUB_JOB") == iam.AC_PRODUCERS["apt"][0], "Wrong Ubuntu package producer")
        produce(iam.ci.ROOT, output)
    else:
        install(output)


if __name__ == "__main__":
    main()
