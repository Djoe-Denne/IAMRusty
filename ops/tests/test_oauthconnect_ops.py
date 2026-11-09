"""Offline OAuthConnect deployment oracles; no Docker, cluster or real secrets."""
import copy
import importlib.util
import json
from pathlib import Path
import re
import sys
import tomllib
import unittest
from types import SimpleNamespace
from unittest.mock import Mock, patch

import yaml

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ops/deploy"))
import local_full_oauthconnect as oauth
import local_full_environment as environment
from image_catalog import select


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


renderer = load("oauth_ops_renderer", "ops/deploy/render-local-full.py")
workload = load("oauth_ops_workload", "ops/deploy/apps/overlays/local-full/helpers/openbao-workload.py")
images = load("oauth_ops_images", "ops/deploy/local-full-images.py")
simulator = load("oauth_ops_simulator", "ops/tests/support/oauth-idp-simulator.py")


def credentials():
    return ({"version": 1, "client_id": "github-public", "client_secret": "synthetic-github-secret", "hmac_secret": "a" * 64},
            {"version": 1, "client_id": "simulated-public", "client_secret": "synthetic-simulated-secret", "hmac_secret": "b" * 64})


class OAuthProfileTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.objects = renderer.render("oauthopsregression", oauth_connect=True)

    def obj(self, kind, name):
        return next(o for o in self.objects if o["kind"] == kind and o["metadata"]["name"] == name)

    def test_two_instances_one_real_connector_no_legacy_deployment(self):
        names = {o["metadata"]["name"] for o in self.objects if o["kind"] == "Deployment"}
        self.assertIn("oauth-connect", names)
        self.assertIn("oauth-idp-simulator", names)
        self.assertTrue(names.isdisjoint({"github-connect", "gitlab-connect"}))
        iam = tomllib.loads(next(o for o in self.objects if o["kind"] == "ConfigMap"
                                and o["metadata"]["name"].startswith("iam-dev-config"))["data"]["development.toml"])
        self.assertEqual([c["id"] for c in iam["idp"]["connectors"]], ["github", "simulated"])
        for c in iam["idp"]["connectors"]:
            self.assertEqual(c["base_url"], "https://oauth-connect:8443/oauth-connect/" + c["id"])
            self.assertEqual(c["pkce_supported"], c["id"] == "simulated")

    def test_public_config_reuses_github_refs_and_isolates_simulated(self):
        config = tomllib.loads(self.obj("ConfigMap", "oauth-connect-config")["data"]["development.toml"])
        github, simulated = config["providers"]
        self.assertEqual(github["client_secret_ref"]["path"], "github-connect/oauth")
        self.assertEqual(github["hmac_secret_ref"]["path"], "github-connect/iam")
        self.assertEqual(simulated["hmac_secret_ref"]["path"], "oauth-connect/simulated/iam")
        self.assertEqual(github["transport_security"], "verified")
        self.assertEqual(simulated["transport_security"], "isolated_test")
        self.assertEqual(github["scopes"], ["user:email"])
        self.assertNotIn("auth", config)
        for provider in config["providers"]:
            self.assertNotIn("client_secret", provider)
            self.assertNotIn("hmac_secret", provider)

    def test_native_agent_private_snapshots_and_security(self):
        for name in ("oauth-connect", "oauth-idp-simulator", "iam"):
            pod = self.obj("Deployment", name)["spec"]["template"]["spec"]
            self.assertFalse(pod["automountServiceAccountToken"])
            self.assertTrue(pod["securityContext"]["runAsNonRoot"])
            self.assertTrue(any(v.get("emptyDir", {}).get("medium") == "Memory" for v in pod["volumes"]))
        pod = self.obj("Deployment", "oauth-connect")["spec"]["template"]["spec"]
        agent = pod["initContainers"][0]
        self.assertEqual(agent["restartPolicy"], "Always")
        self.assertIn("startupProbe", agent)
        for c in pod["containers"] + pod["initContainers"]:
            self.assertTrue(c["securityContext"]["readOnlyRootFilesystem"])
            self.assertFalse(c["securityContext"]["allowPrivilegeEscalation"])
            self.assertEqual(c["securityContext"]["capabilities"]["drop"], ["ALL"])
            self.assertEqual(set(c["resources"]), {"requests", "limits"})
        self.assertEqual(pod["containers"][0]["readinessProbe"]["httpGet"]["path"], "/oauth-connect/ready")

    def test_connector_disables_service_links_conflicting_with_strict_config_prefix(self):
        pod = self.obj("Deployment", "oauth-connect")["spec"]["template"]["spec"]
        self.assertIs(pod.get("enableServiceLinks"), False)
        self.assertFalse(any(e["name"].startswith("OAUTH_CONNECT_")
                             for c in pod["containers"] for e in c.get("env", [])))

    def test_images_are_selected_explicitly_and_require_immutable_mapping(self):
        self.assertEqual(select(["oauth"])["oauth"]["outputs"], {"oauth-connect-service": "oauth-connect-service"})
        selected = images.source_images(oauth_connect=True)
        self.assertIn(oauth.IMAGE, selected)
        self.assertNotIn(oauth.IMAGE, images.source_images())
        mapping = {name: name.split(":")[0] + "@sha256:" + "1" * 64 for name in selected}
        result = renderer.render("oauthopsregression", mapping, oauth_connect=True)
        app = next(o for o in result if o["kind"] == "Deployment" and o["metadata"]["name"] == "oauth-connect")
        self.assertEqual(app["spec"]["template"]["spec"]["containers"][0]["imagePullPolicy"], "IfNotPresent")
        del mapping[oauth.IMAGE]
        with self.assertRaisesRegex(ValueError, "missing immutable"):
            renderer.render("oauthopsregression", mapping, oauth_connect=True)

    def test_profiles_cannot_be_combined(self):
        for flags in ({"github_openbao": True}, {"oauth_proof": True}):
            with self.assertRaises(ValueError):
                renderer.render("oauthopsregression", oauth_connect=True, **flags)
            with self.assertRaises(ValueError):
                images.source_images(oauth_connect=True, **flags)

    def test_private_or_populated_preimage_is_refused_without_mutation(self):
        for change in ("registry", "marker", "trust"):
            objects = renderer.render("oauthopsregression")
            pod, _, config, text = renderer.local_full_openbao.iam_template(objects)
            if change == "registry":
                config["data"]["development.toml"] = text.replace("connectors = []", 'connectors = [{id="foreign"}]')
            elif change == "marker":
                config["data"]["development.toml"] += '\nunknown = "__PRIVATE_MARKER__"\n'
            else:
                pod["containers"][0].setdefault("env", []).append({"name": "SSL_CERT_FILE", "value": "/foreign/ca"})
            before = copy.deepcopy(objects)
            with self.assertRaises(ValueError):
                oauth.configure(objects, "oauthopsregression", "github-client", "simulated-client")
            self.assertEqual(objects, before)

    def test_envoy_only_adds_exact_simulated_relink_callback_get_head(self):
        envoy = yaml.safe_load(self.obj("ConfigMap", "envoy-mesh-config")["data"]["envoy.yaml"])
        config = envoy["static_resources"]["listeners"][0]["filter_chains"][0]["filters"][0]["typed_config"]
        routes = config["route_config"]["virtual_hosts"][0]["routes"]
        callbacks = [r for r in routes if r["match"].get("path", "").endswith("/relink-callback")]
        self.assertEqual({r["match"]["path"] for r in callbacks}, {
            "/iam/api/auth/github/relink-callback", "/iam/api/auth/simulated/relink-callback"})
        for route in callbacks:
            self.assertEqual(route["match"]["headers"][0]["string_match"]["safe_regex"]["regex"], "^(GET|HEAD)$")
        self.assertFalse(any("oauth-connect" in json.dumps(r["match"]) for r in routes))

    def test_network_has_no_connector_jwks_or_fga_dependency(self):
        policies = [o for o in self.objects if o["kind"] == "NetworkPolicy"
                    and o["spec"]["podSelector"].get("matchLabels", {}).get("app.kubernetes.io/name") == "oauth-connect"]
        serialized = json.dumps(policies)
        self.assertNotIn('"app.kubernetes.io/name": "iam"', json.dumps([p for p in policies if "Egress" in p["spec"]["policyTypes"]]))
        self.assertNotIn("openfga", serialized)
        self.assertIn("oauth-idp-simulator", serialized)
        vendor = self.obj("NetworkPolicy", "oauth-connect-vendor-https")["spec"]["egress"][0]
        self.assertEqual(vendor["ports"], [{"protocol": "TCP", "port": 443}])
        self.assertIn("10.0.0.0/8", vendor["to"][0]["ipBlock"]["except"])

    def test_https_browser_front_does_not_change_any_upstream_tls_or_historical_profile(self):
        base = renderer.render("oauthopsregression")
        original = next(o for o in yaml.safe_load_all((ROOT / "ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml").read_text())
                        if o and o["kind"] == "ConfigMap")
        shared = yaml.safe_load(original["data"]["envoy.yaml"])
        final = yaml.safe_load(self.obj("ConfigMap", "envoy-mesh-config")["data"]["envoy.yaml"])
        tls = final["static_resources"]["listeners"][0]["filter_chains"][0]["transport_socket"]["typed_config"]
        self.assertFalse(tls["require_client_certificate"])
        self.assertIn("tls_certificates", tls["common_tls_context"])
        self.assertNotIn("validation_context", tls["common_tls_context"])
        legacy = yaml.safe_load(next(o for o in base if o["kind"] == "ConfigMap" and o["metadata"]["name"] == "envoy-mesh-config")["data"]["envoy.yaml"])
        self.assertNotIn("transport_socket", legacy["static_resources"]["listeners"][0]["filter_chains"][0])
        for old, new in zip(shared["static_resources"]["clusters"], final["static_resources"]["clusters"], strict=True):
            expected = yaml.safe_load(yaml.safe_dump(old["transport_socket"]).replace(
                "aiforall-platform.svc", "aiforall-local-full.svc").replace("aiforall-gateway.svc", "aiforall-local-full.svc"))
            self.assertEqual(new["transport_socket"], expected)
        iam = tomllib.loads(next(o for o in self.objects if o["kind"] == "ConfigMap" and o["metadata"]["name"].startswith("iam-dev-config"))["data"]["development.toml"])
        self.assertEqual(iam["jwt"]["public_base_url"], oauth.PUBLIC_ORIGIN)
        for connector in iam["idp"]["connectors"]:
            self.assertTrue(all(uri.startswith(oauth.PUBLIC_ORIGIN + "/") for uri in connector["redirect_uris"]))
        for name in ("hive", "telegraph", "manifesto", "lazaret", "sentinel-sync"):
            env = self.obj("Deployment", name)["spec"]["template"]["spec"]["containers"][0]["env"]
            self.assertEqual([e["value"] for e in env if e["name"].endswith("__PLATFORM_ISSUER")], [oauth.PUBLIC_ORIGIN + "/iam"])


