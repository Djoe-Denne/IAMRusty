"""Opt-in operator failure evidence, outside Git; never log argv, stdin or env."""
from contextlib import contextmanager
from contextvars import ContextVar
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import sys
import uuid

CURRENT = ContextVar("operator_diagnostics", default=None)
LIMIT = 1024 * 1024


def _directory(scope):
    if scope["path"] is None:
        # Reuse the existing owner-only Windows/POSIX guards. Lazy import keeps
        # ordinary target calls and successful operations entirely unchanged.
        from local_registry import pki
        root = pki.checked_path(scope["root"])
        pki.private_directory(root, create=not root.exists())
        name = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ-") + uuid.uuid4().hex[:12]
        path = root / name
        pki.private_directory(path, create=True)
        scope["path"] = path
    return scope["path"]


def capture_failure(command=None, stdout=b"", stderr=b"", returncode=None, timeout=False, reason=None):
    scope = CURRENT.get()
    if scope is None:
        return
    try:
        directory = _directory(scope)
        scope["sequence"] += 1
        prefix = str(scope["sequence"]).zfill(3)
        tool = Path(command[0]).name if isinstance(command, (list, tuple)) and command else None
        verbs = {"get", "apply", "create", "patch", "wait", "rollout", "start", "stop", "inspect", "info", "ps", "pull", "copy", "load"}
        verb = next((part for part in command[1:] if part in verbs), None) if isinstance(command, (list, tuple)) else None
        metadata = {"stage": scope["stage"], "tool": tool, "returncode": returncode,
                    "operation": verb,
                    "timeout": timeout, "timestamp": datetime.now(timezone.utc).isoformat(),
                    "reason": reason, "outputs": {}}
        for name, contents in (("stdout", stdout), ("stderr", stderr)):
            contents = contents or b""
            if isinstance(contents, str):
                contents = contents.encode("utf-8")
            tail = contents[-LIMIT:]
            file = directory / (prefix + "." + name + ".bin")
            with file.open("xb") as stream:
                stream.write(tail)
            file.chmod(0o600)
            metadata["outputs"][name] = {"bytes": len(contents), "captured_bytes": len(tail),
                                         "truncated": len(contents) > LIMIT, "selection": "tail"}
        file = directory / (prefix + ".json")
        with file.open("x", encoding="utf-8") as stream:
            json.dump(metadata, stream, indent=2)
        file.chmod(0o600)
    except Exception:
        # Never replace the actual failure or print its possibly sensitive text.
        print("Private diagnostics unavailable; original failure retained", file=sys.stderr)


@contextmanager
def operation(stage, directory=None):
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]{0,63}", stage):
        raise ValueError("closed diagnostic stage required")
    scope = {"stage": stage, "root": Path(directory) if directory else Path.home() / ".aiforall-ops-diagnostics",
             "path": None, "sequence": 0}
    token = CURRENT.set(scope)
    try:
        yield
    except BaseException as error:
        if not isinstance(error, SystemExit) or error.code not in (None, 0):
            if not scope["sequence"]:
                code = 130 if isinstance(error, KeyboardInterrupt) else error.code if isinstance(error, SystemExit) and isinstance(error.code, int) else 1
                # Exception strings (notably subprocess errors) may embed argv,
                # stdin or URLs. Preserve the type/code, never arbitrary text.
                capture_failure(returncode=code, reason=type(error).__name__)
            if scope["path"]:
                print("Private diagnostics retained: " + str(scope["path"]), file=sys.stderr)
        raise
    finally:
        CURRENT.reset(token)
