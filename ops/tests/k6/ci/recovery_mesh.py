"""The companion route matcher from the opt-in Kind Envoy configuration."""

from copy import deepcopy

import yaml

from binaries import ROOT, require
from pki import COMPANIONS, mesh_hostname

KIND_ENVOY = ROOT / "ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml"


def routes(config, listener_name):
    listener, = [item for item in config["static_resources"]["listeners"] if item["name"] == listener_name]
    filter_chain, = listener["filter_chains"]
    connection_manager, = [item for item in filter_chain["filters"]
                           if item["name"] == "envoy.filters.network.http_connection_manager"]
    vhost, = connection_manager["typed_config"]["route_config"]["virtual_hosts"]
    return vhost["routes"]


def add_companions(config, run_id):
    overlay, = [item for item in yaml.safe_load_all(KIND_ENVOY.read_text(encoding="utf-8"))
                if item["kind"] == "ConfigMap" and item["metadata"]["name"] == "envoy-mesh-config"]
    kind = yaml.safe_load(overlay["data"]["envoy.yaml"])
    companion_routes = [route for route in routes(kind, "listener_mesh")
                        if route.get("route", {}).get("cluster") in COMPANIONS]
    require(len(companion_routes) == 5, "E2E_COMPANION_MESH_ROUTE_INVENTORY")
    routes(config, "ci_browser").extend(companion_routes)

    clusters = config["static_resources"]["clusters"]
    iam, = [cluster for cluster in clusters if cluster["name"] == "iam"]
    for service in COMPANIONS:
        host = mesh_hostname(run_id, service)
        cluster = deepcopy(iam)
        cluster["name"] = service
        assignment = cluster["load_assignment"]
        assignment["cluster_name"] = service
        endpoint, = assignment["endpoints"]
        backend, = endpoint["lb_endpoints"]
        backend["endpoint"]["address"]["socket_address"]["address"] = host
        tls = cluster["transport_socket"]["typed_config"]
        tls["sni"] = host
        peer, = tls["common_tls_context"]["validation_context"]["match_typed_subject_alt_names"]
        peer["matcher"]["exact"] = host
        clusters.append(cluster)


def create_services(campaign):
    for service in COMPANIONS:
        role = "mesh-" + service
        campaign.create({
            "apiVersion": "v1", "kind": "Service",
            "metadata": {"name": campaign.state["name"] + "-" + role},
            "spec": {
                "selector": {"aiforall.dev/k6-run": campaign.state["run_id"], "aiforall.dev/k6-role": role},
                "ports": [{"name": "https", "port": 8443, "targetPort": "https"}],
            },
        })
