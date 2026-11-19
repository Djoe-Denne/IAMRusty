"""Prepare the identical public Cargo downloads once for the hosted A/C pair."""
import argparse
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import tomllib

import iam_compile_once as iam

LIMIT = 2 * 1024**3


def registry_archive(home, dest):
    with tarfile.open(dest, "x:gz", compresslevel=1) as archive:
        for part in ("cache", "index"):
            source = home / "registry" / part
            iam.require(source.is_dir(), "Missing locked Cargo downloads")
            archive.add(source, arcname=f"registry/{part}")


def restore_registry(source, home):
    home.mkdir(mode=0o700)
    seen, size = set(), 0
    with tarfile.open(source, "r|gz") as archive:
        for member in archive:
            path = PurePosixPath(member.name)
            size += member.size
            iam.require(not path.is_absolute() and ".." not in path.parts and "\\" not in member.name
                        and path.parts[:2] in {("registry", "cache"), ("registry", "index")}
                        and (member.isfile() or member.isdir()) and not member.issparse()
                        and path not in seen and len(seen) < 10000 and size <= LIMIT,
                        "Unexpected Cargo seed member")
            seen.add(path)
            archive.extract(member, home, filter="data")
    iam.require(all((home / "registry" / part).is_dir() for part in ("cache", "index")),
                "Incomplete Cargo seed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("prepare", "restore"))
    args = parser.parse_args()
    known = iam.identity()
    iam.require(known["lane"] == "ac", "Cargo seed is A/C-only")
    os.umask(0o077)
    temp = Path(os.environ["RUNNER_TEMP"])
    home = temp / "iam-ac-cargo-home"
    archive = temp / "iam-ac-cargo-seed"
    if args.operation == "prepare":
        iam.require(os.environ.get("GITHUB_JOB") == iam.AC_PRODUCERS["cargo"][0], "Wrong Cargo seed producer")
        # No private or git-based sources are admitted to this public dependency artifact.
        lock = tomllib.loads((iam.ci.ROOT / "Cargo.lock").read_text(encoding="utf-8"))
        iam.require(all(p.get("source") in {None, "registry+https://github.com/rust-lang/crates.io-index"}
                        for p in lock["package"]), "Unexpected Cargo source")
        home.mkdir(mode=0o700)
        archive.mkdir(mode=0o700)
        subprocess.run(["cargo", "fetch", "--locked", "--target", "x86_64-unknown-linux-gnu"], cwd=iam.ci.ROOT,
                       env={**os.environ, "CARGO_HOME": str(home), "CARGO_REGISTRIES_CRATES_IO_PROTOCOL": "sparse"},
                       check=True)
        registry_archive(home, archive / "bundle.tar.gz")
    else:
        iam.fetch("cargo", archive)
        restore_registry(archive / "bundle.tar.gz", home)
        print("A/C Cargo seed:", json.loads((archive / "github-artifact.json").read_bytes())["digest"])
        with Path(os.environ["GITHUB_ENV"]).open("a", encoding="utf-8") as file:
            print(f"CARGO_HOME={home}", file=file)
            print("CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse", file=file)
            print("CARGO_NET_OFFLINE=true", file=file)


if __name__ == "__main__":
    main()
