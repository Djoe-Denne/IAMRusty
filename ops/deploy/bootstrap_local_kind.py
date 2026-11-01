"""Shared native-Windows/Linux Kind + pinned CNI bootstrap, behind leasev2."""
import argparse
import json
from pathlib import Path
import re
import urllib.request
import local_full_target as authority

ROOT = Path(__file__).resolve().parents[2]


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
        with urllib.request.urlopen("https://raw.githubusercontent.com/projectcalico/calico/v3.29.7/manifests/calico.yaml", timeout=45) as response:
            manifest = response.read(2_000_001)
        if len(manifest) > 2_000_000:
            raise authority.AnchorError("Calico manifest exceeds bound")
        manifest = manifest.decode()
        pattern = r'# - name: CALICO_IPV4POOL_CIDR\s+#   value: "192.168.0.0/16"'
        if len(re.findall(pattern, manifest)) != 1:
            raise authority.AnchorError("pinned Calico shape changed")
        manifest = re.sub(pattern, '- name: CALICO_IPV4POOL_CIDR\n              value: "10.244.0.0/16"', manifest)
        k("apply", "-f", "-", stdin=manifest)
        k("-n", "kube-system", "set", "env", "daemonset/calico-node", "FELIX_IGNORELOOSERPF=true")
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
