"""Real Helm rendering of the opt-in disposable CI profile, no cluster needed."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[4]
HELM = Path(os.environ.get("AIFORALL_HELM") or shutil.which("helm") or
            ROOT / ".tmp/ops-iac-tools/helm-v4.3.1/windows-amd64/helm.exe")
CHART = ROOT / "ops/tests/k6/native/chart"


@unittest.skipUnless(HELM.is_file(), "existing Helm required; skip is not a CI qualification")
class CiRenderTests(unittest.TestCase):
    def render(self, enabled=True, namespace="aiforall-local-full", omit_image=False):
        images = {role: "fixture@sha256:" + "b" * 64
                  for role in ("iam", "oauth", "envoy", "idp", "ext-authz")}
        if omit_image:
            del images["ext-authz"]
        values = dict(name="k6-" + "a" * 32, runId="a" * 32, ciEnabled=enabled,
                      images=images, simulatorSource="# fixture",
                      envoySource=(ROOT / "ops/tests/k6/ci/config/envoy.yaml").read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory(prefix="k6-ci-render-") as directory:
            path = Path(directory) / "values.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            return subprocess.run([str(HELM), "template", "ci", str(CHART),
                                   "--namespace", namespace, "-f", str(path)],
                                  capture_output=True, text=True, timeout=30)

    def objects(self, **kwargs):
        result = self.render(**kwargs)
        self.assertEqual(result.returncode, 0, result.stderr)
        # Helm adds a known YAML document/source preamble. Strip only that
        # exact envelope, never search for a JSON suffix in diagnostic output.
        prefix = "---\n# Source: k6-native-fixtures/templates/fixtures.yaml\n"
        self.assertTrue(result.stdout.startswith(prefix))
        return json.loads(result.stdout[len(prefix):])["items"]

    def test_default_native_inventory_stays_unchanged(self):
        items = self.objects(enabled=False)
        self.assertEqual(len(items), 10)
        self.assertFalse(any(item["metadata"]["name"] in ("iam", "ext-authz") for item in items))

    def test_ci_adds_only_alias_and_real_ext_authz(self):
        items = self.objects()
        self.assertEqual(len(items), 13)
        alias = next(item for item in items if item["metadata"]["name"] == "iam")
        self.assertEqual(alias["spec"]["selector"], {
            "aiforall.dev/k6-run": "a" * 32, "aiforall.dev/k6-role": "iam"})
        deployment = next(item for item in items if item["kind"] == "Deployment"
                          and item["metadata"]["name"].endswith("-ext-authz"))
        pod = deployment["spec"]["template"]["spec"]
        self.assertFalse(pod["automountServiceAccountToken"])
        self.assertTrue(pod["securityContext"]["runAsNonRoot"])
        container = pod["containers"][0]
        env = {entry["name"]: entry["value"] for entry in container["env"]}
        self.assertEqual(env["EXT_AUTHZ_JWKS_URL"],
                         "https://iam.aiforall-local-full.svc.cluster.local:8443/iam/.well-known/jwks.json")
        self.assertEqual(env["EXT_AUTHZ_AUDIENCE"], "aiforall")
        self.assertEqual(container["imagePullPolicy"], "Never")
        self.assertIn("tcpSocket", container["readinessProbe"])
        self.assertFalse(container["securityContext"]["allowPrivilegeEscalation"])
        self.assertEqual(container["securityContext"]["capabilities"]["drop"], ["ALL"])

    def test_namespace_and_ext_authz_input_refusals(self):
        self.assertNotEqual(self.render(namespace="default").returncode, 0)
        self.assertNotEqual(self.render(omit_image=True).returncode, 0)

    def config(self):
        items = self.objects()
        return json.loads(items[0]["data"]["envoy.json"])

    def test_browser_tls_and_exact_route_exemptions(self):
        config = self.config()
        self.assertEqual(config["admin"]["address"]["socket_address"]["address"], "127.0.0.1")
        chain = config["static_resources"]["listeners"][0]["filter_chains"][0]
        tls = chain["transport_socket"]["typed_config"]
        self.assertFalse(tls["require_client_certificate"])
        self.assertIn("tls_certificates", tls["common_tls_context"])
        manager = chain["filters"][0]["typed_config"]
        routes = manager["route_config"]["virtual_hosts"][0]["routes"]
        self.assertEqual(len(routes), 4)
        self.assertEqual(routes[0]["match"]["path"], "/iam/health")
        self.assertEqual(routes[0]["match"]["headers"][0]["string_match"], {"exact": "GET"})
        regex = routes[1]["match"]["safe_regex"]["regex"]
        import re
        for suffix in ("login", "callback", "relink-callback"):
            self.assertIsNotNone(re.fullmatch(regex, "/iam/api/auth/gitlab/" + suffix))
        for suffix in ("link", "relink", "relink-start", "relink-start-alias", "login/extra"):
            self.assertIsNone(re.fullmatch(regex, "/iam/api/auth/gitlab/" + suffix))
        self.assertNotIn("typed_per_filter_config", routes[-1])
        filters = manager["http_filters"]
        self.assertEqual([f["name"] for f in filters], ["envoy.filters.http.lua",
                         "envoy.filters.http.ext_authz", "envoy.filters.http.router"])
        self.assertIn('"x-principal-"', filters[0]["typed_config"]["default_source_code"]["inline_string"])
        self.assertFalse(filters[1]["typed_config"]["failure_mode_allow"])

    def test_upstreams_keep_verified_mtls_and_fixture_iam_remap(self):
        config = self.config()
        clusters = {cluster["name"]: cluster for cluster in config["static_resources"]["clusters"]}
        self.assertEqual(set(clusters), {"iam", "ext_authz"})
        for name, cluster in clusters.items():
            tls = cluster["transport_socket"]["typed_config"]
            self.assertIn("tls_certificates", tls["common_tls_context"])
            context = tls["common_tls_context"]["validation_context"]
            self.assertEqual(context["trusted_ca"]["filename"], "/app/certs/ca.crt")
            expected = "iam.aiforall-local-full.svc.cluster.local" if name == "iam" else "ext-authz"
            self.assertEqual(tls["sni"], expected)
            self.assertEqual(context["match_typed_subject_alt_names"],
                             [{"san_type": "DNS", "matcher": {"exact": expected}}])
        address = clusters["iam"]["load_assignment"]["endpoints"][0]["lb_endpoints"][0]["endpoint"]["address"]["socket_address"]
        self.assertEqual(address["address"], "k6-" + "a" * 32 + "-iam.aiforall-local-full.svc.cluster.local")


if __name__ == "__main__":
    unittest.main()
