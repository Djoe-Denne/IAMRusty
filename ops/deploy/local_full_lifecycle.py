"""Interpret public lifecycle metadata; no workload inventory is encoded here."""
import json
import copy
from functools import lru_cache
import tomllib
import yaml

import local_full_helm as helm
from local_full_declarations import identity as declarations_identity


def profiles():
    return yaml.safe_load((helm.CHART / "files/lifecycle.yaml").read_text(encoding="utf-8"))["profiles"]


def profile(data):
    declarations = profiles()
    owned = {(e.get("kind"), e.get("namespace"), e.get("name")) for e in data.get("ownedResources", [])}
    def installed(name):
        return all((e["kind"], e["namespace"], e["name"]) in owned for e in declarations[name]["identities"])
    explicit = data.get("connectorProfile")
    if explicit:
        if explicit not in declarations or not installed(explicit):
            raise ValueError("selected connector profile is not installed in the original lease")
        return explicit
    candidates = [name for name, value in declarations.items() if value["identities"] and installed(name)]
    if len(candidates) > 1:
        raise ValueError("ambiguous connector profiles in environment lease")
    return candidates[0] if candidates else "base"


@lru_cache(maxsize=8)
def _objects(selected_profile, source_identity):
    return helm.render(selected_profile)


def objects(data):
    return copy.deepcopy(_objects(profile(data), declarations_identity()))


def coffer(data):
    candidates = [o for o in _objects(data.get("connectorProfile", "base"), declarations_identity())
                  if o["metadata"].get("annotations", {}).get("aiforall.dev/coffer") == "true"]
    if len(candidates) != 1:
        raise ValueError("unique declared coffer required")
    return {**candidates[0]["metadata"], "containerName": candidates[0]["spec"]["template"]["spec"]["containers"][0]["name"]}


def validate_public_input(target, read_owned, declarations, inputs):
    for source in declarations:
        obj = read_owned(target, source["kind"], source["name"])
        try:
            text = obj["data"][source["key"]]
            value = tomllib.loads(text) if source["format"] == "toml" else json.loads(text)
            for component in source["path"]:
                value = value[component]
            if value != inputs["client_id"]:
                raise ValueError
        except (ValueError, KeyError, TypeError, IndexError):
            raise ValueError("private input source differs from the declared consumer (values redacted)") from None
