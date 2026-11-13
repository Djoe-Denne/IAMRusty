"""Pinned complete Chromium distribution and private NSS trust, no browser probe."""
import hashlib
import json
from pathlib import Path
import zipfile

from binaries import ROOT, require, sha


def tools(directory):
    directory = Path(directory).resolve()
    lock = json.loads((ROOT / "ops/tests/k6/config/tools.lock.json").read_text(encoding="utf-8"))
    for name, archive in (("k6", "k6.tar.gz"), ("chromium", "chromium.zip")):
        require(sha(directory / archive) == lock[name]["archive_sha256"], "E2E_TOOL_ARCHIVE_PIN")
        require(sha(directory / lock[name]["executable"]) == lock[name]["executable_sha256"], "E2E_TOOL_EXECUTABLE_PIN")
    expected = set()
    with zipfile.ZipFile(directory / "chromium.zip") as archive:
        for member in archive.infolist():
            if member.is_dir():
                continue
            path = directory / member.filename
            require(path.is_relative_to(directory) and not path.is_symlink() and path.is_file(), "E2E_BROWSER_BUNDLE_PATH")
            expected.add(path)
            with archive.open(member) as source, path.open("rb") as actual:
                while chunk := source.read(1024 * 1024):
                    require(chunk == actual.read(len(chunk)), "E2E_BROWSER_BUNDLE_CHANGED")
                require(actual.read(1) == b"", "E2E_BROWSER_BUNDLE_CHANGED")
    browser_root = directory / Path(lock["chromium"]["executable"]).parts[0]
    require({path for path in browser_root.rglob("*") if path.is_file()} == expected, "E2E_BROWSER_BUNDLE_EXTRA_FILES")
    return {name: {"path": str(directory / lock[name]["executable"]),
                   "sha256": lock[name]["executable_sha256"], "version": lock[name]["version"]}
            for name in ("k6", "chromium")}


def trust(home, certificate, nickname, cli):
    # Chromium M146+ default; no legacy/global profile is adopted.
    database = Path(home) / ".local/share/pki/nssdb"
    require(not database.exists(), "E2E_NSS_PROFILE_COLLISION")
    database.mkdir(mode=0o700, parents=True)
    cli("certutil", ["-N", "--empty-password", "-d", "sql:" + str(database)])
    cli("certutil", ["-A", "-d", "sql:" + str(database), "-n", nickname, "-t", "C,,", "-i", str(certificate)])
    exported = cli("certutil", ["-L", "-d", "sql:" + str(database), "-n", nickname, "-a"])
    actual = cli("openssl", ["x509", "-outform", "DER"], stdin=exported)
    expected = cli("openssl", ["x509", "-in", str(certificate), "-outform", "DER"])
    require(actual == expected, "E2E_NSS_CA_MISMATCH")
    return hashlib.sha256(actual).hexdigest()
