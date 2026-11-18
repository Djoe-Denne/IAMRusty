"""Build declared production targets and record their actual native E2E provenance."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time

from pipeline import INVENTORY, ROOT


def build_arguments(service):
    packages = INVENTORY["builds"][service]
    return ["build", "--locked",
            *[arg for package in packages for arg in ("--package", package)]]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--service", choices=INVENTORY["builds"], required=True)
    args = parser.parse_args()
    command = build_arguments(args.service)
    timing_path = os.environ.get("CI_BUILD_TIMING_FILE", "")
    if timing_path:
        target = Path(os.environ["CARGO_TARGET_DIR"])
        if target.exists() and any(target.iterdir()):
            print("Refusing cache qualification with pre-existing target artifacts.", file=sys.stderr)
            return 2
    started = time.perf_counter()
    result = subprocess.run(["cargo", *command], cwd=ROOT, check=False)
    elapsed = time.perf_counter() - started
    if timing_path:
        Path(timing_path).write_text(json.dumps({
            "schema": "aiforall.ci-build-timing/v1", "service": args.service,
            "source_sha": os.environ.get("GITHUB_SHA", ""),
            "run_id": os.environ.get("GITHUB_RUN_ID", ""),
            "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT", ""),
            "job": os.environ.get("GITHUB_JOB", ""), "command": command,
            "target_initially_empty": True, "elapsed_seconds": round(elapsed, 3),
            "exit_code": result.returncode,
        }, sort_keys=True) + "\n", encoding="utf-8")
    if result.returncode == 0 and os.environ.get("GITHUB_ENV"):
        with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as stream:
            stream.write("K6_BUILD_ARGS_JSON=" + json.dumps(command) + "\n")
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
