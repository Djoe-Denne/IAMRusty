"""Build one declared service without changing the native E2E build contract."""

import argparse
import os
import subprocess
import sys

from pipeline import INVENTORY, ROOT


def build_arguments(service):
    packages = INVENTORY["builds"][service]
    return ["build", "--locked", "--all-targets",
            *[arg for package in packages for arg in ("--package", package)]]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--service", choices=INVENTORY["builds"], required=True)
    args = parser.parse_args()
    command = build_arguments(args.service)
    result = subprocess.run(["cargo", *command], cwd=ROOT, check=False)
    if result.returncode == 0 and os.environ.get("GITHUB_ENV"):
        package_args = " ".join(command[3:])
        with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as stream:
            stream.write("K6_BUILD_PACKAGE_ARGS=" + package_args + "\n")
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
