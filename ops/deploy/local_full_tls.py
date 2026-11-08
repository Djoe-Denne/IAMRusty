"""Explicit local TLS preservation, not certificate validation or rotation."""

PLATFORM_TLS = frozenset({
    ("aiforall-local-full", "platform-mesh-certs"),
    ("aiforall-local-full", "lazaret-identity-ca"),
    ("aiforall-local-plugins", "lazaret-workload-ca"),
})


def preserved_platform_tls(data, *, enabled, oauth_connect, read_deployment):
    """Verify existing leased consumers without reading any Kubernetes Secret.

    A missing or expired Secret is not repaired here: Kubernetes/application TLS
    checks remain active. Input acknowledgements are not Secret UID/byte proofs.
    """
    if not enabled:
        return frozenset()
    if (not oauth_connect or data.get("anchor_state") != "anchored"
            or data.get("cluster") != "aiforall-local-full"
            or data.get("context") != "kind-aiforall-local-full"):
        raise ValueError("TLS preservation requires an existing anchored OAuthConnect local-full deployment")
    for namespace, name in PLATFORM_TLS:
        if (not data.get("namespaceUids", {}).get(namespace)
                or data.get("localSecretInputs", {}).get(namespace + "/" + name) is not True):
            raise ValueError("TLS preservation requires previously acknowledged local platform inputs")
    for name, secret in (("envoy-mesh", "platform-mesh-certs"), ("lazaret", "lazaret-identity-ca")):
        obj = read_deployment(name)
        metadata = obj.get("metadata", {})
        if (metadata.get("name") != name or metadata.get("namespace") != "aiforall-local-full"
                or not metadata.get("uid") or not any(
                    entry.get("kind") == "Deployment" and entry.get("name") == name
                    and entry.get("namespace") == "aiforall-local-full"
                    and entry.get("uid") == metadata["uid"]
                    and entry.get("context") == data["context"] and entry.get("cluster") == data["cluster"]
                    for entry in data.get("ownedResources", []))):
            raise ValueError("TLS preservation consumer UID is not leased")
        pod = obj.get("spec", {}).get("template", {}).get("spec", {})
        volumes = {v["name"] for v in pod.get("volumes", []) if v.get("secret", {}).get("secretName") == secret}
        mounted = {m["name"] for c in pod.get("containers", []) + pod.get("initContainers", [])
                   for m in c.get("volumeMounts", [])}
        if not volumes.intersection(mounted):
            raise ValueError("TLS preservation requires the original mounted platform Secret reference")
    return PLATFORM_TLS


def omit_preserved_tls(objects, preserved):
    """Omit only the closed TLS set from apply; leave all other objects intact."""
    if not set(preserved).issubset(PLATFORM_TLS):
        raise ValueError("unknown preserved TLS identity")
    return [o for o in objects if not (o["kind"] == "Secret" and (
        o["metadata"].get("namespace"), o["metadata"]["name"]) in preserved)]


def reuse_openbao_tls(objects, tls_objects, data, read_deployment):
    """Keep the existing leased OpenBao TLS reference instead of rotating its leaf."""
    current = read_deployment("openbao")
    metadata = current.get("metadata", {})
    if not any(entry.get("kind") == "Deployment" and entry.get("name") == "openbao"
               and entry.get("namespace") == "aiforall-local-full" and entry.get("uid")
               and entry["uid"] == metadata.get("uid") and metadata.get("name") == "openbao"
               and metadata.get("namespace") == "aiforall-local-full"
               and entry.get("context") == data["context"] and entry.get("cluster") == data["cluster"]
               for entry in data.get("ownedResources", [])):
        raise ValueError("OpenBao TLS preservation consumer UID is not leased")
    pod = current.get("spec", {}).get("template", {}).get("spec", {})
    mounted = {m["name"] for c in pod.get("containers", []) + pod.get("initContainers", [])
               for m in c.get("volumeMounts", [])}
    names = {v.get("secret", {}).get("secretName") for v in pod.get("volumes", [])
             if v["name"] in mounted and v.get("secret", {}).get("secretName", "").startswith("openbao-server-tls-")}
    if len(names) != 1:
        raise ValueError("OpenBao TLS preservation requires one original mounted TLS reference")
    name = names.pop()
    if not any(entry.get("kind") == "Secret" and entry.get("name") == name
               and entry.get("namespace") == "aiforall-local-full" and entry.get("uid")
               and entry.get("context") == data["context"] and entry.get("cluster") == data["cluster"]
               for entry in data.get("ownedResources", [])):
        raise ValueError("OpenBao TLS source reference is not recorded in the original lease")
    targets = [v["secret"] for o in objects if o["kind"] == "Deployment" and o["metadata"]["name"] == "openbao"
               for v in o["spec"]["template"]["spec"].get("volumes", [])
               if v.get("secret", {}).get("secretName", "").startswith("openbao-server-tls-")]
    proposed = [o for o in tls_objects if o["kind"] == "Secret"
                and o["metadata"]["name"].startswith("openbao-server-tls-")]
    if len(targets) != 1 or len(proposed) != 1 or targets[0]["secretName"] != proposed[0]["metadata"]["name"]:
        raise ValueError("ambiguous proposed OpenBao TLS preservation")
    targets[0]["secretName"] = name
    return [o for o in tls_objects if o is not proposed[0]]
