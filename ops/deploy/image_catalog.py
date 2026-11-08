"""Explicit Linux deliverables; never discover executables in a shared Cargo cache."""
import hashlib
import json

FRESHNESS_POLICY = "snapshot-final-v1"


def service(directory, package, migration=None, source=None):
    outputs = {package: package}
    if migration:
        outputs[migration[0]] = migration[1]
    return {"source": source or "aiforall-" + package + ":latest",
            "dockerfile": "services/" + directory + "/Dockerfile", "outputs": outputs,
            "inputs": ["services/" + directory + "/"]}


TARGETS = {
    "iam": service("IAMRusty", "iam-service", ("iam-migration", "iammigration")),
    "hive": service("Hive", "hive-service", ("hive-migration", "hivemigration")),
    "telegraph": service("Telegraph", "telegraph-service", ("telegraph-migration", "telegraphmigration")),
    "manifesto": service("Manifesto", "manifesto-service", ("manifesto-migration", "manifestomigration")),
    "lazaret": service("Lazaret", "lazaret-service", ("lazaret-migration", "migration"),
                       "aiforall-lazaret-service:local-full"),
    "github": service("GitHubConnect", "github-connect-service"),
    "gitlab": service("GitLabConnect", "gitlab-connect-service"),
    "oauth": service("OAuthConnect", "oauth-connect-service"),
    "sentinel": {"source": "aiforall-sentinel-sync:local-full", "dockerfile": "ops/deploy/build/Dockerfile.worker",
                 "outputs": {"sentinel-sync": "sentinel-sync"}, "inputs": ["workers/sentinel-sync/"]},
    "ext-authz": {"source": "aiforall-ext-authz:local", "dockerfile": "ops/deploy/build/Dockerfile.ext-authz",
                  "outputs": {"ext-authz": "ext-authz"},
                  "inputs": ["workers/ext-authz/", "services/IAMRusty/domain/"]},
    "controller": {"source": "aiforall-apparatus-controller:j2",
                   "dockerfile": "ops/deploy/build/Dockerfile.controller",
                   "outputs": {"apparatus-operator": "apparatus-controller"},
                   "inputs": ["workers/apparatus-operator/", "services/Lazaret/"],
                   "features": ["apparatus-operator/controller"]},
}

LOCAL_FULL = ("iam", "hive", "telegraph", "manifesto", "lazaret", "sentinel", "ext-authz", "controller")


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
