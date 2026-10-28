"""Offline Kustomize render/split; never prints generated configuration or keys."""
import argparse
import base64
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
APPLICATIONS = {"iam", "hive", "telegraph", "manifesto", "lazaret", "sentinel-sync", "ext-authz", "envoy-mesh", "apparatus-operator", "gitlab-connect", "oauth-idp-simulator"}


def validate_proof_inputs(value):
    expected = {"version", "run_id", "client_id", "client_secret", "hmac_secret", "oauth_state_secret"}
    if not isinstance(value, dict) or set(value) != expected or type(value["version"]) is not int or value["version"] != 1:
        raise ValueError("invalid private OAuth proof input schema")
    patterns = {"run_id": r"[a-z0-9]{12,32}", "client_id": r"[A-Za-z0-9_-]{8,128}",
                "client_secret": r"[A-Za-z0-9_-]{32,128}", "hmac_secret": r"[a-f0-9]{64}",
                "oauth_state_secret": r"[a-f0-9]{64}"}
    if any(not isinstance(value[key], str) or not re.fullmatch(pattern, value[key]) for key, pattern in patterns.items()):
        raise ValueError("invalid private OAuth proof input values")
    if len({value[key] for key in ("client_secret", "hmac_secret", "oauth_state_secret")}) != 3:
        raise ValueError("dedicated distinct OAuth proof secrets required")
    return value


def proof_profile(objects, run_id, inputs):
    if inputs is not None:
        validate_proof_inputs(inputs)
        if inputs["run_id"] != run_id:
            raise ValueError("OAuth proof run identity differs")
    replacements = {"__OAUTH_PROOF_CLIENT_ID__": "client_id", "__OAUTH_PROOF_CLIENT_SECRET__": "client_secret",
                    "__OAUTH_PROOF_HMAC__": "hmac_secret", "__OAUTH_PROOF_STATE__": "oauth_state_secret"}
    names = {obj["metadata"]["name"]: obj["metadata"]["name"] + "-" + run_id for obj in objects
             if obj["kind"] == "Secret" and obj["metadata"]["name"].startswith("oauth-proof-")}
    names.update({name: name + "-" + run_id for name in
                  ("oauth-proof-inputs", "oauth-proof-public-ca", "oauth-proof-connector-tls")})
    authority = "http://127.0.0.1:8080/iam"
    for obj in objects:
        if obj["kind"] == "Secret" and obj["metadata"]["name"] in names:
            obj["metadata"]["name"] = names[obj["metadata"]["name"]]
            if inputs:
                for key, encoded in obj.get("data", {}).items():
                    text = base64.b64decode("".join(encoded.split()), validate=True).decode()
                    for marker, field in replacements.items():
                        text = text.replace(marker, inputs[field])
                    obj["data"][key] = base64.b64encode(text.encode()).decode()
        if obj["kind"] == "Deployment":
            pod = obj["spec"]["template"]["spec"]
            for volume in pod.get("volumes", []):
                reference = volume.get("secret", {})
                if reference.get("secretName") in names:
                    reference["secretName"] = names[reference["secretName"]]
            name = obj["metadata"]["name"]
            if name in {"hive", "telegraph", "manifesto", "lazaret", "sentinel-sync"}:
                prefix = "SENTINEL_SYNC_PRINCIPAL" if name == "sentinel-sync" else name.upper() + "_AUTH__JWT"
                env = pod["containers"][0].setdefault("env", [])
                for key, value in (("PLATFORM_ISSUER", authority), ("ALLOW_HISTORICAL_PLATFORM_ISSUER", "false")):
                    variable = prefix + "__" + key
                    existing = [entry for entry in env if entry["name"] == variable]
                    if existing and (len(existing) != 1 or existing[0].get("value") != value):
                        raise ValueError("conflicting explicit principal policy in OAuth proof")
                    if not existing:
                        env.append({"name": variable, "value": value})
    if inputs:
        provider = {key: inputs[key] for key in ("client_id", "client_secret", "run_id")}
        objects.append({"apiVersion": "v1", "kind": "Secret", "metadata": {
            "name": names["oauth-proof-inputs"], "namespace": "aiforall-local-full"}, "type": "Opaque",
            "data": {"provider.json": base64.b64encode(json.dumps(provider).encode()).decode()}})


