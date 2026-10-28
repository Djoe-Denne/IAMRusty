"""Final render checks only; never prints manifests, Secrets or key material."""
import copy
import base64
import contextlib
import importlib.util
import io
import json
import sys
import tempfile
from pathlib import Path
from types import SimpleNamespace
import unittest
import tomllib
from unittest.mock import patch

import yaml

ROOT = Path(__file__).resolve().parents[2]


def load_renderer():
    spec = importlib.util.spec_from_file_location("oauth_renderer", ROOT / "ops/deploy/render-local-full.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_deployer():
    sys.path.insert(0, str(ROOT / "ops/deploy"))
    try:
        spec = importlib.util.spec_from_file_location("oauth_deployer", ROOT / "ops/deploy/deploy-local-full.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module
    finally:
        sys.path.pop(0)


class BrowserAlignmentTests(unittest.TestCase):
    def test_full_http_downstream_preserves_internal_mtls_and_shared_tls(self):
        source = yaml.safe_load_all((ROOT / "ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml").read_text())
        original = next(obj for obj in source if obj["kind"] == "ConfigMap")
        shared = yaml.safe_load(original["data"]["envoy.yaml"])
        objects = load_renderer().render("oauthalignmentvalidation")
        config = next(obj for obj in objects if obj["kind"] == "ConfigMap"
                      and obj["metadata"]["name"] == "envoy-mesh-config")
        final = yaml.safe_load(config["data"]["envoy.yaml"])
        before = shared["static_resources"]["listeners"][0]
        after = final["static_resources"]["listeners"][0]
        self.assertEqual(after["name"], "listener_mesh")
        self.assertEqual(after["address"]["socket_address"]["port_value"], 10000)
        self.assertIn("transport_socket", before["filter_chains"][0])
        self.assertNotIn("transport_socket", after["filter_chains"][0])
        self.assertNotIn("filter_chain_match", after["filter_chains"][0])
        # No shared-source mutation; compare every upstream transport after ONLY
        # the existing namespace rewrite, including SAN matchers/client cert.
        for old, new in zip(shared["static_resources"]["clusters"], final["static_resources"]["clusters"], strict=True):
            expected = yaml.safe_load(yaml.safe_dump(old["transport_socket"]).replace(
                "aiforall-platform.svc", "aiforall-local-full.svc").replace(
                "aiforall-gateway.svc", "aiforall-local-full.svc"))
            self.assertEqual(new["transport_socket"], expected)
        filters = after["filter_chains"][0]["filters"][0]["typed_config"]["http_filters"]
        self.assertEqual([f["name"] for f in filters], [
            "envoy.filters.http.lua", "envoy.filters.http.ext_authz", "envoy.filters.http.router"])
        self.assertFalse(filters[1]["typed_config"]["failure_mode_allow"])

    def test_downstream_conversion_rejects_ambiguous_or_foreign_listener(self):
        renderer = load_renderer()
        sample = {"static_resources": {"listeners": [{"name": "listener_mesh", "address": {
            "socket_address": {"port_value": 10000}}, "filter_chains": [
                {"transport_socket": {"name": "tls"}, "filters": [{"name": "http"}]}]}]}}
        for change in ([], [copy.deepcopy(sample["static_resources"]["listeners"][0])] * 2,
                       [{"name": "foreign"}]):
            value = copy.deepcopy(sample)
            value["static_resources"]["listeners"] = change
            with self.assertRaises(ValueError):
                renderer.http_downstream(value)

    def test_browser_sequence_keeps_native_cookie_jar_and_request_scoped_auth(self):
        source = (ROOT / "ops/tests/oauth-browser-local-full.js").read_text()
        for forbidden in (".addCookies(", ".setExtraHTTPHeaders(", ".request.get(",
                          "APIRequestContext.new", "ignoreHTTPSErrors:", "browser_run_code_unsafe"):
            self.assertNotIn(forbidden, source)
        self.assertEqual(source.count('headers: {Authorization:'), 1)
        for required in ('start.status !== 200', 'start.body.auth_url',
                         'result.status !== 200', 'result.body.operation !== "login"',
                         'result.status === 202', 'result = await login()',
                         'await page.goto(start.body.auth_url)',
                         'PENDING_GATEWAY_IAM_IDP_CORRELATION',
                         'nonce.domain !== "127.0.0.1"', 'nonce.httpOnly',
                         'nonce.sameSite !== "Lax"', 'callbackBearer', 'callbackCookie'):
            self.assertIn(required, source)

    def test_cert_option_is_fresh_and_connector_leaf_is_scoped(self):
        source = (ROOT / "ops/scripts/prepare-local-full-certs.sh").read_text()
        self.assertIn('--oauth-proof)', source)
        self.assertLess(source.index('if [ -e "$LOCAL_FULL_CERT_DIR" ]'), source.index('ca "$LOCAL_FULL_CERT_DIR/mesh"'))
        leaf = next(line for line in source.splitlines() if 'mesh" gitlab-connect ' in line)
        self.assertIn('DNS:gitlab-connect,DNS:gitlab-connect.aiforall-local-full.svc.cluster.local', leaf)
        self.assertNotIn('localhost', leaf)

    def test_proof_images_are_explicit_and_preserve_default_sources(self):
        # Import-only: no Docker inspection, tagging, build or runtime.
        sys.path.insert(0, str(ROOT / "ops/deploy"))
        try:
            spec = importlib.util.spec_from_file_location("oauth_images", ROOT / "ops/deploy/local-full-images.py")
            images = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(images)
        finally:
            sys.path.pop(0)
        self.assertEqual(len(images.source_images()), 9)
        self.assertEqual(images.source_images(), images.SOURCES)
        self.assertEqual(images.source_images(True), images.SOURCES + ["aiforall-gitlab-connect-service:latest"])
        self.assertEqual(len(images.SOURCES), 9, "opt-in must not mutate the default list")

    def test_proof_registry_drives_exact_relink_exemption(self):
        # Boundary unit: real renderer transforms fixture Kustomize output. The
        # final proof-overlay render remains a separate gate once policy is set.
        renderer = load_renderer()
        source = yaml.safe_load_all((ROOT / "ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml").read_text())
        mesh = next(obj for obj in source if obj["kind"] == "ConfigMap")
        mesh["metadata"]["namespace"] = "aiforall-local-full"
        configs = ['[idp]\nconnectors = []\n',
                   '[[idp.connectors]]\nid = "gitlab"\nredirect_uris = ["http://127.0.0.1:18080/iam/api/auth/gitlab/relink-callback"]\n']
        for enabled, config in enumerate(configs):
            iam = {"apiVersion": "v1", "kind": "Secret" if enabled else "ConfigMap", "metadata": {
                "name": "oauth-proof-iam-config-effective" if enabled else "iam-dev-config-effective", "namespace": "aiforall-local-full"},
                "data": {"development.toml": base64.b64encode(config.encode()).decode() if enabled else config}}
            output = SimpleNamespace(returncode=0, stdout=yaml.safe_dump_all([mesh, iam]))
            with patch.object(renderer.subprocess, "run", return_value=output) as command:
                result = renderer.render("oauthregistryvalidation", oauth_proof=bool(enabled))
            selected = renderer.OVERLAY / "oauth-proof" if enabled else renderer.OVERLAY
            self.assertEqual(command.call_args.args[0][-1], str(selected))
            final = yaml.safe_load(result[0]["data"]["envoy.yaml"])
            manager = final["static_resources"]["listeners"][0]["filter_chains"][0]["filters"][0]["typed_config"]
            routes = manager["route_config"]["virtual_hosts"][0]["routes"]
            exact = [route["match"]["path"] for route in routes
                     if route.get("match", {}).get("path", "").endswith("/relink-callback")]
            self.assertEqual(exact, ["/iam/api/auth/gitlab/relink-callback"] if enabled else [])
            self.assertFalse(any(route.get("match", {}).get("path", "").endswith("/relink/callback") for route in routes))

    def profile_configs(self, inputs=None):
        objects = load_renderer().render("oauthprofilevalidation", oauth_proof=True, proof_inputs=inputs)
        def config(prefix):
            obj = next(obj for obj in objects if obj["kind"] == "Secret" and obj["metadata"]["name"].startswith(prefix))
            return tomllib.loads(base64.b64decode("".join(obj["data"]["development.toml"].split())).decode())
        return objects, config("oauth-proof-iam-config-"), config("oauth-proof-gitlab-config-")

    def test_oauth_proof_keeps_8080_authority_and_exact_18080_redirects(self):
        objects, iam, gitlab = self.profile_configs()
        self.assertEqual(iam["jwt"]["public_base_url"], "http://127.0.0.1:8080")
        for config in (iam, gitlab):
            self.assertEqual(config["auth"]["jwt"]["platform_issuer"], "http://127.0.0.1:8080/iam")
            self.assertIs(config["auth"]["jwt"]["allow_historical_platform_issuer"], False)
            self.assertEqual(config["auth"]["jwt"]["jwks_url"], "http://iam:8080/iam/.well-known/jwks.json")
        redirects = ["http://127.0.0.1:18080/iam/api/auth/gitlab/callback",
                     "http://127.0.0.1:18080/iam/api/auth/gitlab/relink-callback"]
        self.assertEqual(iam["idp"]["connectors"][0]["redirect_uris"], redirects)
        self.assertEqual(gitlab["gitlab"]["redirect_uris"], redirects)
        for obj in objects:
            name = obj["metadata"]["name"]
            if obj["kind"] == "Deployment" and name in {"hive", "telegraph", "manifesto", "lazaret", "sentinel-sync"}:
                env = obj["spec"]["template"]["spec"]["containers"][0]["env"]
                policies = [entry for entry in env if entry["name"].endswith("__PLATFORM_ISSUER")]
                self.assertEqual([entry.get("value") for entry in policies], ["http://127.0.0.1:8080/iam"])

    def test_oauth_registry_redirect_is_independent_of_platform_public_base_url(self):
        objects, iam, _ = self.profile_configs()
        self.assertEqual([entry["id"] for entry in iam["idp"]["connectors"]], ["gitlab"])
        mesh = next(obj for obj in objects if obj["kind"] == "ConfigMap" and obj["metadata"]["name"] == "envoy-mesh-config")
        final = yaml.safe_load(mesh["data"]["envoy.yaml"])
        manager = final["static_resources"]["listeners"][0]["filter_chains"][0]["filters"][0]["typed_config"]
        routes = manager["route_config"]["virtual_hosts"][0]["routes"]
        paths = [route.get("match", {}).get("path") for route in routes]
        self.assertEqual(paths.count("/iam/api/auth/gitlab/relink-callback"), 1)
        self.assertNotIn("/iam/api/auth/github/relink-callback", paths)
        self.assertNotIn("/iam/api/auth/gitlab/relink/callback", paths)
        default = tomllib.loads((ROOT / "ops/deploy/apps/overlays/local-full/config/iam.toml").read_text())
        self.assertEqual(default["idp"]["connectors"], [])

    def test_proof_origin_redirects_pkce_and_private_connector(self):
        objects, iam, gitlab = self.profile_configs()
        self.assertEqual(iam["idp"]["connectors"][0]["base_url"], "https://gitlab-connect:8443/gitlab-connect")
        self.assertIs(iam["idp"]["connectors"][0]["pkce_supported"], True)
        self.assertIs(gitlab["gitlab"]["pkce_supported"], True)
        self.assertEqual(gitlab["gitlab"]["transport_security"], "isolated_test")
        self.assertEqual(gitlab["gitlab"]["auth_url"], "http://localhost:18081/oauth/authorize")
        apps = {obj["metadata"]["name"]: obj for obj in objects if obj["kind"] == "Deployment"}
        pod = apps["iam"]["spec"]["template"]["spec"]
        init = next(container for container in pod["initContainers"] if container["name"] == "oauth-proof-trust")
        self.assertEqual(init["image"], pod["containers"][0]["image"])
        self.assertIn('/etc/ssl/certs/ca-certificates.crt', init["command"][-1])
        self.assertIn('/public-ca/ca.crt', init["command"][-1])
        connector = apps["gitlab-connect"]["spec"]["template"]["spec"]
        references = [volume.get("secret", {}).get("secretName", "") for volume in connector["volumes"]]
        self.assertFalse(any("platform-mesh-certs" in name or "public-ca" in name for name in references))
        self.assertTrue(any(name.startswith("oauth-proof-connector-tls-") for name in references))
        for policy in (obj for obj in objects if obj["kind"] == "NetworkPolicy" and obj["metadata"]["name"].startswith("oauth-proof-")):
            self.assertNotIn("ipBlock", json.dumps(policy))
            self.assertNotIn("namespaceSelector", json.dumps(policy))

    def test_proof_private_inputs_drive_hash_without_secret_output(self):
        inputs = dict(version=1, run_id="oauthprofilevalidation", client_id="proofclient",
                      client_secret="c" * 48, hmac_secret="a" * 64, oauth_state_secret="b" * 64)
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            first, iam, gitlab = self.profile_configs(inputs)
            second, _, _ = self.profile_configs({**inputs, "client_secret": "d" * 48})
        for key in ("client_secret", "hmac_secret", "oauth_state_secret"):
            self.assertTrue(inputs[key] not in output.getvalue(), "private input was printed")
        self.assertTrue(iam["idp"]["connectors"][0]["hmac_secret"] == gitlab["gitlab"]["hmac_secret"], "HMAC mismatch")
        def identity(objects):
            return next(obj for obj in objects if obj["kind"] == "Deployment" and obj["metadata"]["name"] == "iam")["spec"]["template"]["metadata"]["annotations"]["aiforall.dev/config-sha256"]
        self.assertNotEqual(identity(first), identity(second))
        renderer = load_renderer()
        with patch.object(renderer.subprocess, "run") as command:
            for invalid in ({**inputs, "unknown": "field"}, {**inputs, "client_secret": '"\\n'},
                            {**inputs, "run_id": "anotherproof12345"}):
                with self.assertRaises(ValueError):
                    renderer.render(inputs["run_id"], oauth_proof=True, proof_inputs=invalid)
            command.assert_not_called()
        deployer = load_deployer()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "inputs.json"
            path.write_text(json.dumps(inputs))
            with patch.object(deployer, "private_file", return_value=False):
                with self.assertRaises(ValueError):
                    deployer.oauth_inputs(path)
            with patch.object(deployer, "private_file", return_value=True):
                self.assertTrue(deployer.oauth_inputs(path) == inputs, "private input did not round trip")
                path.write_text(json.dumps(inputs)[:-1] + ',"run_id":"duplicate"}')
                with self.assertRaises(ValueError):
                    deployer.oauth_inputs(path)
            mesh = Path(directory) / "mesh"
            mesh.mkdir()
            for name in ("ca.crt", "ca.key", "gitlab-connect.crt", "gitlab-connect.key", "envoy-mesh.key"):
                (mesh / name).write_bytes(b"synthetic-fixture-" + name.encode())
            certs, digest = deployer.oauth_cert_inputs(Path(directory), inputs["run_id"])
            self.assertEqual([set(cert["data"]) for cert in certs], [{"ca.crt"}, {"tls.crt", "tls.key"}])
            self.assertEqual(len(digest), 64)
        foreign = {"apiVersion": "v1", "kind": "Service", "metadata": {
            "name": "gitlab-connect", "namespace": "aiforall-local-full"}}
        with patch.object(deployer, "k", return_value="foreign-uid") as command:
            with self.assertRaises(ValueError):
                deployer.preflight_oauth_objects([foreign], {"ownedResources": []})
            self.assertEqual(command.call_args.args[0], "get")


if __name__ == "__main__":
    unittest.main()