class SecretBoundaryTests(unittest.TestCase):
    def test_seed_validates_every_document_before_any_write(self):
        github, simulated = credentials()
        for mutation in ("shared", "malformed", "existing-conflict"):
            values = oauth.seed_inputs(github, simulated)
            if mutation == "shared":
                values["simulated"]["hmac_secret"] = values["github"]["hmac_secret"]
            elif mutation == "malformed":
                values["simulated"]["client_secret"] = "short"
            bao = Mock()
            def request(method, path, body=None, missing=False):
                if method == "POST":
                    self.fail("a refused seed must not write any KV document")
                if mutation == "existing-conflict" and path.endswith("oauth-connect/simulated/iam"):
                    if "/metadata/" in path:
                        return {"data": {"current_version": 1, "versions": {"1": {}}}}
                    return {"data": {"data": {"hmac_secret": "c" * 64}}}
                return None
            bao.request.side_effect = request
            with self.assertRaises(workload.Refused):
                workload.oauth_seed(bao, values)
            github, simulated = credentials()

    def test_existing_github_secrets_are_reused_not_overwritten(self):
        github, simulated = credentials()
        existing = {"github-connect/oauth": {"client_secret": github["client_secret"]},
                    "github-connect/iam": {"hmac_secret": github["hmac_secret"]}}
        writes = []
        def request(method, path, body=None, missing=False):
            key = path.split("/", 2)[2]
            if method == "POST":
                writes.append(key)
                self.assertEqual(body["options"], {"cas": 0})
                return {}
            if key not in existing:
                return None
            return ({"data": {"current_version": 1, "versions": {"1": {}}}} if "/metadata/" in path
                    else {"data": {"data": existing[key]}})
        workload.oauth_seed(SimpleNamespace(request=request), oauth.seed_inputs(github, simulated))
        self.assertEqual(writes, ["oauth-connect/simulated/oauth", "oauth-connect/simulated/iam"])

    def test_pair_validation_refuses_shared_or_incompatible_simulated_credentials(self):
        for field, value in (("client_secret", "short"), ("client_id", "has.dot"), ("hmac_secret", "a" * 64)):
            github, simulated = credentials()
            simulated[field] = value
            with self.assertRaisesRegex(ValueError, "values redacted"):
                oauth.seed_inputs(github, simulated)

    def test_iam_and_simulator_snapshot_never_publish_partial_secrets(self):
        github, simulated = credentials()
        objects = renderer.render("oauthopsregression", oauth_connect=True)
        template = next(o for o in objects if o["kind"] == "ConfigMap" and o["metadata"]["name"].startswith("iam-dev-config"))["data"]["development.toml"]
        def request(_method, path):
            value = github["hmac_secret"] if "github-connect" in path else simulated["hmac_secret"]
            return {"data": {"data": {"hmac_secret": value}}}
        with patch.object(workload.Path, "read_text", return_value=template), patch.object(workload, "publish") as publish:
            workload.snapshot(SimpleNamespace(request=request), "iam-oauth-hmac")
            result = tomllib.loads(publish.call_args.args[1])["idp"]["connectors"]
            self.assertEqual([c["hmac_secret"] for c in result], [github["hmac_secret"], simulated["hmac_secret"]])
            publish.reset_mock()
            with self.assertRaises(workload.Refused):
                workload.snapshot(SimpleNamespace(request=lambda *_: {"data": {"data": {"hmac_secret": "malformed"}}}), "iam-oauth-hmac")
            publish.assert_not_called()
        with patch.object(workload.Path, "read_text", return_value=json.dumps({"client_id": simulated["client_id"],
                "run_id": "oauthopsregression", "instance": "simulated"})), patch.object(workload, "publish") as publish:
            workload.snapshot(SimpleNamespace(request=lambda *_: {"data": {"data": {"client_secret": simulated["client_secret"]}}}), "oauth-simulator")
            self.assertEqual(publish.call_args.args[0], "provider.json")
            self.assertEqual(json.loads(publish.call_args.args[1])["client_secret"], simulated["client_secret"])

    def test_workload_policies_are_exact_and_separate(self):
        text = (ROOT / "ops/deploy/apps/overlays/local-full/helpers/seed-openbao.sh").read_text()
        policies = dict(re.findall(r"bao policy write ([\w-]+) - >[^\n]* <<'EOF'\n(.*?)\nEOF", text, re.S))
        iam = policies["local-full-iam-oauth-hmac"]
        self.assertNotIn("/oauth\"", iam)
        self.assertIn("github-connect/iam", iam)
        self.assertIn("oauth-connect/simulated/iam", iam)
        simulated = policies["local-full-oauth-simulator"]
        self.assertNotIn("github-connect", simulated)
        self.assertNotIn("/iam\"", simulated)
        for name in ("local-full-iam-oauth-hmac", "local-full-oauth-simulator", "local-full-oauth-connect"):
            self.assertNotIn("*", policies[name])
            self.assertNotIn('"update"', policies[name])

    def test_missing_simulated_recovery_source_refuses_before_mutation(self):
        objects = renderer.render("oauthopsregression", oauth_connect=True)
        target, runtime, save = Mock(), Mock(), Mock()
        target.data = {"ownedResources": [{"kind": "Deployment", "name": "oauth-connect", "namespace": environment.CLUSTER}]}
        reference = {"path": "synthetic-input", "size": 1, "mtime_ns": 1}
        state = {"github_input": reference, "pending_epoch": "next", "lease_path": "synthetic-lease"}
        github, _ = credentials()
        def owned(_target, kind, name, *_):
            obj = copy.deepcopy(next(o for o in objects if o["kind"] == kind and o["metadata"]["name"] == name))
            if kind == "ConfigMap":
                obj["data"]["development.toml"] = obj["data"].get("development.toml", "").replace("offline-synthetic-client", github["client_id"])
            return obj
        with patch.object(environment, "owned", side_effect=owned), patch.object(environment, "input_reference", return_value=reference):
            with self.assertRaisesRegex(ValueError, "private simulated input"):
                environment.recover_coffer(target, state, save, runtime, Mock(return_value=github), Mock())
        runtime.create.assert_not_called()
        target.kubectl.assert_not_called()
        save.assert_not_called()

    def test_new_coffer_epoch_reseeds_both_sources_then_restarts_exact_dependents(self, legacy=False):
        objects = renderer.render("oauthopsregression", oauth_connect=True,
                                  github_client_id="github-public", simulated_client_id="simulated-public")
        target, runtime, save = Mock(), Mock(), Mock()
        target.data = {"ownedResources": [{"kind": "Deployment", "name": "oauth-connect", "namespace": environment.CLUSTER}]}
        if legacy:
            target.data["connectorProfile"] = "oauth-connect"
            target.data["ownedResources"] += [{"kind": "Deployment", "name": name, "namespace": environment.CLUSTER}
                                             for name in ("github-connect", "gitlab-connect")]
        github, simulated = credentials()
        refs = {name: {"path": name, "size": 1, "mtime_ns": 1} for name in ("github-private", "simulated-private")}
        state = {"github_input": refs["github-private"], "simulated_input": refs["simulated-private"],
                 "pending_epoch": "next-epoch", "lease_path": "synthetic-lease"}
        def owned(_target, kind, name, *_):
            obj = copy.deepcopy(next(o for o in objects if o["kind"] == kind and o["metadata"]["name"] == name))
            obj["metadata"].update(uid="owned-" + name, resourceVersion="10")
            return obj
        runtime.create.return_value = {"metadata": {"uid": "seed-job-uid"}}
        reader = lambda path: github if str(path) == "github-private" else simulated
        make_secret = lambda name, values: {"metadata": {"name": name}, "private-test-inputs": values}
        with patch.object(environment, "owned", side_effect=owned), \
                patch.object(environment, "input_reference", side_effect=lambda path: refs[str(path)]), \
                patch.object(environment, "coffer_epoch", return_value="next-epoch"):
            environment.recover_coffer(target, state, save, runtime, reader, make_secret)
        job, private = [call.args[0] for call in runtime.create.call_args_list]
        self.assertEqual(job["spec"]["template"]["spec"]["serviceAccountName"], "oauth-connect-seed")
        self.assertEqual(private["metadata"]["ownerReferences"][0]["uid"], "seed-job-uid")
        payload = json.loads(private["private-test-inputs"]["inputs.json"])
        self.assertEqual(payload, oauth.seed_inputs(github, simulated))
        calls = [call.args[0] for call in target.kubectl.call_args_list]
        self.assertEqual(calls[0][:2], ["wait", "--for=condition=complete"])
        patches = [call for call in calls if call[0] == "patch"]
        self.assertEqual([call[1] for call in patches], ["deployment/" + name for name in
                         ("lazaret", "iam", "oauth-connect", "oauth-idp-simulator")])
        for call in patches:
            patch_ops = json.loads(call[-1])
            self.assertEqual(patch_ops[0]["path"], "/metadata/uid")
            self.assertEqual(patch_ops[1], {"op": "test", "path": "/metadata/resourceVersion", "value": "10"})
        self.assertEqual(state["recovery_seeded_epoch"], "next-epoch")

    def test_selected_oauth_epoch_preserves_legacy_inventory_without_ambiguity(self):
        self.test_new_coffer_epoch_reseeds_both_sources_then_restarts_exact_dependents(legacy=True)

    def test_profile_selection_requires_explicit_marker_and_installed_oauth(self):
        entries = [{"kind": "Deployment", "name": name, "namespace": environment.CLUSTER}
                   for name in ("github-connect", "oauth-connect")]
        with self.assertRaisesRegex(ValueError, "ambiguous"):
            environment.selected_connectors({"ownedResources": entries})
        self.assertEqual(environment.selected_connectors({"connectorProfile": "oauth-connect", "ownedResources": entries}),
                         (False, True))
        with self.assertRaisesRegex(ValueError, "not installed"):
            environment.selected_connectors({"connectorProfile": "oauth-connect", "ownedResources": entries[:1]})

    def test_readiness_scopes_selected_profile_without_adopting_or_dropping_legacy(self):
        entries = [{"kind": "Deployment", "name": name, "namespace": environment.CLUSTER}
                   for name in ("github-connect", "gitlab-connect", "oauth-connect", "iam")]
        for explicit in (False, True):
            target = Mock()
            target.data = {"ownedResources": copy.deepcopy(entries)}
            if explicit:
                target.data["connectorProfile"] = "oauth-connect"
            before = copy.deepcopy(target.data)
            def read(_target, kind, name, namespace):
                return {"spec": {"replicas": 1, "template": {"spec": {"containers": [{"image": "synthetic"}]}}},
                        "status": {"readyReplicas": 0 if name.endswith("-connect") and name != "oauth-connect" else 1}}
            with patch.object(environment, "owned", side_effect=read) as owned, \
                    patch.object(environment, "coffer_epoch", return_value="same"):
                result = environment.running_operation(SimpleNamespace(action="status"), {"epoch": "same"},
                    Mock(), target, None, [{"running": True}], Mock(), Mock(), Mock())
            expected = ["oauth-connect", "iam"] if explicit else [e["name"] for e in entries]
            self.assertEqual([o["name"] for o in result["deployments"]], expected)
            self.assertEqual([c.args[2] for c in owned.call_args_list], expected)
            self.assertEqual(result["ready"], explicit)
            self.assertEqual(target.data, before)
            target.kubectl.assert_not_called()

    def test_seed_partial_failure_never_deletes_or_rewrites_acquired_documents(self):
        github, simulated = credentials()
        values = {}
        def request(method, path, body=None, missing=False):
            key = path.split("/", 2)[2]
            if method == "POST":
                self.assertEqual(body["options"], {"cas": 0})
                if len(values) == 2:
                    raise workload.Refused("synthetic CAS conflict")
                values[key] = copy.deepcopy(body["data"])
                return {}
            self.assertEqual(method, "GET")
            return None
        with self.assertRaises(workload.Refused):
            workload.oauth_seed(SimpleNamespace(request=request), oauth.seed_inputs(github, simulated))
        self.assertEqual(values, {"github-connect/oauth": {"client_secret": github["client_secret"]},
                                  "github-connect/iam": {"hmac_secret": github["hmac_secret"]}})


class SimulatedInstanceTests(unittest.TestCase):
    def test_simulated_slug_refuses_legacy_callback_and_keeps_verified_email_boolean(self):
        provider = simulator.Provider("simulated-client", "synthetic-client-secret", "oauthopsregression", instance="simulated")
        self.assertIn("https://127.0.0.1:18080/iam/api/auth/simulated/callback", provider.redirects)
        self.assertTrue(provider.redirects.isdisjoint(simulator.REDIRECTS))
        with self.assertRaises(simulator.Rejected):
            provider.authorize({"response_type": "code", "client_id": "simulated-client", "state": "state",
                "code_challenge": "a" * 43, "code_challenge_method": "S256", "redirect_uri": next(iter(simulator.REDIRECTS))})
        provider.tokens["synthetic-token"] = provider.clock() + 60
        self.assertIs(provider.profile("Bearer synthetic-token")["email_verified"], True)


if __name__ == "__main__":
    unittest.main()
