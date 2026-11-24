"""Run one synchronous AIForAll qualification under the shared Windows slot."""

import csv
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid


LOCK = Path("I:/cargo-target/AIForAll.qualification.lock")
BUSY = 75
UNCERTAIN = 70
TOOLS = {
    "cargo.exe", "cargo-clippy.exe", "cargo-fmt.exe", "clippy-driver.exe",
    "rustc.exe", "rustfmt.exe", "rustdoc.exe", "link.exe",
}


def windows_compiler_active():
    result = subprocess.run(
        ["tasklist.exe", "/FO", "CSV", "/NH"], capture_output=True,
        check=True, text=True, errors="replace", timeout=15,
    )
    rows = list(csv.reader(result.stdout.splitlines()))
    if not any(len(row) >= 5 and row[1].strip().isdigit() for row in rows):
        raise RuntimeError("Could not verify the compiler process inventory")
    return any(row and row[0].casefold() in TOOLS for row in rows)


def run_child(command):
    return subprocess.run(command).returncode


def run_with_slot(lock, command, *, active=windows_compiler_active, launch=run_child):
    ticket = (json.dumps({
        "lease": uuid.uuid4().hex,
        "pid": os.getpid(),
        "worktree": str(Path.cwd()),
        "acquired_utc": datetime.now(timezone.utc).isoformat(),
    }) + "\n").encode("utf-8")
    try:
        with lock.open("xb") as f:
            f.write(ticket)
            f.flush()
            os.fsync(f.fileno())
    except FileExistsError:
        print(f"Cargo slot busy or awaiting recovery: {lock}", file=sys.stderr)
        return BUSY

    try:
        # Also reject already running compiler commands that did not opt in.
        if active():
            result = BUSY
        else:
            result = launch(command)
            if active():
                raise RuntimeError("Compiler processes remain after the validation command")
        if lock.read_bytes() != ticket:
            raise RuntimeError("Cargo lease identity changed")
        lock.unlink()
        if result == BUSY:
            print("Cargo slot unavailable (or child returned 75)", file=sys.stderr)
        return result
    except (KeyboardInterrupt, OSError, RuntimeError, subprocess.SubprocessError):
        # The child may still hold the target. Do not clear a lease on ambiguity.
        print(f"Cargo slot release not proven; inspect {lock} before recovery", file=sys.stderr)
        return UNCERTAIN


def main(args):
    if len(args) < 2 or args[0] != "--":
        print("Usage: python cargo_slot.py -- <synchronous program> [arguments]", file=sys.stderr)
        return 2
    if os.name != "nt" or not LOCK.parent.is_dir():
        print(f"Required Windows Cargo slot location is unavailable: {LOCK.parent}", file=sys.stderr)
        return UNCERTAIN
    if any(os.environ.get(k) for k in ("CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR", "CARGO_BUILD_JOBS")):
        print("Unexpected Cargo target/jobs override", file=sys.stderr)
        return UNCERTAIN
    try:
        return run_with_slot(LOCK, args[1:])
    except OSError:
        print(f"Could not initialize the Cargo lease at {LOCK}; inspect before retry", file=sys.stderr)
        return UNCERTAIN


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
