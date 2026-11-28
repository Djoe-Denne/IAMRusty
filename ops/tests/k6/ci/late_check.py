"""Disposable relay that delays a real positive Check, never a test-generated allow."""

from copy import deepcopy
import http.client
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
from pathlib import Path
import ssl
import time

PROFILE = "/iam/api/me"
CHECK_DNS = "ext-authz.aiforall-local-full.svc.cluster.local"
RELAY_DNS = "late-check.aiforall-local-full.svc.cluster.local"
PORT = 18448
DELAY_SECONDS = 1.25
PRINCIPAL_HEADERS = ("x-principal-iss", "x-principal-sub")


def add_listener(config):
    static = config["static_resources"]
    ingress, = [item for item in static["listeners"] if item["name"] == "ci_browser"]
    check, = [item for item in static["clusters"] if item["name"] == "ext_authz"]
    listener = deepcopy(ingress)
    listener["name"] = "ci_late_check"
    listener["address"]["socket_address"]["port_value"] = PORT
    chain, = listener["filter_chains"]
    manager = chain["filters"][0]["typed_config"]
    manager["stat_prefix"] = "ci_late_check"
    manager["route_config"]["name"] = "ci_late_check_routes"
    host, = manager["route_config"]["virtual_hosts"]
    host["routes"] = [
        {"match": {"path": PROFILE}, "route": {"cluster": "iam"}},
        {"match": {"prefix": "/"}, "direct_response": {"status": 404}},
    ]
    ext_authz, = [item for item in manager["http_filters"] if item["name"] == "envoy.filters.http.ext_authz"]
    uri = ext_authz["typed_config"]["http_service"]["server_uri"]
    uri.update(uri=f"https://{RELAY_DNS}:8090/", cluster="late_check")
    upstream = deepcopy(check)
    upstream["name"] = "late_check"
    assignment = upstream["load_assignment"]
    assignment["cluster_name"] = "late_check"
    socket = assignment["endpoints"][0]["lb_endpoints"][0]["endpoint"]["address"]["socket_address"]
    socket["address"] = RELAY_DNS
    tls = upstream["transport_socket"]["typed_config"]
    tls["sni"] = RELAY_DNS
    tls["common_tls_context"]["validation_context"]["match_typed_subject_alt_names"] = [
        {"san_type": "DNS", "matcher": {"exact": RELAY_DNS}},
    ]
    static["listeners"].append(listener)
    static["clusters"].append(upstream)


class CheckRelay(BaseHTTPRequestHandler):
    def log_message(self, _format, *args):  # noqa: A002
        # The standard request logger can include credentials or URLs.
        return

    def do_GET(self):
        if self.path != PROFILE or "Authorization" not in self.headers:
            self.send_error(404)
            return
        connection = http.client.HTTPSConnection(CHECK_DNS, 8090, timeout=2, context=self.server.check_context)
        try:
            connection.request("GET", PROFILE, headers={"Authorization": self.headers["Authorization"]})
            response = connection.getresponse()
            principal = [response.getheader(name) for name in PRINCIPAL_HEADERS]
            if response.status != 200 or not all(principal):
                self.send_error(502)
                return
        except (OSError, http.client.HTTPException):
            self.send_error(502)
            return
        finally:
            connection.close()
        before = time.monotonic_ns()
        time.sleep(DELAY_SECONDS)
        attempted_ms = (time.monotonic_ns() - before) // 1_000_000
        try:
            self.send_response(200)
            for name, value in zip(PRINCIPAL_HEADERS, principal, strict=True):
                self.send_header(name, value)
            self.send_header("Content-Length", "0")
            self.end_headers()
        except OSError:
            # Envoy may already have closed the timed-out Check stream.
            pass
        print(json.dumps({"upstream_status": 200, "delayed_positive_attempt_ms": attempted_ms}), flush=True)


def serve():
    pki = Path("/app/certs")
    server_context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    server_context.load_cert_chain(pki / "late-check.crt", pki / "late-check.key")
    server_context.load_verify_locations(pki / "ca.crt")
    server_context.verify_mode = ssl.CERT_REQUIRED
    client_context = ssl.create_default_context(cafile=str(pki / "ca.crt"))
    client_context.load_cert_chain(pki / "late-check.crt", pki / "late-check.key")
    with HTTPServer(("0.0.0.0", 8090), CheckRelay) as server:
        server.socket = server_context.wrap_socket(server.socket, server_side=True)
        server.check_context = client_context
        server.serve_forever()


if __name__ == "__main__":
    serve()
