"""Pinned complete Chromium distribution and private NSS trust, no browser probe."""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import zipfile

from binaries import ROOT, require, sha

LOG_LIMIT = 65536


def private_log_diagnostic(path):
    """Closed projection of native FATALs lost by k6's startup stderr parser."""
    path = Path(path)
    try:
        info = path.lstat()
    except FileNotFoundError:
        return {"log": "ABSENT", "causes": []}
    except OSError:
        return {"log": "WITHHELD", "causes": []}
    if (not stat.S_ISREG(info.st_mode) or info.st_uid != getattr(os, "getuid", lambda: 0)()
            or info.st_size > LOG_LIMIT):
        return {"log": "WITHHELD", "causes": []}
    try:
        with path.open("rb") as stream:
            raw = stream.read(LOG_LIMIT + 1)
        if len(raw) > LOG_LIMIT:
            return {"log": "WITHHELD", "causes": []}
        text = raw.decode("utf-8", errors="strict")
    except (OSError, UnicodeError):
        return {"log": "WITHHELD", "causes": []}
    patterns = {
        "NO_USABLE_SANDBOX": r"No usable sandbox",
        "NAMESPACE_DENIED": r"Failed to move to new namespace|Failed to unshare|unshare.*Operation not permitted",
        "SUID_HELPER": r"SUID sandbox helper|setuid sandbox.*not configured|chrome-sandbox.*Permission denied",
        "CRASHPAD": r"chrome_crashpad_handler:|crashpad.*(?:failed|error|database is required)",
        "LIBRARY_LOAD": r"error while loading shared libraries|cannot open shared object file",
        "RESOURCE_EXHAUSTION": r"Resource temporarily unavailable|Cannot allocate memory|No space left on device",
        "PROFILE_LOCK": r"Failed to create a ProcessSingleton|profile.*already in use",
    }
    causes = sorted(name for name, pattern in patterns.items() if re.search(pattern, text, flags=re.I))
    return {"log": "EMPTY" if not raw else "CLASSIFIED" if causes else "UNCLASSIFIED", "causes": causes}


def preflight(pins, cli):
    executable = Path(pins["chromium"]["path"])
    result = {"executables": {}, "libraries": {}}
    for role, path in (("chrome", executable), ("crashpad", executable.parent / "chrome_crashpad_handler"),
                       ("sandbox_bundle", executable.parent / "chrome_sandbox")):
        info = path.stat()
        result["executables"][role] = {"mode": stat.S_IMODE(info.st_mode),
            "owned": info.st_uid == os.getuid(), "executable": os.access(path, os.X_OK), "symlink": path.is_symlink()}
        require(result["executables"][role]["owned"] and result["executables"][role]["executable"]
                and not result["executables"][role]["symlink"], "E2E_BROWSER_EXECUTABLE_MODE")
        if role != "sandbox_bundle":
            output = cli("ldd", [str(path)])
            result["libraries"][role] = {"resolved": b"not found" not in output.lower()}
            require(result["libraries"][role]["resolved"], "E2E_BROWSER_LIBRARY_MISSING")
    return result


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
