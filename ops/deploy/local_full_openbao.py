"""Private-template validation and compatibility seed facade; resources live in Helm."""
import base64
import re
import tomllib

NAMESPACE = "aiforall-local-full"

def iam_template(objects):
    """Find the single actually mounted IAM source, never an unused base CM.

    Returns (pod, volume, source object, text). Only operates on in-memory
    objects; it does not read a live Secret or attest deployment provenance.
    """
    try:
        # Singleton unpacking enforces exactly one match at every boundary;
        # its ValueError is handled by the same redacted failure below.
        deployment, = (o for o in objects if o["kind"] == "Deployment" and o["metadata"]["name"] == "iam")
        pod = deployment["spec"]["template"]["spec"]
        app, = (c for c in pod["containers"] if c["name"] == "iam")
        mount, = (m for m in app["volumeMounts"] if m["mountPath"] in {"/app/config", "/app/config/development.toml"})
        volume, = (v for v in pod["volumes"] if v["name"] == mount["name"])
        if "emptyDir" in volume:
            template, = (m for c in pod.get("initContainers", []) if c["name"] == "openbao-snapshot"
                         for m in c.get("volumeMounts", []) if m["mountPath"] == "/template" and m.get("readOnly"))
            volume, = (v for v in pod["volumes"] if v["name"] == template["name"])
        if set(volume) - {"name"} not in ({"secret"}, {"configMap"}):
            raise ValueError
        private = "secret" in volume
        reference = volume["secret" if private else "configMap"]
        name = reference["secretName" if private else "name"]
        source, = (o for o in objects if o["kind"] == ("Secret" if private else "ConfigMap")
                   and o["metadata"]["name"] == name and o["metadata"].get("namespace") == NAMESPACE)
        if ("items" in reference or (private and mount.get("readOnly") is not True)
                or mount.get("subPath", "development.toml") != "development.toml"):
            raise ValueError
        if "development.toml" in source.get("stringData", {}):
            raise ValueError
        text = source["data"]["development.toml"]
        if not isinstance(text, str):
            raise ValueError
        if private:
            text = base64.b64decode("".join(text.split()), validate=True).decode("utf-8")
        if len(text.encode("utf-8")) > 1048576:
            raise ValueError
        return pod, volume, source, text
    except (KeyError, TypeError, ValueError, UnicodeError):
        raise ValueError("SOURCE_IAM_UNAVAILABLE: unique selected IAM source required (values redacted)") from None


def validate_gitlab_template(template):
    """Validate the selected confidential GitLab template, never compose profiles."""
    fields = {"id", "base_url", "hmac_secret", "pkce_supported", "redirect_uris"}
    try:
        before = tomllib.loads(template)
        connectors = before["idp"]["connectors"]
        if not isinstance(connectors, list) or re.search(r"__[A-Z][A-Z0-9_]*__", template):
            raise ValueError
        connector, = connectors
        if (set(connector) != fields or not isinstance(connector["id"], str)
                or connector["id"].lower() != "gitlab"
                or connector["base_url"] != "https://gitlab-connect:8443/gitlab-connect"
                or not isinstance(connector["hmac_secret"], str) or len(connector["hmac_secret"].encode()) < 16
                or type(connector["pkce_supported"]) is not bool or not connector["pkce_supported"]
                or connector["redirect_uris"] != ["http://127.0.0.1:18080/iam/api/auth/gitlab/callback",
                                                 "http://127.0.0.1:18080/iam/api/auth/gitlab/relink-callback"]):
            raise ValueError
        return before
    except (KeyError, TypeError, ValueError):
        # TOML errors may include private source excerpts: never propagate them.
        raise ValueError("invalid or ambiguous IAM connector preimage (values redacted)") from None


def validate_iam_trust(objects):
    """Validate a selected private trust source without composing any Pod/spec."""
    pod, _, _, text = iam_template(objects)
    try:
        tomllib.loads(text)
    except (ValueError, TypeError):
        raise ValueError("invalid private IAM template (values redacted)") from None
    app = next(c for c in pod["containers"] if c["name"] == "iam")
    sources = [item for item in app.get("env", []) if item["name"] == "SSL_CERT_FILE"]
    if not sources:
        return
    if (len(sources) != 1 or set(sources[0]) != {"name", "value"}
            or not re.fullmatch(r"/[A-Za-z0-9_./-]+", sources[0]["value"])
            or ".." in sources[0]["value"].split("/")):
        raise ValueError("ambiguous IAM trust source")
    mounts = [m for m in app.get("volumeMounts", []) if sources[0]["value"].startswith(m["mountPath"].rstrip("/") + "/")]
    if len(mounts) != 1 or not mounts[0].get("readOnly"):
        raise ValueError("IAM private trust source is not a unique read-only mount")
