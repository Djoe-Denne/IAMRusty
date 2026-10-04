"""Idempotent local bootstrap; publishes only public FGA identifiers to one CM."""
import hashlib
import json
import os
import ssl
import time
import urllib.request
import urllib.parse
from pathlib import Path

DEADLINE = time.monotonic() + 210
FGA = "http://openfga:8080"
MODEL = json.loads(Path("/model/model.json").read_text())


def request(base, path, method="GET", body=None, headers=None, context=None):
    if time.monotonic() >= DEADLINE:
        raise RuntimeError("bootstrap deadline exceeded")
    data = None if body is None else json.dumps(body).encode()
    h = {"Content-Type": "application/json", **(headers or {})}
    req = urllib.request.Request(base + path, data=data, headers=h, method=method)
    with urllib.request.urlopen(req, timeout=5, context=context) as response:
        return json.load(response)


def pages(path, key):
    result = []
    token = ""
    while True:
        suffix = "?continuation_token=" + urllib.parse.quote(token) if token else ""
        page = request(FGA, path + suffix)
        result.extend(page.get(key, []))
        token = page.get("continuation_token", "")
        if not token:
            return result


def canonical_model(model):
    return {k: model[k] for k in ("schema_version", "type_definitions", "conditions") if k in model and model[k]}


def main():
    # Never silently reset a store, tuples, authorization model or CM.
    matches = [s for s in pages("/stores", "stores") if s.get("name") == "aiforall-local-full"]
    if len(matches) > 1:
        raise RuntimeError("ambiguous local-full store inventory")
    store = matches[0] if matches else request(FGA, "/stores", "POST", {"name": "aiforall-local-full"})
    sid = store["id"]
    candidates = pages("/stores/" + sid + "/authorization-models", "authorization_models")
    existing = [m for m in candidates if canonical_model(m) == canonical_model(MODEL)]
    model = existing[0] if existing else request(FGA, "/stores/" + sid + "/authorization-models", "POST", MODEL)
    mid = model.get("id", model.get("authorization_model_id"))
    if not sid or not mid:
        raise RuntimeError("missing bootstrap identifiers")
    namespace = os.environ["POD_NAMESPACE"]
    if namespace != "aiforall-local-full":
        raise RuntimeError("bootstrap namespace must be explicit local-full")
    token = Path("/var/run/secrets/kubernetes.io/serviceaccount/token").read_text().strip()
    ca = ssl.create_default_context(cafile="/var/run/secrets/kubernetes.io/serviceaccount/ca.crt")
    url = "https://kubernetes.default.svc"
    path = "/api/v1/namespaces/" + namespace + "/configmaps/local-full-fga"
    # Restricted RBAC only permits get/patch of this pre-created ConfigMap.
    current = request(url, path, headers={"Authorization": "Bearer " + token}, context=ca)
    data = {"store-id": sid, "model-id": mid, "model-sha256": hashlib.sha256(json.dumps(MODEL, sort_keys=True).encode()).hexdigest()}
    request(url, path, "PATCH", {"metadata": {"resourceVersion": current["metadata"]["resourceVersion"]}, "data": data},
            {"Authorization": "Bearer " + token, "Content-Type": "application/merge-patch+json"}, ca)
    print("local-full OpenFGA store/model configured; no tuple deletion")


if __name__ == "__main__":
    main()
