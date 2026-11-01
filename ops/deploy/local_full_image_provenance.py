"""Reject stale images for the three changed code boundaries; no Docker startup."""
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BOUNDARIES = {
    "aiforall-github-connect-service:latest": ["services/GitHubConnect"],
    "aiforall-lazaret-service:local-full": ["services/Lazaret"],
    "aiforall-apparatus-controller:j2": ["workers/apparatus-operator"],
}


def verify(images):
    sdk = subprocess.run(["git", "-C", str(ROOT / "rustycog"), "rev-parse", "HEAD"], capture_output=True, text=True, timeout=15)
    if sdk.returncode:
        raise ValueError("cannot identify consumed SDK revision")
    for source, paths in BOUNDARIES.items():
        if source not in images:
            continue
        result = subprocess.run(["docker", "image", "inspect", "--format", "{{json .Config.Labels}}", images[source]],
                                capture_output=True, text=True, timeout=30)
        if result.returncode:
            raise ValueError("cannot verify local image provenance: " + source)
        labels = json.loads(result.stdout) or {}
        revision = labels.get("org.opencontainers.image.revision", "")
        if not re.fullmatch(r"[a-f0-9]{40}", revision) or labels.get("aiforall.dev.sdk-revision") != sdk.stdout.strip():
            raise ValueError("image lacks matching source/SDK provenance: " + source)
        # A label is build metadata, not a signature. The clean delivery build must
        # attest it; this also rejects locally stale code after a newer source edit.
        commit = subprocess.run(["git", "cat-file", "-t", revision], cwd=ROOT, capture_output=True, text=True, timeout=15)
        if commit.returncode or commit.stdout.strip() != "commit":
            raise ValueError("image source revision is unavailable locally: " + source)
        changed = subprocess.run(["git", "diff", "--quiet", revision, "--", *paths, "Cargo.toml", "Cargo.lock", "rustycog", "crates"],
                                 cwd=ROOT, capture_output=True, timeout=30)
        if changed.returncode:
            raise ValueError("image source differs from the retained code: " + source)
