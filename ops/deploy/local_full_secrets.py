"""Confidential byte hydration of DECLARED envelopes; never a Helm values input."""
import base64
import json

import yaml

from local_full_helm import CHART, ROOT


def envelope(name, values):
    obj = yaml.safe_load((CHART / "files/private/opaque-secret.yaml").read_text(encoding="utf-8"))
    obj["metadata"]["name"] = name
    obj["data"] = {key: base64.b64encode(value).decode("ascii") for key, value in values.items()}
    return obj


def hydrate(objects, *, proof_inputs=None, workload_ca=None):
    """Only secret data changes; no resources/specs are synthesized after Helm."""
    markers = json.loads((CHART / "files/private/local-insecure-fixtures.json").read_text(encoding="utf-8"))
    markers.pop("description")
    markers.update({} if proof_inputs is None else {
        "__OAUTH_PROOF_CLIENT_ID__": proof_inputs["client_id"],
        "__OAUTH_PROOF_CLIENT_SECRET__": proof_inputs["client_secret"],
        "__OAUTH_PROOF_HMAC__": proof_inputs["hmac_secret"],
        "__OAUTH_PROOF_STATE__": proof_inputs["oauth_state_secret"]})
    for obj in objects:
        if obj["kind"] != "Secret":
            continue
        annotations = obj["metadata"].get("annotations", {})
        identity = annotations.get("aiforall.dev/private-envelope")
        if not identity:
            continue
        values = {}
        for key, source in json.loads(annotations.get("aiforall.dev/private-files", "{}")).items():
            path = CHART / source
            if not source.startswith("files/private/") or not path.resolve().is_relative_to((CHART / "files/private").resolve()):
                raise ValueError("private envelope template outside declared chart")
            values[key] = path.read_text(encoding="utf-8").rstrip("\n")
        if identity == "fixture-iam-signing-key":
            values["test-platform.pem"] = (ROOT / "services/IAMRusty/config/keys/test-platform.pem").read_text(encoding="utf-8")
        if annotations.get("aiforall.dev/template-configmap"):
            selected = [item for item in objects if item["kind"] == "ConfigMap"
                        and item["metadata"]["name"] == annotations["aiforall.dev/template-configmap"]
                        and item["metadata"].get("namespace") == obj["metadata"].get("namespace")]
            if len(selected) != 1:
                raise ValueError("unique declared private template source required")
            values.update(selected[0]["data"])
        for field, marker in json.loads(annotations.get("aiforall.dev/private-field-markers", "{}")).items():
            if marker not in markers:
                raise ValueError("unknown declared private fixture field")
            values[field] = marker
        if identity == "proof-provider":
            if proof_inputs is None:
                continue
            values["provider.json"] = json.dumps({key: proof_inputs[key] for key in ("client_id", "client_secret", "run_id")})
        if identity == "workload-public-ca" and workload_ca is not None:
            values["ca.crt"] = workload_ca.decode("ascii")
        for key, text in values.items():
            for marker, value in markers.items():
                text = text.replace(marker, value)
            values[key] = text.encode("utf-8")
        obj["data"] = {key: base64.b64encode(value).decode("ascii") for key, value in values.items()}
    if any(o["kind"] == "Deployment" and o["metadata"]["name"] == "iam" for o in objects):
        from local_full_openbao import validate_iam_trust, validate_gitlab_template, iam_template
        validate_iam_trust(objects)
        if proof_inputs is not None:
            validate_gitlab_template(iam_template(objects)[3])