def http_downstream(envoy):
    """Only local-full's known edge changes transport, never an upstream hop."""
    listeners = envoy["static_resources"]["listeners"]
    targets = [listener for listener in listeners if listener.get("name") == "listener_mesh"]
    if (len(targets) != 1
            or targets[0].get("address", {}).get("socket_address", {}).get("port_value") != 10000):
        raise ValueError("unique local-full listener_mesh:10000 required")
    chains = targets[0].get("filter_chains", [])
    if len(chains) != 1 or not chains[0].get("filters") or chains[0].get("filter_chain_match"):
        raise ValueError("unique local-full HTTP filter chain required")
    # No recursive/global removal: client certificate, CA and SAN verification
    # on all five upstreams deliberately survive unchanged.
    chains[0].pop("transport_socket", None)
    chains[0].pop("filter_chain_match", None)


def render(run_id, images=None, cert_identity="", oauth_proof=False, proof_inputs=None):
    if not re.fullmatch(r"[a-z0-9]{12,32}", run_id):
        raise ValueError("run-id must be 12-32 lowercase alphanumeric characters")
    if proof_inputs is not None:
        validate_proof_inputs(proof_inputs)
        if not oauth_proof or proof_inputs["run_id"] != run_id:
            raise ValueError("private OAuth proof inputs require matching explicit profile/run")
    # This command is purely offline: a fixed NONEXISTENT config prevents any
    # ambient credential fallback if kubectl ever tries to resolve an API.
    selected_overlay = OVERLAY / "oauth-proof" if oauth_proof else OVERLAY
    result = subprocess.run(["kubectl", "--kubeconfig", str(ROOT / "ops/deploy/offline-no-kubeconfig"), "--context", "kind-aiforall-local-full", "kustomize", "--load-restrictor", "LoadRestrictionsNone", str(selected_overlay)],
                            capture_output=True, text=True, check=False, timeout=60, encoding="utf-8")
    if result.returncode:
        # Do not echo rendered manifests or stderr that could contain file data.
        raise RuntimeError("offline Kustomize render failed (exit " + str(result.returncode) + ")")
    # Issuer 'aiforall-platform' is an identity, not a namespace. Only the
    # source Envoy document's DNS addresses need textual namespace replacement.
    original = list(yaml.safe_load_all(result.stdout))
    objects = [o for o in original if o]
    if oauth_proof:
        proof_profile(objects, run_id, proof_inputs)
    # Read the effective rendered profile, never the empty base registry after
    # selecting a proof overlay. It is also the configuration included in hash.
    iam_configs = []
    for obj in objects:
        name = obj["metadata"]["name"]
        if "development.toml" not in obj.get("data", {}):
            continue
        if oauth_proof and obj["kind"] == "Secret" and name.startswith("oauth-proof-iam-config-"):
            iam_configs.append(base64.b64decode("".join(obj["data"]["development.toml"].split()), validate=True).decode())
        elif not oauth_proof and obj["kind"] == "ConfigMap" and (name == "iam-dev-config" or name.startswith("iam-dev-config-")):
            iam_configs.append(obj["data"]["development.toml"])
    if len(iam_configs) != 1:
        raise ValueError("unique effective IAM configuration required")
    iam_config = tomllib.loads(iam_configs[0])
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
            http_downstream(envoy)
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
    parser.add_argument("--oauth-proof", action="store_true")
    args = parser.parse_args()
    images = json.loads(args.image_manifest.read_text()) if args.image_manifest else None
    objects = render(args.run_id, images, args.cert_identity, args.oauth_proof)
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
