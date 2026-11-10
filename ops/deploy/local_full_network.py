"""UID/RV/spec CAS for measured API inputs. Desired specs come only from Helm."""
import ipaddress
import json

import local_full_helm as helm


def control_plane(nodes):
    if len(nodes) != 1:
        raise ValueError("unique local control-plane required for network input")
    addresses = [item["address"] for item in nodes[0]["status"]["addresses"] if item["type"] == "InternalIP"]
    if len(addresses) != 1:
        raise ValueError("unique local control-plane IPv4 required")
    address = ipaddress.IPv4Address(addresses[0])
    if not address.is_private or address.is_loopback or address.is_multicast or address.is_unspecified:
        raise ValueError("local control-plane address refused")
    return nodes[0], address


def api_preimages(target, address, read_owned, *, partial=False):
    network = {"apiCidr": str(ipaddress.IPv4Address(address)) + "/32", "ingressCidr": ""}
    desired = [o for o in helm.render(phase="foundation", values={"network": network})
               if o["kind"] == "NetworkPolicy" and o["metadata"]["name"] == "allow-api"]
    changes = []
    for obj in desired:
        metadata = obj["metadata"]
        if partial and not any(e.get("kind") == "NetworkPolicy" and e.get("name") == metadata["name"]
                              and e.get("namespace") == metadata["namespace"] for e in target.data.get("ownedResources", [])):
            continue
        current = read_owned(target, "NetworkPolicy", metadata["name"], metadata["namespace"])
        peers = current["spec"].get("egress", [{}])[0].get("to", [])
        if len(peers) not in {1, 2}:
            raise ValueError("unexpected local API peer count")
        previous = ""
        if len(peers) == 2:
            previous = peers[1].get("ipBlock", {}).get("cidr", "")
            cidr = ipaddress.IPv4Network(previous)
            if cidr.prefixlen != 32 or peers[1] != {"ipBlock": {"cidr": str(cidr)}}:
                raise ValueError("unexpected local API endpoint peer")
        expected = next(o for o in helm.render(phase="foundation", values={"network": {"apiCidr": previous, "ingressCidr": ""}})
                        if o["kind"] == "NetworkPolicy" and o["metadata"]["name"] == metadata["name"]
                        and o["metadata"]["namespace"] == metadata["namespace"])
        if current["spec"] != expected["spec"]:
            raise ValueError("unexpected local API policy; no automatic overwrite")
        if current["spec"] != obj["spec"]:
            changes.append((metadata["namespace"], current, obj["spec"]))
    return changes


def apply_changes(target, changes):
    for namespace, current, desired in changes:
        patch = [{"op": "test", "path": "/metadata/uid", "value": current["metadata"]["uid"]},
                 {"op": "test", "path": "/metadata/resourceVersion", "value": current["metadata"]["resourceVersion"]},
                 {"op": "test", "path": "/spec", "value": current["spec"]},
                 {"op": "replace", "path": "/spec", "value": desired}]
        raw = target.kubectl(["patch", "networkpolicy/" + current["metadata"]["name"], "-n", namespace,
            "--type=json", "-p", json.dumps(patch), "-o", "json"], text=True, timeout=30)
        ack = json.loads(raw)
        if ack["metadata"]["uid"] != current["metadata"]["uid"] or ack["spec"] != desired:
            raise ValueError("API policy patch ACK differs; no success claimed")
