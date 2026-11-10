"""Consume explicit local build IDs or published OCI digests; never compile."""
import argparse
import json
import re
import subprocess
from pathlib import Path
from local_full_target import LocalClusterTargetAnchor
from local_full_image_provenance import verify as verify_provenance

def source_images(oauth_proof=False, github_openbao=False, oauth_connect=False):
    """Only an explicit proof profile adds the real connector, never a mock."""
    if sum(map(bool, (oauth_proof, github_openbao, oauth_connect))) > 1:
        raise ValueError("distinct GitLab/GitHub profiles required")
    from local_full_helm import render
    profile = "oauth-connect" if oauth_connect else "github-openbao" if github_openbao else "gitlab-proof" if oauth_proof else "base"
    return sorted({c["image"] for obj in render(profile) if obj["kind"] == "Deployment"
                   for c in obj["spec"]["template"]["spec"].get("containers", []) + obj["spec"]["template"]["spec"].get("initContainers", [])
                   if c["image"].startswith("aiforall-")})


SOURCES = source_images()  # Compatibility view, never a second maintained roster.


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lease", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--oauth-proof", action="store_true")
    parser.add_argument("--github-openbao", action="store_true")
    parser.add_argument("--oauth-connect", action="store_true")
    parser.add_argument("--build-manifest", type=Path, help="explicit output of build/images.py; never rebuild implicitly")
    parser.add_argument("--release-manifest", type=Path, help="published OCI receipt; never falls back to kind load")
    parser.add_argument("--registry-state", type=Path)
    args = parser.parse_args()
    target = LocalClusterTargetAnchor(args.lease, expected_cluster="aiforall-local-full")
    target.check()
    if args.output.exists():
        raise ValueError("image manifest exists; refuse overwrite")
    sources = source_images(args.oauth_proof, args.github_openbao, args.oauth_connect)
    if args.release_manifest:
        if args.build_manifest or not args.registry_state:
            raise ValueError("OCI receipt requires --registry-state and no --build-manifest")
        from local_registry import Registry
        registry = Registry(args.registry_state)
        release = json.loads(args.release_manifest.read_text(encoding="utf-8"))
        if release.get("schema") != "aiforall.image-release/v1" or release.get("source", {}).get("mode") != "working-tree":
            raise ValueError("local OCI publication receipt required")
        images = {item["source"]: item["oci_ref"] for item in release["images"].values()}
        images["aiforall-zot:gold"] = registry.lock["p4_zot"]["reference"]
        if set(images) != set(sources):
            raise ValueError("OCI receipt does not cover exactly the selected deployment profile")
        with registry.operation():
            registry.verify_mapping(images)
            target.registry_connect(registry)
            for image in images.values():
                target.registry_pull(registry, image)
            registry.verify_mapping(images)
            with args.output.open("x", encoding="utf-8") as stream:
                json.dump(images, stream, indent=2)
        print("verified/pulled OCI image digests; no kind load, build or deployment")
        return
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
    from private_diagnostics import operation
    with operation("consume-images"):
        main()
