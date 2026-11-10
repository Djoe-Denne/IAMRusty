"""Validated declarative Linux deliverables; no Cargo-cache discovery."""
import hashlib
import json
from pathlib import Path
import re

FRESHNESS_POLICY = "snapshot-final-v1"


def load(path=Path(__file__).parent / "build/targets.json"):
    value = json.loads(Path(path).read_text(encoding="utf-8"))["variable"]["inventory"]["default"]
    if value.get("schema") != "aiforall.image-targets/v1":
        raise ValueError("unknown image inventory schema")
    targets = value["targets"]
    for name, item in targets.items():
        if (not re.fullmatch(r"[a-z][a-z0-9-]*", name)
                or set(item) != {"source", "dockerfile", "outputs", "inputs", "features"}
                or not isinstance(item["source"], str) or not item["source"]
                or not isinstance(item["outputs"], dict) or not item["outputs"]
                or not isinstance(item["inputs"], list) or not item["inputs"]
                or not isinstance(item["features"], list)
                or any(not isinstance(s, str) or not s for s in
                       [item["dockerfile"], *item["outputs"], *item["outputs"].values(), *item["inputs"], *item["features"]])):
            raise ValueError("invalid typed image target")
    if any(not isinstance(names, list) or not names or any(n not in targets for n in names)
           for names in value["groups"].values()):
        raise ValueError("invalid image inventory group")
    return value


INVENTORY = load()
TARGETS = INVENTORY["targets"]  # Compatibility view, never a second maintained catalogue.
LOCAL_FULL = tuple(INVENTORY["groups"]["local-full"])


def select(names):
    names = list(dict.fromkeys(names))
    if not names or any(name not in TARGETS for name in names):
        raise ValueError("select explicit image targets from: " + ",".join(TARGETS))
    return {name: TARGETS[name] for name in names}


def build_context(names, profile):
    selected = select(names)
    value = {"targets": sorted(selected), "profile": profile,
             "features": sorted({feature for item in selected.values() for feature in item.get("features", [])})}
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()
