"""Verify bytes consumed by every Rust image, not only HEAD or three services."""
import json
import re
import subprocess
from image_catalog import FRESHNESS_POLICY, TARGETS, build_context
from image_sources import ROOT, capture, target_identity

BOUNDARIES = {item["source"]: name for name, item in TARGETS.items()}


def verify_labels(source, labels, snapshot):
    name = BOUNDARIES[source]
    build_targets = labels.get("aiforall.dev.build-targets", "").split(",")
    profile = labels.get("aiforall.dev.cargo-profile")
    if (labels.get("aiforall.dev.source-input") != target_identity(snapshot, name)
            or labels.get("aiforall.dev.source-policy") != "local"
            or labels.get("aiforall.dev.freshness-policy") != FRESHNESS_POLICY
            or name not in build_targets or profile not in {"dev-image", "release"}
            or labels.get("aiforall.dev.build-context") != build_context(build_targets, profile)
            or not re.fullmatch(r"[a-f0-9]{40}", labels.get("org.opencontainers.image.revision", ""))):
        raise ValueError("image lacks matching working-tree/SDK input provenance; rebuild: " + source)


def verify(images):
    snapshot = capture(ROOT)
    for source in BOUNDARIES.keys() & images.keys():
        result = subprocess.run(["docker", "image", "inspect", "--format", "{{json .Config.Labels}}", images[source]],
                                capture_output=True, text=True, timeout=30)
        if result.returncode:
            raise ValueError("cannot verify local image provenance: " + source)
        verify_labels(source, json.loads(result.stdout) or {}, snapshot)
