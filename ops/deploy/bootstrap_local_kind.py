"""Shared native-Windows/Linux Kind + pinned CNI bootstrap, behind leasev2."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import local_full_target as authority

ROOT = Path(__file__).resolve().parents[2]


def calico_manifest():
    """Verify the vendored bytes and render locally, never through the API."""
    overlay = ROOT / "ops/deploy/third-party/calico"
    provenance = json.loads((overlay / "provenance.json").read_text(encoding="utf-8"))
    contents = (overlay / "base/calico.yaml").read_bytes()
    if (provenance["version"] != "3.29.7" or len(contents) != provenance["bytes"]
            or hashlib.sha256(contents).hexdigest() != provenance["sha256"]):
        raise authority.AnchorError("vendored Calico checksum/version mismatch")
    result = subprocess.run(["kubectl", "kustomize", str(overlay)], capture_output=True, text=True, timeout=60)
    if result.returncode or not result.stdout.strip():
        raise authority.AnchorError("offline Calico rendering failed")
    return result.stdout


def initialize(lease, cluster, config, after_final_it=False):
    if (Path(config).name == "cluster-local-full.yaml") != (cluster == "aiforall-local-full"):
        raise authority.AnchorError("full topology/target mismatch")
    authority.bootstrap(lease, cluster, config, after_final_it)
    target = authority.LocalClusterTargetAnchor(lease, expected_cluster=cluster)

    def k(*args, stdin=None, timeout=660):
        return target.kubectl(list(args), text=True, input=stdin, timeout=timeout)

    sets = json.loads(k("-n", "kube-system", "get", "daemonsets", "-o", "json"))["items"]
    if any(re.search(r"kindnet|cilium|flannel|weave|antrea|canal|kube-router", item["metadata"]["name"]) for item in sets):
        raise authority.AnchorError("incompatible CNI; no replacement")
    cidrs = k("get", "nodes", "-o", "jsonpath={.items[*].spec.podCIDR}").split()
    if not cidrs or any(not re.fullmatch(r"10\.244\.\d+\.\d+/(1[6-9]|2\d|3[0-2])", value) for value in cidrs):
        raise authority.AnchorError("incompatible/unavailable CIDR")
    calico = [item for item in sets if item["metadata"]["name"] == "calico-node"]
    if not calico:
        if any(item["metadata"]["name"] != "kube-proxy" for item in sets):
            raise authority.AnchorError("unknown DaemonSet; no adoption")
        k("apply", "-f", "-", stdin=calico_manifest())
    else:
        if len(calico) != 1 or any(not c["image"].endswith(":v3.29.7") for c in calico[0]["spec"]["template"]["spec"]["containers"]):
            raise authority.AnchorError("incompatible Calico pin")
        pools = json.loads(k("get", "ippools", "-o", "json"))["items"]
        if len(pools) != 1 or pools[0]["spec"]["cidr"] != "10.244.0.0/16":
            raise authority.AnchorError("pool mismatch; no patch/delete")
    for resource in ("daemonset/calico-node", "deployment/calico-kube-controllers", "deployment/coredns"):
        k("-n", "kube-system", "rollout", "status", resource, "--timeout=600s")
    k("wait", "--for=condition=Ready", "nodes", "--all", "--timeout=300s")
    pools = json.loads(k("get", "ippools", "-o", "json"))["items"]
    if len(pools) != 1 or pools[0]["spec"]["cidr"] != "10.244.0.0/16":
        raise authority.AnchorError("final pool mismatch")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lease", type=Path, required=True)
    parser.add_argument("--cluster", choices=list(authority.ALLOWED), required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--after-final-it", action="store_true")
    args = parser.parse_args()
    initialize(args.lease, args.cluster, args.config, args.after_final_it)


if __name__ == "__main__":
    from private_diagnostics import operation
    with operation("bootstrap-kind"):
        main()
