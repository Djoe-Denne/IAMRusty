"""Pin already-built local images by actual ID; no cargo, build or pull."""
import argparse
import json
import re
import subprocess
from pathlib import Path
from local_full_target import LocalClusterTargetAnchor

SOURCES = ["aiforall-iam-service:latest", "aiforall-hive-service:latest", "aiforall-telegraph-service:latest",
           "aiforall-manifesto-service:latest", "aiforall-lazaret-service:local-full", "aiforall-sentinel-sync:local-full",
           "aiforall-ext-authz:local", "aiforall-apparatus-controller:j2", "aiforall-zot:gold"]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lease", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    target = LocalClusterTargetAnchor(args.lease, expected_cluster="aiforall-local-full")
    target.check()
    if args.output.exists():
        raise ValueError("image manifest exists; refuse overwrite")
    result = {}
    for source in SOURCES:
        target.check()
        inspected = subprocess.run(["docker", "image", "inspect", "--format", "{{.Id}}", source], capture_output=True, text=True, timeout=30)
        image_id = inspected.stdout.strip()
        if inspected.returncode or not re.fullmatch(r"sha256:[a-f0-9]{64}", image_id):
            raise ValueError("required existing application image missing: " + source)
        content_image = source.split(":")[0] + ":sha256-" + image_id[7:]
        tagged = subprocess.run(["docker", "tag", source, content_image], capture_output=True, timeout=30)
        if tagged.returncode:
            raise RuntimeError("content image tagging failed")
        result[source] = content_image
    with args.output.open("x", encoding="utf-8") as target:
        json.dump(result, target, indent=2)
    print("pinned 9 prebuilt image IDs; no build/pull/runtime startup performed")


if __name__ == "__main__":
    main()
