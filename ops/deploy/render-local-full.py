"""Offline Kustomize render/split; never prints generated configuration or keys."""
import argparse
import copy
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path
from urllib.parse import urlsplit

import yaml

ROOT = Path(__file__).resolve().parents[2]
OVERLAY = ROOT / "ops/deploy/apps/overlays/local-full"
DEPENDENCIES = {"postgres", "openfga", "redis", "localstack", "openbao", "zot", "component-catalog", "mailhog"}
APPLICATIONS = {"iam", "hive", "telegraph", "manifesto", "lazaret", "sentinel-sync", "ext-authz", "envoy-mesh", "apparatus-operator"}


def render(run_id, images=None, cert_identity=""):
    if not re.fullmatch(r"[a-z0-9]{12,32}", run_id):
        raise ValueError("run-id must be 12-32 lowercase alphanumeric characters")
    # This command is purely offline: a fixed NONEXISTENT config prevents any
    # ambient credential fallback if kubectl ever tries to resolve an API.
    result = subprocess.run(["kubectl", "--kubeconfig", str(ROOT / "ops/deploy/offline-no-kubeconfig"), "--context", "kind-aiforall-local-full", "kustomize", "--load-restrictor", "LoadRestrictionsNone", str(OVERLAY)],
                            capture_output=True, text=True, check=False, timeout=60, encoding="utf-8")
    if result.returncode:
        # Do not echo rendered manifests or stderr that could contain file data.
        raise RuntimeError("offline Kustomize render failed (exit " + str(result.returncode) + ")")
    # Issuer 'aiforall-platform' is an identity, not a namespace. Only the
    # source Envoy document's DNS addresses need textual namespace replacement.
    original = list(yaml.safe_load_all(result.stdout))
    objects = [o for o in original if o]
    # Only the explicit registry's direct IAM relink redirects authorize an
    # access-JWT exemption. Current local-full registry is empty: retain none.
    iam_config = tomllib.loads((OVERLAY / "config/iam.toml").read_text())
    relink_paths = set()
    for connector in iam_config.get("idp", {}).get("connectors", []):
        slug = connector.get("id")
        if slug not in {"github", "gitlab"}:
            continue
        path = "/iam/api/auth/" + slug + "/relink-callback"
        if any(urlsplit(uri).path == path for uri in connector.get("redirect_uris", [])):
            relink_paths.add(path)
    for obj in objects:
        if obj["kind"] == "ConfigMap" and obj["metadata"]["name"] == "envoy-mesh-config":
            obj["data"]["envoy.yaml"] = obj["data"]["envoy.yaml"].replace("aiforall-platform.svc", "aiforall-local-full.svc").replace("aiforall-gateway.svc", "aiforall-local-full.svc")
            envoy = yaml.safe_load(obj["data"]["envoy.yaml"])
            for listener in envoy["static_resources"]["listeners"]:
                for chain in listener["filter_chains"]:
                    for network_filter in chain["filters"]:
                        config = network_filter.get("typed_config", {})
                        for host in config.get("route_config", {}).get("virtual_hosts", []):
                            host["routes"] = [route for route in host["routes"]
                                              if not route.get("match", {}).get("path", "").endswith("/relink-callback")
                                              or route["match"]["path"] in relink_paths]
            obj["data"]["envoy.yaml"] = yaml.safe_dump(envoy, sort_keys=False)
        if obj["kind"] == "Job":
            obj["metadata"]["name"] += "-" + run_id
    configuration = [{"kind": o["kind"], "name": o["metadata"]["name"], "data": o.get("data"), "stringData": o.get("stringData")}
                     for o in objects if o["kind"] in {"ConfigMap", "Secret"}]
    identity = hashlib.sha256((json.dumps(configuration, sort_keys=True) + cert_identity).encode()).hexdigest()
    for obj in objects:
        if obj["kind"] == "Deployment":
            template = obj["spec"]["template"]
            template.setdefault("metadata", {}).setdefault("annotations", {})["aiforall.dev/config-sha256"] = identity
            for container in template["spec"].get("containers", []) + template["spec"].get("initContainers", []):
                image = container["image"]
                if image.startswith("aiforall-") and images is not None:
                    replacement = images.get(image)
                    if not replacement or not re.fullmatch(r"[a-z0-9./:_-]+(?::sha256-[a-f0-9]{64}|@sha256:[a-f0-9]{64})", replacement):
                        raise ValueError("missing immutable application image mapping for " + image)
                    container["image"] = replacement
            # Hashes in ConfigMap volume names and content image tags now both
            # affect the PodTemplate; no forced restart against an old Ready pod.
    serialized = json.dumps(objects)
    if "host.docker.internal" in serialized or "oodhive-monolith" in serialized:
        raise ValueError("host/hybrid dependency leaked into local-full render")
    for obj in objects:
        if obj["kind"] not in {"Namespace", "CustomResourceDefinition", "ClusterRole", "ClusterRoleBinding"}:
            if obj["metadata"].get("namespace") not in {"aiforall-local-full", "aiforall-local-apparatus", "aiforall-local-plugins"}:
                raise ValueError("resource outside explicit local-full namespaces")
    return objects


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--run-id", default="staticvalidation")
    parser.add_argument("--image-manifest", type=Path)
    parser.add_argument("--cert-identity", default="")
    args = parser.parse_args()
    images = json.loads(args.image_manifest.read_text()) if args.image_manifest else None
    objects = render(args.run_id, images, args.cert_identity)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    if any(args.output_dir.iterdir()):
        raise ValueError("render output directory must be empty (no overwrite)")
    groups = {"foundation": [], "dependencies": [], "bootstrap": [], "applications": []}
    for obj in objects:
        name = obj["metadata"]["name"]
        group = "bootstrap" if obj["kind"] == "Job" else "dependencies" if obj["kind"] == "Deployment" and name in DEPENDENCIES else "applications" if obj["kind"] == "Deployment" and name in APPLICATIONS else "foundation"
        groups[group].append(copy.deepcopy(obj))
    # First replicas boot serially against their own schema; scale out after
    # migrations/readiness settle, avoiding two simultaneous boot migrations.
    for obj in groups["applications"]:
        obj["spec"]["replicas"] = 1
    for name, items in groups.items():
        with (args.output_dir / (name + ".json")).open("x", encoding="utf-8") as target:
            json.dump({"apiVersion": "v1", "kind": "List", "items": items}, target)
    # Only metadata enters stdout. Rendered local credentials stay in files.
    print(json.dumps({"resource_count": len(objects), "groups": {k: len(v) for k, v in groups.items()},
                      "immutable_app_images_supplied": images is not None,
                      "objects": [o["kind"] + "/" + o["metadata"]["name"] for o in objects]}))


if __name__ == "__main__":
    main()
