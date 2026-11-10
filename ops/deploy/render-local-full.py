"""Compatibility facade for the standard offline Helm local-full renderer.

Private proof hydration is explicit and memory-only; it never reaches Helm.
"""
import json
from pathlib import Path
import re
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import local_full_helm as helm
import local_full_openbao

ROOT = helm.ROOT


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


def render(run_id, images=None, cert_identity="", oauth_proof=False, proof_inputs=None,
           github_openbao=False, github_client_id="offline-synthetic-client", oauth_connect=False,
           simulated_client_id="offline-simulated-client", *, phase="final", public_ca="", network=None,
           isolated_plugins=False, plugin_ca_sha256="", secret_refs=None):
    if not isinstance(run_id, str) or not re.fullmatch(r"[a-z0-9]{12,32}", run_id):
        raise ValueError("run-id must be 12-32 lowercase alphanumeric characters")
    if sum(map(bool, (github_openbao, oauth_proof, oauth_connect))) > 1:
        raise ValueError("GitHub legacy, GitLab legacy and OAuthConnect are distinct profiles")
    if (not isinstance(github_client_id, str) or not re.fullmatch(r"[A-Za-z0-9_.-]{1,128}", github_client_id)
            or not isinstance(simulated_client_id, str) or not re.fullmatch(r"[A-Za-z0-9_-]{8,128}", simulated_client_id)):
        raise ValueError("invalid connector public client ID")
    if proof_inputs is not None:
        validate_proof_inputs(proof_inputs)
        if not oauth_proof or proof_inputs["run_id"] != run_id:
            raise ValueError("private OAuth proof inputs require matching explicit profile/run")
    profile = "oauth-connect" if oauth_connect else "github-openbao" if github_openbao else "gitlab-proof" if oauth_proof else "base"
    if images is not None:
        if not isinstance(images, dict) or not images:
            raise ValueError("immutable image mapping must be a nonempty object")
    objects = helm.render(profile, phase, values={"runId": run_id, "images": images or {}, "secretRefs": secret_refs or {},
        "certIdentity": cert_identity, "githubClientId": github_client_id, "simulatedClientId": simulated_client_id,
        "publicCa": public_ca, "network": network or {"apiCidr": "", "ingressCidr": ""},
        "isolatedPlugins": isolated_plugins, "pluginCaSha256": plugin_ca_sha256})
    if proof_inputs is not None:
        from local_full_secrets import hydrate
        hydrate(objects, proof_inputs=proof_inputs)
    return objects


def main():
    args = helm.parser("render", choices={"--phase": helm.PHASES}).parse_args()
    images = json.loads(args.image_manifest.read_text()) if args.image_manifest else None
    objects = render(args.run_id, images, args.cert_identity, args.oauth_proof,
                     github_openbao=args.github_openbao, oauth_connect=args.oauth_connect, phase=args.phase)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    if any(args.output_dir.iterdir()):
        raise ValueError("render output directory must be empty (no overwrite)")
    groups = {name: [] for name in ("foundation", "dependencies", "bootstrap", "applications")}
    for obj in objects:
        groups[helm.category(obj)].append(obj)
    for name, items in groups.items():
        with (args.output_dir / (name + ".json")).open("x", encoding="utf-8") as target:
            json.dump({"apiVersion": "v1", "kind": "List", "items": items}, target)
    print(json.dumps({"resource_count": len(objects), "groups": {k: len(v) for k, v in groups.items()},
                      "immutable_app_images_supplied": images is not None,
                      "objects": [o["kind"] + "/" + o["metadata"]["name"] for o in objects]}))


if __name__ == "__main__":
    main()
