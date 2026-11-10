"""Hermetic Helm parity/phase/lifecycle oracles. Never contacts a daemon/API."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest
from types import SimpleNamespace
from unittest.mock import Mock, patch

import yaml

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ops/deploy"))
import local_full_helm as helm
import local_full_lifecycle as lifecycle
import local_full_phases as phases
import local_full_environment as environment


def normalized(objects, profile):
    """Explicit migration dispositions, not a blanket spec/secret scrubber."""
    objects = copy.deepcopy(objects)
    if profile == "gitlab-proof":
        import base64
        templates = [o for o in objects if o["kind"] == "Secret" and o["metadata"]["name"].startswith("oauth-proof-iam-config-")]
        if templates and templates[0].get("data", {}).get("development.toml"):
            text = base64.b64decode(templates[0]["data"]["development.toml"]).decode()
            next(o for o in objects if o["kind"] == "ConfigMap" and o["metadata"]["name"].startswith("iam-dev-config-"))["data"]["development.toml"] = text
    result = {}
    template_sources = {(o["metadata"].get("namespace"), o["metadata"]["name"]): o["metadata"]["annotations"]["aiforall.dev/template-configmap"]
                        for o in objects if o["kind"] == "Secret" and o["metadata"].get("annotations", {}).get("aiforall.dev/private-envelope") == "runtime-template"}
    for obj in objects:
        if obj["kind"] == "Secret" and obj["metadata"].get("annotations", {}).get("aiforall.dev/private-envelope") == "runtime-template":
            continue  # Explicit confidential configuration-envelope migration.
        if obj["kind"] == "Secret" and (obj["metadata"]["name"] == "iam-local-signing-key" or obj["metadata"]["name"].startswith("oauth-proof-inputs-")):
            continue  # New explicit confidential envelopes, asserted separately.
        obj["metadata"].pop("annotations", None)
        if obj["kind"] == "Secret":
            obj.pop("data", None)
            obj.pop("stringData", None)
        if obj["kind"] == "Deployment":
            template = obj["spec"]["template"]
            template["metadata"].pop("annotations", None)
            pod = template["spec"]
            pod["volumes"] = [v for v in pod.get("volumes", []) if v["name"] != "signing-key"]
            for volume in pod["volumes"]:
                reference = volume.get("secret", {})
                source = template_sources.get((obj["metadata"]["namespace"], reference.get("secretName")))
                if source:
                    volume.pop("secret")
                    volume["configMap"] = {"name": source}
                if reference.get("secretName") in {"openbao-server-tls", "github-connect-server-tls", "oauth-connect-server-tls"}:
                    reference["secretName"] += "-staticvalidation"
            for container in pod["containers"]:
                for variable in container.get("env", []):
                    reference = variable.get("valueFrom", {}).get("secretKeyRef", {})
                    if reference.get("key") == "grant-hmac" and (obj["metadata"]["namespace"], reference.get("name")) in template_sources:
                        variable.pop("valueFrom")
                        variable["value"] = "__LOCAL_GRANT_HMAC__"
                    if variable.get("value") == "manifesto-grant-snapshot-dev-hs256":
                        variable["value"] = "__LOCAL_GRANT_HMAC__"
                for mount in container.get("volumeMounts", []):
                    if mount.get("subPath") == "test-platform.pem" and mount["name"] == "signing-key":
                        mount["name"] = "config"
                        mount.pop("readOnly", None)
                    if obj["metadata"]["name"] in {"iam", "manifesto"} and mount["name"] == "config":
                        mount.pop("readOnly", None)  # Explicitly tightened; independently asserted below.
        if obj["kind"] == "ConfigMap":
            data = obj.get("data", {})
            data.pop("test-platform.pem", None)
            # Permission setup now consists of standard public documents. Its
            # semantic parity is asserted by test_permission_documents below.
            data.pop("seed-openbao.sh", None)
            for key in list(data):
                if key.startswith("openbao-") and key != "openbao-workload.py":
                    data.pop(key)
            for key, value in data.items():
                if key.endswith(".toml"):
                    data[key] = tomllib.loads(value)
                    def public_fixture_markers(value):
                        if isinstance(value, dict):
                            for field, item in value.items():
                                if field == "oauth_state_secret" and item == "b9a3475cd01f826ee684f2703acb15d96c02e85147bd6a3905f8a63e12d9c407":
                                    value[field] = "__LOCAL_OAUTH_STATE__"
                                elif field == "hs256_secret" and item == "manifesto-grant-snapshot-dev-hs256":
                                    value[field] = "__LOCAL_GRANT_HMAC__"
                                else:
                                    public_fixture_markers(item)
                        elif isinstance(value, list):
                            for item in value:
                                public_fixture_markers(item)
                    public_fixture_markers(data[key])
                elif key.endswith(".json"):
                    data[key] = json.loads(value)
                elif key.endswith(".yaml"):
                    data[key] = yaml.safe_load(value)
                else:
                    data[key] = value.replace("\r\n", "\n").strip()
        key = "/".join((obj["kind"], obj["metadata"].get("namespace", "cluster"), obj["metadata"]["name"]))
        result[key] = obj
    return result


class DeclarativeLocalFull(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.helm = helm.executable()  # An absent/wrong tool is NOT a simulated PASS.

    def test_all_profiles_and_phases_standard_lint_and_template(self):
        for profile in sorted(helm.PROFILES_ALLOWED):
            final = helm.render(profile)
            categories = {category: {o["metadata"]["name"] for o in final if helm.category(o) == category}
                          for category in ("foundation", "dependencies", "bootstrap", "applications")}
            for phase in helm.PHASES:
                with self.subTest(profile=profile, phase=phase):
                    lint = subprocess.run([self.helm, "lint", str(helm.CHART), "-f", str(helm.PROFILES / (profile + ".yaml")),
                        "-f", str(helm.PROFILES / "phases" / (phase + ".yaml"))], capture_output=True, text=True, timeout=60)
                    self.assertEqual(lint.returncode, 0, lint.stderr[:200])
                    objects = helm.render(profile, phase)
                    expected = set().union(*categories.values()) if phase == "final" else categories["applications" if phase in {"quiesce", "applications-first"} else phase]
                    self.assertEqual({o["metadata"]["name"] for o in objects}, expected)
                    if phase in {"quiesce", "applications-first"}:
                        self.assertTrue(all(o["kind"] == "Deployment" and o["spec"]["replicas"] == (0 if phase == "quiesce" else 1) for o in objects))
                        for obj in objects:
                            final_obj = next(o for o in final if o["kind"] == "Deployment" and o["metadata"]["name"] == obj["metadata"]["name"])
                            self.assertEqual(obj["spec"]["template"], final_obj["spec"]["template"], "scale-out must NOT create another PodTemplate/migration rollout")

    def test_complete_synthetic_baseline_parity(self):
        golden = json.loads((ROOT / "ops/tests/fixtures/local-full-5199613.json").read_text(encoding="utf-8"))
        for profile in sorted(helm.PROFILES_ALLOWED):
            with self.subTest(profile=profile):
                actual = helm.render(profile)
                self.assertEqual(normalized(actual, profile), golden[profile])
                signing = next(o for o in actual if o["kind"] == "Secret" and o["metadata"]["name"] == "iam-local-signing-key")
                self.assertNotIn("data", signing)
                self.assertNotIn("PRIVATE KEY", json.dumps(actual))
                self.assertNotIn("manifesto-grant-snapshot-dev-hs256", json.dumps(actual))
                self.assertNotIn("b9a3475cd01f826ee684f2703acb15d96c02e85147bd6a3905f8a63e12d9c407", json.dumps(actual))
                # Assert the tightening explicitly before parity normalization.
                for name in ("iam", "manifesto"):
                    pod = next(o for o in actual if o["kind"] == "Deployment" and o["metadata"]["name"] == name)["spec"]["template"]["spec"]
                    mounts = [m for c in pod["containers"] + pod.get("initContainers", []) for m in c.get("volumeMounts", [])]
                    config = [m for m in mounts if m["name"] == "config"]
                    self.assertTrue(config, name)
                    self.assertTrue(all(m.get("readOnly") is True for m in config), name)
                    if name == "iam":
                        signing = [m for m in mounts if m.get("subPath") == "test-platform.pem"]
                        self.assertTrue(signing)
                        self.assertTrue(all(m["name"] == "signing-key" and m.get("readOnly") is True for m in signing))
                        volume = next(v for v in pod["volumes"] if v["name"] == "signing-key")
                        self.assertEqual(volume["secret"]["secretName"], "iam-local-signing-key")
                self.assertFalse(any("test-platform.pem" in o.get("data", {}) for o in actual if o["kind"] == "ConfigMap"))

    def test_schema_and_ambient_inputs_refused(self):
        for values in ({"runId": "bad"}, {"githubClientId": 'injection"'}, {"images": {"image": "latest"}},
                       {"network": {"apiCidr": "0.0.0.0/0", "ingressCidr": ""}},
                       {"network": {"apiCidr": "999.1.1.1/32", "ingressCidr": ""}},
                       {"network": {"apiCidr": "", "ingressCidr": "0.0.0.0/0"}},
                       {"network": {"apiCidr": "", "ingressCidr": "172.19.0.0/16"}}):
            with self.subTest(values=values), self.assertRaises(RuntimeError):
                helm.render(values=values)
        with self.assertRaises(ValueError):
            helm.render(values={"client_secret": "SENTINEL_NOT_FOR_HELM"})
        with patch.dict(os.environ, KUBECONFIG="SENTINEL_AMBIENT", HELM_KUBECONTEXT="SENTINEL_AMBIENT"):
            self.assertTrue(helm.render())

    def test_declared_permission_documents(self):
        source = json.loads((helm.CHART / "files/openbao/bootstrap.json").read_text())
        self.assertEqual(source, json.loads((ROOT / "ops/tests/fixtures/local-full-openbao-5199613.json").read_text()))
        objects = helm.render("oauth-connect")
        data = next(o["data"] for o in objects if o["kind"] == "ConfigMap" and "seed-openbao.sh" in o.get("data", {}))
        for name, role in source["roles"].items():
            self.assertEqual(json.loads(data["openbao-role-" + name + ".json"]), role)
        for name, policy in source["policies"].items():
            self.assertEqual(data["openbao-policy-" + name + ".hcl"], policy)
        self.assertNotIn("bound_service_account_names=", data["seed-openbao.sh"])
        self.assertNotIn("<<'EOF'", data["seed-openbao.sh"])
        self.assertEqual(source["roles"]["oauth-connect"]["audience"], "openbao-oauth-connect")
        self.assertIn('path "secret/data/github-connect/oauth"', source["policies"]["local-full-oauth-connect"])
        self.assertNotIn('capabilities = ["read", "create", "update"]', source["policies"]["local-full-oauth-connect"])

    def test_workload_added_only_to_declarations_gets_render_wait_and_epoch(self):
        with tempfile.TemporaryDirectory() as directory:
            chart = Path(directory) / "chart"
            shutil.copytree(helm.CHART, chart)
            values = yaml.safe_load((chart / "values.yaml").read_text())
            descriptor = copy.deepcopy(next(v for v in values["resources"].values() if v.get("epoch")))
            descriptor["object"]["metadata"]["name"] = "synthetic-consumer"
            descriptor["object"]["spec"]["selector"]["matchLabels"] = {"app.kubernetes.io/name": "synthetic-consumer"}
            descriptor["object"]["spec"]["template"]["metadata"]["labels"] = {"app.kubernetes.io/name": "synthetic-consumer"}
            if descriptor.get("migrationScope"):
                descriptor["migrationScope"] = "synthetic_database"
            values["resources"]["synthetic-consumer"] = descriptor
            (chart / "values.yaml").write_text(yaml.safe_dump(values), encoding="utf-8")
            original = helm.render
            render = lambda profile="base", phase="final", **kw: original(profile, phase, chart=chart, **kw)
            for phase, replicas in (("quiesce", 0), ("applications-first", 1), ("final", 1)):
                obj = next(o for o in render(phase=phase) if o["metadata"]["name"] == "synthetic-consumer")
                self.assertEqual(obj["spec"]["replicas"], replicas)
            api = SimpleNamespace(k=Mock())
            phases.wait_workloads(api, render())
            self.assertTrue(any("deployment/synthetic-consumer" in call.args for call in api.k.call_args_list))
            target = SimpleNamespace(data={"connectorProfile": "base"}, kubectl=Mock())
            lifecycle._objects.cache_clear()
            self.addCleanup(lifecycle._objects.cache_clear)
            def owned(_target, _kind, name, namespace=environment.CLUSTER):
                obj = copy.deepcopy(next(o for o in render() if o["kind"] == "Deployment" and o["metadata"]["name"] == name))
                obj["metadata"].update(uid="owned-" + name, resourceVersion="1")
                return obj
            with patch.object(helm, "render", side_effect=render), patch.object(environment, "owned", side_effect=owned), patch.object(environment, "coffer_epoch", return_value="epoch"):
                environment.recover_coffer(target, {"pending_epoch": "epoch"}, Mock(), Mock(), Mock(), Mock())
            self.assertTrue(any(call.args[0][:2] == ["patch", "deployment/synthetic-consumer"] for call in target.kubectl.call_args_list))

            # The declared format defines migrationScope exclusivity. Two
            # writers sharing it must fail in Helm, before any phase mutator.
            descriptor["migrationScope"] = "manifesto_dev"
            (chart / "values.yaml").write_text(yaml.safe_dump(values), encoding="utf-8")
            with self.assertRaises(RuntimeError):
                original("base", chart=chart)

    def test_storage_templates_are_disjoint_and_bounded(self):
        for readonly in (True, False):
            pod = helm.storage("volume", "aiforall-local-full", "synthetic-storage", "synthetic-claim", read_only=readonly)[0]
            self.assertEqual(pod["spec"]["volumes"][0]["persistentVolumeClaim"]["readOnly"], readonly)
            self.assertEqual("fsGroup" in pod["spec"]["securityContext"], not readonly)
        pod = helm.storage("postgres", "aiforall-local-restore-synthetic", "synthetic-restore", "synthetic-claim", database="target_db")[0]
        self.assertEqual(pod["spec"]["securityContext"]["runAsUser"], 70)
        self.assertNotIn("command", pod["spec"]["containers"][0])
        with self.assertRaises(RuntimeError):
            helm.storage("volume", "production", "synthetic", "claim")

    def test_just_graph_orders_every_phase_and_predecessors(self):
        just = shutil.which("just")
        self.assertIsNotNone(just)
        result = subprocess.run([just, "--justfile", str(ROOT / "ops/deploy/local-full.just"), "--dry-run", "run"],
                                capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr[:200])
        commands = result.stdout + result.stderr
        positions = [commands.index("_phase " + phase + " ") for phase in ("plan", "target", "quiesce", "foundation", "dependencies", "bootstrap", "applications-first", "final")]
        self.assertEqual(positions, sorted(positions))
        self.assertEqual(phases.graph_predecessor("final"), "applications-first")
        self.assertEqual(phases.graph_predecessor("plan"), "none")


if __name__ == "__main__":
    unittest.main()
