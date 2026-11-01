"""Pin already-built local images by actual ID; no cargo, build or pull."""
import argparse
import json
import re
import subprocess
from pathlib import Path
from local_full_target import LocalClusterTargetAnchor
from local_full_image_provenance import verify as verify_provenance

SOURCES = ["aiforall-iam-service:latest", "aiforall-hive-service:latest", "aiforall-telegraph-service:latest",
           "aiforall-manifesto-service:latest", "aiforall-lazaret-service:local-full", "aiforall-sentinel-sync:local-full",
            "aiforall-ext-authz:local", "aiforall-apparatus-controller:j2", "aiforall-zot:gold"]


def source_images(oauth_proof=False, github_openbao=False):
    """Only an explicit proof profile adds the real connector, never a mock."""
    if oauth_proof and github_openbao:
        raise ValueError("distinct GitLab/GitHub profiles required")
    return SOURCES + (["aiforall-gitlab-connect-service:latest"] if oauth_proof else []) + (
        ["aiforall-github-connect-service:latest"] if github_openbao else [])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lease", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--oauth-proof", action="store_true")
    parser.add_argument("--github-openbao", action="store_true")
    parser.add_argument("--build-manifest", type=Path, help="explicit output of build/images.py; never rebuild implicitly")
    args = parser.parse_args()
    target = LocalClusterTargetAnchor(args.lease, expected_cluster="aiforall-local-full")
    target.check()
    if args.output.exists():
        raise ValueError("image manifest exists; refuse overwrite")
    sources = source_images(args.oauth_proof, args.github_openbao)
    built = {}
    if args.build_manifest:
        manifest = json.loads(args.build_manifest.read_text(encoding="utf-8"))
        if manifest.get("schema") != "aiforall.image-build/v1":
            raise ValueError("unexpected build manifest schema")
        built = {record["source"]: record for record in manifest["targets"].values()}
        if len(built) != len(manifest["targets"]):
            raise ValueError("duplicate build manifest sources")
        if any(source not in built for source in sources if source != "aiforall-zot:gold"):
            raise ValueError("build manifest does not cover the selected deployment profile")
    inputs = {source: built.get(source, {}).get("local_ref", source) for source in sources}
    verify_provenance(inputs)
    result = {}
    for source in sources:
        target.check()
        inspected = subprocess.run(["docker", "image", "inspect", "--format", "{{.Id}}", inputs[source]], capture_output=True, text=True, timeout=30)
        image_id = inspected.stdout.strip()
        if inspected.returncode or not re.fullmatch(r"sha256:[a-f0-9]{64}", image_id):
            raise ValueError("required existing application image missing: " + source)
        if source in built and built[source]["image_id"] != image_id:
            raise ValueError("build manifest image ID disagrees with the local image")
        content_image = source.split(":")[0] + ":sha256-" + image_id[7:]
        tagged = subprocess.run(["docker", "tag", inputs[source], content_image], capture_output=True, timeout=30)
        if tagged.returncode:
            raise RuntimeError("content image tagging failed")
        result[source] = content_image
    with args.output.open("x", encoding="utf-8") as target:
        json.dump(result, target, indent=2)
    print("pinned " + str(len(result)) + " prebuilt image IDs; no build/pull/runtime startup performed")


if __name__ == "__main__":
    main()
