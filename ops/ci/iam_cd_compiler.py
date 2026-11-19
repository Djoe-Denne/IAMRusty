"""Transfer only pinned sccache disk entries within the trusted C/D benchmark."""
import argparse
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tarfile

import compiler_cache
import iam_compile_once as iam

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


def export_snapshot(cache, archive):
    with tarfile.open(archive, "x:gz", compresslevel=0) as bundle:
        paths = sorted(path for path in cache.glob("*/*/*") if entry_name(path.relative_to(cache).as_posix()))
        iam.require(all(not path.is_symlink() and not path.parent.is_symlink()
                        and not path.parent.parent.is_symlink() for path in paths), "Compiler cache symlink")
        entries = (bundle.gettarinfo(str(path), path.relative_to(cache).as_posix()) for path in paths)
        for entry in checked_entries(entries):
            with (cache / entry.name).open("rb") as stream:
                bundle.addfile(entry, stream)


def restore_snapshot(archive, cache):
    iam.require(cache.is_dir() and not cache.is_symlink() and not any(cache.iterdir()),
                "Restore requires a fresh compiler cache")
    with tarfile.open(archive, "r|gz") as bundle:
        for entry in checked_entries(bundle):
            bundle.extract(entry, cache, filter="data")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("export", "restore"))
    args = parser.parse_args()
    os.umask(0o077)
    known = iam.identity()
    iam.require(known["lane"] == "ac", "Compiler snapshot is C/D-only")
    temp = Path(os.environ["RUNNER_TEMP"])
    cache = Path(os.environ["SCCACHE_DIR"])
    iam.require(cache == temp / "iam-ac-sccache" and cache.is_dir() and not cache.is_symlink(),
                "Unexpected compiler cache directory")
    folder = temp / "iam-cd-compiler"
    archive = folder / "bundle.tar.gz"
    if args.operation == "export":
        iam.require(os.environ.get("GITHUB_JOB") == "build-d-server", "Wrong compiler exporter")
        compiler_cache.attest()
        subprocess.run(["sccache", "--stop-server"], check=True)
        folder.mkdir(mode=0o700)
        export_snapshot(cache, archive)
    else:
        iam.fetch("compiler", folder)
        restore_snapshot(archive, cache)


if __name__ == "__main__":
    main()
