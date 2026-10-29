#!/usr/bin/env python3
"""Explicit acquisition only: verify pinned archives, safely extract, never execute."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import stat
import sys
import tarfile
import urllib.request
import zipfile

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / "lib"))
from safety import Rejected, new_output, strict_json

MAX_ARCHIVE = 300 * 1024 * 1024
MAX_EXPANDED = 1200 * 1024 * 1024


def member_path(root, name):
    relative = PurePosixPath(name)
    if relative.is_absolute() or ".." in relative.parts or "\\" in name:
        raise Rejected("TOOL_UNPINNED")
    destination = root.joinpath(*relative.parts)
    if not destination.is_relative_to(root) or destination == root:
        raise Rejected("TOOL_UNPINNED")
    return destination


def ensure_directory(root, path):
    current = root
    for part in path.relative_to(root).parts:
        current = current / part
        current.mkdir(mode=0o700, exist_ok=True)


def extract(root, raw, kind):
    archive = tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") if kind == "k6" else zipfile.ZipFile(io.BytesIO(raw))
    entries = archive.getmembers() if kind == "k6" else archive.infolist()
    if len(entries) > 4096:
        raise Rejected("TOOL_UNPINNED")
    expanded = 0
    for entry in entries:
        name = entry.name if kind == "k6" else entry.filename
        destination = member_path(root, name)
        directory = entry.isdir() if kind == "k6" else entry.is_dir()
        mode = entry.mode if kind == "k6" else entry.external_attr >> 16
        size = entry.size if kind == "k6" else entry.file_size
        if (kind == "k6" and not (entry.isfile() or directory)) or stat.S_ISLNK(mode):
            raise Rejected("TOOL_UNPINNED")
        expanded += size
        if expanded > MAX_EXPANDED:
            raise Rejected("TOOL_UNPINNED")
        if directory:
            ensure_directory(root, destination)
            continue
        ensure_directory(root, destination.parent)
        source = archive.extractfile(entry) if kind == "k6" else archive.open(entry)
        with source, destination.open("xb") as target:
            while block := source.read(1024 * 1024):
                target.write(block)
        destination.chmod(0o500 if mode & 0o111 else 0o400)
    archive.close()


def acquire(output):
    lock = strict_json((HERE / "config/tools.lock.json").read_bytes())
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    for name in ("k6", "chromium"):
        entry = lock[name]
        # URLs are fixed public reviewed lock entries, never campaign values.
        with opener.open(entry["url"], timeout=90) as response:
            if not response.url.startswith("https://"):
                raise Rejected("TOOL_UNPINNED")
            raw = response.read(MAX_ARCHIVE + 1)
        if len(raw) > MAX_ARCHIVE or hashlib.sha256(raw).hexdigest() != entry["archive_sha256"]:
            raise Rejected("TOOL_UNPINNED")
        archive = output / ("k6.tar.gz" if name == "k6" else "chromium.zip")
        with archive.open("xb") as stream:
            stream.write(raw)
        archive.chmod(0o400)
        extract(output, raw, name)
        if hashlib.sha256((output / entry["executable"]).read_bytes()).hexdigest() != entry["executable_sha256"]:
            raise Rejected("TOOL_UNPINNED")


class ClosedParser(argparse.ArgumentParser):
    def error(self, message):
        print('{"acquisition":"FAIL","diagnostic":"INPUT_INVALID","version":1}')
        raise SystemExit(2)


def main():
    # Provisioning is intentionally separate from run/browser/tools/validate.
    parser = ClosedParser(description=__doc__)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        if sys.platform != "linux" or os.getuid() == 0:
            raise Rejected("PLATFORM_UNQUALIFIED", True)
        output = new_output(args.output)
        acquire(output)
        print('{"acquisition":"VERIFIED_NOT_EXECUTED","version":1}')
        return 0
    except BaseException:
        # Native network/archive exceptions can include URLs: never echo them.
        print('{"acquisition":"FAIL","diagnostic":"TOOL_UNPINNED","version":1}')
        return 1


if __name__ == "__main__":
    sys.exit(main())
