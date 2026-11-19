"""Disposable native IAM benchmark: only collect Cargo duration and fresh units."""

from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

from build import build_arguments
from pipeline import ROOT


def main():
    phase = sys.argv[1]
    source = ROOT / "services/IAMRusty/src/main.rs"
    marker = b'\nconst _: &str = "iam-native-cache-lab-leaf-edit";\n'
    current = source.read_bytes()
    if phase == "edit":
        if marker in current:
            raise SystemExit("leaf edit already applied")
        source.write_bytes(current + marker)
        return
    if phase not in {"seed", "gha", "archive", "retained-identical", "retained-edit"}:
        raise SystemExit("unknown lab phase")
    if (marker in current) != (phase in {"gha", "archive", "retained-edit"}):
        raise SystemExit("unexpected leaf source for this phase")

    counts = Counter()
    command = ["cargo", *build_arguments("iam"), "--message-format=json"]
    start = time.monotonic()
    with subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE, text=True, encoding="utf-8") as child:
        for line in child.stdout:
            if not line.startswith("{"):
                continue
            event = json.loads(line)
            if event["reason"] == "compiler-artifact":
                counts["fresh" if event["fresh"] else "rebuilt"] += 1
            elif event["reason"] == "compiler-message":
                message = event["message"]["rendered"]
                if message:
                    print(message, end="")
        status = child.wait()
    elapsed = time.monotonic() - start
    if status or not sum(counts.values()):
        raise SystemExit(status or 2)
    if phase == "retained-identical" and counts["rebuilt"]:
        raise SystemExit("same workspace unexpectedly rebuilt Cargo targets")
    evidence = json.dumps({
        "phase": phase, "cargo_seconds": round(elapsed, 3),
        "fresh": counts["fresh"], "rebuilt": counts["rebuilt"],
        "leaf_sha256": hashlib.sha256(current).hexdigest(), "argv": command,
        "run": os.environ.get("GITHUB_RUN_ID"), "sha": os.environ.get("GITHUB_SHA"),
    }, sort_keys=True)
    print("IAM_CACHE_LAB " + evidence)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with Path(summary).open("a", encoding="utf-8") as stream:
            print(f"- **{phase}**: {elapsed:.2f}s; {counts['fresh']} fresh, {counts['rebuilt']} rebuilt", file=stream)


if __name__ == "__main__":
    main()
