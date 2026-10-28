"""Offline configuration regressions, not evidence of an Envoy TLS handshake.

Envoy v1.31.2 common.proto defines match_typed_subject_alt_names. Identities
come from the certificate generators, independently of SNI or DNS routing.
The real same-CA/wrong-name control remains a separate runtime gate.
"""
import importlib.util
import re
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
SHORT_NAMES = {
    "ext_authz": "ext-authz", "iam": "iam-service", "hive": "hive-service",
    "telegraph": "telegraph-service", "manifesto": "manifesto-service",
}
KIND_NAMES = {
    "ext_authz": "ext-authz.aiforall-gateway.svc.cluster.local",
    **{name: name + ".aiforall-platform.svc.cluster.local"
       for name in ("iam", "hive", "telegraph", "manifesto")},
}


class UpstreamServerIdentityTests(unittest.TestCase):
    def assert_identities(self, config, expected):
        clusters = config["static_resources"]["clusters"]
        self.assertCountEqual([cluster["name"] for cluster in clusters], expected)
        for cluster in clusters:
            with self.subTest(cluster=cluster["name"]):
                socket = cluster["transport_socket"]
                self.assertEqual(socket["name"], "envoy.transport_sockets.tls")
                tls = socket["typed_config"]
                self.assertEqual(tls["@type"],
                                 "type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.UpstreamTlsContext")
                self.assertFalse(tls.get("auto_sni_san_validation", False))
                self.assertFalse(cluster.get("upstream_http_protocol_options", {}).get("auto_san_validation", False))
                common = tls["common_tls_context"]
                self.assertEqual(common["tls_certificates"], [{
                    "certificate_chain": {"filename": "/etc/envoy/certs/envoy-mesh.crt"},
                    "private_key": {"filename": "/etc/envoy/certs/envoy-mesh.key"},
                }])
                # Exact dictionary also excludes permissive validation overrides,
                # generic localhost SANs, untyped matchers and OR alternatives.
                self.assertEqual(common["validation_context"], {
                    "trusted_ca": {"filename": "/etc/envoy/certs/ca.crt"},
                    "match_typed_subject_alt_names": [{
                        "san_type": "DNS", "matcher": {"exact": expected[cluster["name"]]},
                    }],
                })

    def envoy_config(self, objects):
        configs = [obj for obj in objects if obj and obj["kind"] == "ConfigMap"
                   and obj["metadata"]["name"] == "envoy-mesh-config"]
        self.assertEqual(len(configs), 1)
        return yaml.safe_load(configs[0]["data"]["envoy.yaml"])

    def test_compose_clusters_require_their_server_dns_identity(self):
        config = yaml.safe_load((ROOT / "ops/deploy/mesh/envoy.yaml").read_text(encoding="utf-8"))
        self.assert_identities(config, SHORT_NAMES)

    def test_kind_clusters_require_their_server_dns_identity(self):
        source = ROOT / "ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml"
        self.assert_identities(self.envoy_config(yaml.safe_load_all(source.read_text(encoding="utf-8"))), KIND_NAMES)

    def test_expected_identities_are_issued_by_certificate_generators(self):
        source = (ROOT / "ops/scripts/generate-platform-mesh-certs.sh").read_text(encoding="utf-8")
        issued = dict(re.findall(r'^issue_leaf "([^"]+)"(?: "([^"]*)")?$', source, re.MULTILINE))
        self.assertIn('san="DNS:${name},DNS:localhost,IP:127.0.0.1"', source)
        for cluster, short in SHORT_NAMES.items():
            with self.subTest(cluster=cluster):
                self.assertEqual(issued[short], "DNS:" + KIND_NAMES[cluster])
        full = (ROOT / "ops/scripts/prepare-local-full-certs.sh").read_text(encoding="utf-8")
        self.assertIn("for name in iam hive telegraph manifesto; do", full)
        self.assertIn('"DNS:$name-service,DNS:$name,DNS:$name.aiforall-local-full.svc.cluster.local,DNS:localhost,IP:127.0.0.1"', full)
        self.assertIn("for name in envoy-mesh ext-authz mesh-client; do", full)
        self.assertIn('"DNS:$name,DNS:$name.aiforall-local-full.svc.cluster.local,DNS:localhost,IP:127.0.0.1"', full)

    def test_local_full_render_requires_its_rewritten_server_dns_identities(self):
        spec = importlib.util.spec_from_file_location("identity_renderer", ROOT / "ops/deploy/render-local-full.py")
        renderer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(renderer)
        self.assertEqual(renderer.ROOT, ROOT)
        # Actual offline Kustomize subprocess; no mocked rendering, main, output
        # files, API calls or Secret dumps. Environment failures are errors.
        objects = renderer.render("serveridentityvalidation")
        full_names = {cluster: ("ext-authz" if cluster == "ext_authz" else cluster)
                      + ".aiforall-local-full.svc.cluster.local" for cluster in SHORT_NAMES}
        self.assert_identities(self.envoy_config(objects), full_names)


if __name__ == "__main__":
    unittest.main()
