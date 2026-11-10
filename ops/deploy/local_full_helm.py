"""Pinned, client-side Helm invocation. Only typed PUBLIC inputs cross this API."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

import yaml

ROOT = Path(__file__).resolve().parents[2]
CHART = ROOT / "ops/deploy/charts/local-full"
PROFILES = ROOT / "ops/deploy/profiles/local-full"
HELM_VERSION = "v4.3.1+gc0d08d0"
_SCHEMA = json.loads((CHART / "values.schema.json").read_text(encoding="utf-8"))
PROFILES_ALLOWED = set(_SCHEMA["properties"]["profile"]["enum"])
PHASES = tuple(_SCHEMA["properties"]["phase"]["enum"])


def parser(command, *, choices=None):
    """Native argparse kwargs; only Path needs a JSON-to-Python type binding."""
    result = argparse.ArgumentParser()
    for names, options in json.loads((CHART / "files/cli.json").read_text(encoding="utf-8"))[command]:
        if options.get("type") == "path":
            options["type"] = Path
            if "default" in options:
                options["default"] = Path(options["default"]).expanduser()
        if names[-1] in (choices or {}):
            options["choices"] = choices[names[-1]]
        result.add_argument(*names, **options)
    return result


def executable():
    candidate = os.environ.get("AIFORALL_HELM") or shutil.which("helm")
    if not candidate:
        raise RuntimeError("Helm 4.3.1 required; set AIFORALL_HELM to the verified executable")
    result = subprocess.run([candidate, "version", "--short"], capture_output=True,
                            text=True, encoding="utf-8", timeout=15, check=False)
    if result.returncode or result.stdout.strip() != HELM_VERSION:
        raise RuntimeError("verified Helm version must be " + HELM_VERSION)
    return candidate


def render(profile="base", phase="final", *, values=None, chart=CHART):
    if profile not in PROFILES_ALLOWED or phase not in PHASES:
        raise ValueError("unknown explicit local-full profile or phase")
    # No .env, --set, ambient kubeconfig, Helm install, lookup or post-renderer.
    # The only temporary file is public input, never an envelope's secret bytes.
    values = {} if values is None else values
    allowed = {"runId", "images", "secretRefs", "certIdentity", "githubClientId", "simulatedClientId", "publicCa",
               "network", "isolatedPlugins", "pluginCaSha256", "storage"}
    if set(values) - allowed:
        raise ValueError("unsupported or private Helm input")
    helm = executable()
    with tempfile.TemporaryDirectory(prefix="aiforall-helm-public-") as directory:
        path = Path(directory) / "public.json"
        path.write_text(json.dumps(values), encoding="utf-8")
        result = subprocess.run([helm, "template", "local-full", str(chart),
            "--namespace", "aiforall-local-full", "-f", str(PROFILES / (profile + ".yaml")),
            "-f", str(PROFILES / "phases" / (phase + ".yaml")), "-f", str(path)],
            capture_output=True, text=True, encoding="utf-8", check=False, timeout=60)
    if result.returncode:
        # Helm diagnostics may quote values. Do not print them, even on schema failure.
        raise RuntimeError("offline Helm render failed (exit " + str(result.returncode) + ")")
    return [obj for obj in yaml.safe_load_all(result.stdout) if obj]


def category(obj):
    value = obj.get("metadata", {}).get("annotations", {}).get("aiforall.dev/category")
    if value not in {"foundation", "dependencies", "bootstrap", "applications"}:
        raise ValueError("resource lacks declared phase category")
    return value


def storage(mode, namespace, name, claim, *, read_only=False, database="offline_restore"):
    return render(values={"storage": {"mode": mode, "namespace": namespace, "name": name,
        "claim": claim, "readOnly": read_only, "database": database}})
