"""Bounded offline checks for the deployment boundary, not a live coffer proof."""
import importlib.util
import copy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ops/deploy"))


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


render = load("openbao_renderer", "ops/deploy/render-local-full.py")
workload = load("openbao_workload", "ops/deploy/apps/overlays/local-full/helpers/openbao-workload.py")
provenance = load("openbao_provenance", "ops/deploy/local_full_image_provenance.py")
bootstrap = load("openbao_bootstrap", "ops/deploy/bootstrap_local_kind.py")


class LocalOpenBaoTests(unittest.TestCase):
    def test_full_deploy_refuses_storage_transition_without_writing(self):
        deploy = load("storage_transition_deploy", "ops/deploy/deploy-local-full.py")
        proposed = {"kind": "Deployment", "metadata": {"name": "postgres", "namespace": "aiforall-local-full"},
                    "spec": {"template": {"spec": {"volumes": [
                        {"name": "data", "persistentVolumeClaim": {"claimName": "postgres-data"}}]}}}}
        for source in ({"name": "data", "emptyDir": {}},
                       {"name": "data", "hostPath": {"path": "/data"}},
                       {"name": "data", "persistentVolumeClaim": {"claimName": "another-claim"}}):
            current = copy.deepcopy(proposed)
            current["spec"]["template"]["spec"]["volumes"] = [source]
            with patch.object(deploy, "k", return_value=json.dumps(current)) as command:
                with self.assertRaisesRegex(ValueError, "storage transition refused"):
                    deploy.preflight_storage_transitions([proposed])
                self.assertEqual(command.call_args.args[0], "get")
                self.assertEqual(command.call_count, 1)
        for source in ("", json.dumps(proposed)):
            with patch.object(deploy, "k", return_value=source):
                deploy.preflight_storage_transitions([proposed])

    def test_every_profile_references_pvcs_and_preserves_auxiliary_volumes(self):
        expected = {"postgres": ("data", "postgres-data", {"init"}),
                    "zot": ("data", "zot-data", {"config", "htpasswd"}),
                    "apparatus-operator": ("admission-store", "admission-store", set())}
        for flags in ({}, {"oauth_proof": True}, {"github_openbao": True}):
            objects = render.render("storagevalidation", **flags)
            deployments = {o["metadata"]["name"]: o for o in objects if o["kind"] == "Deployment"}
            for name, (volume_name, claim, auxiliaries) in expected.items():
                volumes = deployments[name]["spec"]["template"]["spec"]["volumes"]
                self.assertIn({"name": volume_name, "persistentVolumeClaim": {"claimName": claim}}, volumes)
                self.assertTrue(auxiliaries <= {v["name"] for v in volumes})
                self.assertFalse(any("hostPath" in v or "emptyDir" in v for v in volumes))

    def test_storage_guard_rejects_ignored_patch_and_missing_auxiliary(self):
        original = render.render("storagevalidation")
        for name in ("postgres", "zot", "apparatus-operator"):
            objects = copy.deepcopy(original)
            deployment = next(o for o in objects if o["kind"] == "Deployment" and o["metadata"]["name"] == name)
            volumes = deployment["spec"]["template"]["spec"]["volumes"]
            volume = next(v for v in volumes if "persistentVolumeClaim" in v)
            volume.pop("persistentVolumeClaim")
            volume["emptyDir"] = {}
            with self.assertRaisesRegex(ValueError, "persistence patch missing"):
                render.validate_persistent_storage(objects)
        objects = copy.deepcopy(original)
        pod = next(o for o in objects if o["kind"] == "Deployment" and o["metadata"]["name"] == "postgres")["spec"]["template"]["spec"]
        pod["volumes"] = [v for v in pod["volumes"] if v["name"] != "init"]
        for c in pod["containers"]:
            c["volumeMounts"] = [v for v in c["volumeMounts"] if v["name"] != "init"]
        with self.assertRaisesRegex(ValueError, "required auxiliary volume"):
            render.validate_persistent_storage(objects)

    def test_operator_pvc_does_not_inherit_legacy_root_chown(self):
        for flags in ({}, {"oauth_proof": True}, {"github_openbao": True}):
            with self.subTest(profile=flags):
                objects = render.render("operatornonroot", **flags)
                pod = next(o for o in objects if o["kind"] == "Deployment"
                           and o["metadata"]["name"] == "apparatus-operator")["spec"]["template"]["spec"]
                self.assertFalse(pod.get("initContainers"))
                self.assertTrue(pod["securityContext"]["runAsNonRoot"])
                self.assertEqual(pod["securityContext"]["fsGroup"], 65534)
                for container in pod["containers"]:
                    security = container["securityContext"]
                    self.assertNotEqual(security.get("runAsUser"), 0)
                    self.assertFalse(security["allowPrivilegeEscalation"])
                    self.assertFalse(security["capabilities"].get("add"))

    def test_profiles_preserve_gitlab_and_bound_github_identity(self):
        for flags in ({}, {"oauth_proof": True}, {"github_openbao": True}):
            objects = render.render("offlinevalidation", **flags)
            serialized = json.dumps(objects)
            self.assertNotIn("local-full-dev-root", serialized)
            for obj in objects:
                if obj["kind"] == "NetworkPolicy":
                    for direction in ("ingress", "egress"):
                        for rule in obj["spec"].get(direction, []):
                            self.assertNotIn(8200, [p["port"] for p in rule.get("ports", [])])
            deployments = {o["metadata"]["name"]: o for o in objects if o["kind"] == "Deployment"}
            lazaret = deployments["lazaret"]["spec"]["template"]["spec"]
            self.assertEqual(lazaret["serviceAccountName"], "lazaret")
            self.assertEqual(lazaret["initContainers"][0]["command"][-1], "lazaret")
            operator = deployments["apparatus-operator"]["spec"]["template"]["spec"]["containers"][0]
            env = {item["name"]: item.get("value") for item in operator["env"]}
            self.assertEqual(env["APPARATUS_COSIGN_BIN"], "/usr/local/bin/cosign")
            self.assertEqual(env["APPARATUS_VAULT_CA_CERT"], "/run/openbao-ca/ca.crt")
            if flags.get("github_openbao"):
                pod = deployments["github-connect"]["spec"]["template"]["spec"]
                self.assertEqual(pod["initContainers"][0]["restartPolicy"], "Always")
                self.assertFalse(pod["automountServiceAccountToken"])
                app = pod["containers"][0]
                self.assertNotIn("openbao-identity", [m["name"] for m in app["volumeMounts"]])
                sink = next(m for m in app["volumeMounts"] if m["name"] == "github-token")
                self.assertTrue(sink["readOnly"])
                self.assertNotIn("subPath", sink)
                iam = deployments["iam"]["spec"]["template"]["spec"]
                trust = next(c for c in iam["initContainers"] if c["name"] == "trusted-roots")
                self.assertIn("/etc/ssl/certs/ca-certificates.crt /run/openbao-ca/ca.crt", trust["command"][-1])
                policies = {o["metadata"]["name"]: o["spec"] for o in objects if o["kind"] == "NetworkPolicy"}
                self.assertEqual(policies["allow-github-jwks"]["egress"][0]["ports"][0]["port"], 8080)
                self.assertEqual(policies["allow-iam-jwks"]["ingress"][0]["ports"][0]["port"], 8080)
            if flags.get("oauth_proof"):
                self.assertIn("gitlab-connect", deployments)
        with self.assertRaises(ValueError):
            render.render("offlinevalidation", oauth_proof=True, github_openbao=True)

    def test_missing_immutable_github_image_is_refused(self):
        objects = render.render("offlinevalidation", github_openbao=True)
        images = {c["image"]: "local/image:sha256-" + "a" * 64
                  for o in objects if o["kind"] == "Deployment"
                  for c in o["spec"]["template"]["spec"].get("containers", []) + o["spec"]["template"]["spec"].get("initContainers", [])
                  if c["image"].startswith("aiforall-")}
        del images["aiforall-github-connect-service:latest"]
        with self.assertRaises(ValueError):
            render.render("offlinevalidation", images, github_openbao=True)

    def test_stale_image_source_or_sdk_is_refused_without_running_docker(self):
        source = "aiforall-github-connect-service:latest"
        snapshot = {"files": [{"path": "services/GitHubConnect/src/main.rs", "sha256": "a" * 64, "mode": 420},
                              {"path": "rustycog/src/lib.rs", "sha256": "b" * 64, "mode": 420}]}
        labels = {"org.opencontainers.image.revision": "a" * 40, "aiforall.dev.source-policy": "local",
                  "aiforall.dev.freshness-policy": provenance.FRESHNESS_POLICY,
                  "aiforall.dev.cargo-profile": "dev-image", "aiforall.dev.build-targets": "github",
                  "aiforall.dev.build-context": provenance.build_context(["github"], "dev-image"),
                  "aiforall.dev.source-input": provenance.target_identity(snapshot, "github")}
        for mismatch in ("none", "source", "sdk"):
            current = json.loads(json.dumps(snapshot))
            if mismatch != "none":
                current["files"][int(mismatch == "sdk")]["sha256"] = "c" * 64
            def command(args, **kwargs):
                self.assertEqual(args[0], "docker")
                return SimpleNamespace(returncode=0, stdout=json.dumps(labels))
            with patch.object(provenance, "capture", return_value=current), patch.object(provenance.subprocess, "run", side_effect=command):
                if mismatch == "none":
                    provenance.verify({source: "local/image:sha256-" + "c" * 64})
                else:
                    with self.assertRaises(ValueError):
                        provenance.verify({source: "local/image:sha256-" + "c" * 64})

    def test_portable_bootstrap_retains_cni_pool_and_all_ready_gates(self):
        calls = []
        def kubectl(args, **kwargs):
            calls.append(args)
            if "daemonsets" in args:
                return json.dumps({"items": [{"metadata": {"name": "calico-node"}, "spec": {"template": {"spec": {
                    "containers": [{"image": "calico/node:v3.29.7"}]}}}}]})
            if "jsonpath={.items[*].spec.podCIDR}" in args:
                return "10.244.0.0/24 10.244.1.0/24 10.244.2.0/24"
            if "ippools" in args:
                return json.dumps({"items": [{"spec": {"cidr": "10.244.0.0/16"}}]})
            return ""
        with patch.object(bootstrap.authority, "bootstrap") as create, \
                patch.object(bootstrap.authority, "LocalClusterTargetAnchor") as anchor:
            anchor.return_value.kubectl.side_effect = kubectl
            bootstrap.initialize(Path("private-lease"), "aiforall-local-full", Path("cluster-local-full.yaml"), True)
            create.assert_called_once()
            self.assertEqual(anchor.call_args.kwargs["expected_cluster"], "aiforall-local-full")
        self.assertEqual(sum("ippools" in args for args in calls), 2)
        self.assertEqual([args[4] for args in calls if "rollout" in args],
                         ["daemonset/calico-node", "deployment/calico-kube-controllers", "deployment/coredns"])
        self.assertTrue(any("--for=condition=Ready" in args and "nodes" in args for args in calls))

    def test_seed_is_create_only_idempotent_and_conflict_preserving(self):
        inputs = {"version": 1, "client_id": "synthetic", "client_secret": "public-test-sentinel", "hmac_secret": "a" * 64}

        class FakeBao:
            def __init__(self):
                self.values, self.writes = {}, 0
                self.deleted = False

            def request(self, method, path, body=None, missing=False):
                key = path.removeprefix("secret/metadata/").removeprefix("secret/data/")
                if method == "POST":
                    if key in self.values or body["options"] != {"cas": 0}:
                        raise workload.Refused("CAS conflict")
                    self.values[key] = body["data"]
                    self.writes += 1
                    return {}
                if path.startswith("secret/metadata/"):
                    return {"data": {"current_version": 1, "versions": {"1": {"destroyed": self.deleted}}}} if key in self.values else None
                return {"data": {"data": self.values[key]}}

        bao = FakeBao()
        workload.seed(bao, inputs)
        workload.seed(bao, inputs)
        self.assertEqual(bao.writes, 2)
        bao.deleted = True
        with self.assertRaises(workload.Refused):
            workload.seed(bao, inputs)
        self.assertEqual(bao.writes, 2)
        bao.deleted = False
        before = json.dumps(bao.values, sort_keys=True)
        with self.assertRaises(workload.Refused):
            workload.seed(bao, {**inputs, "hmac_secret": "b" * 64})
        self.assertEqual(json.dumps(bao.values, sort_keys=True), before)
        self.assertEqual(bao.writes, 2)

    def test_certificate_extension_preserves_every_old_byte_and_rejects_partial(self):
        # Owned synthetic PKI only; never reads the operator's certificate directory.
        base = Path(os.environ.get("LOCALAPPDATA", str(Path.home() / ".cache"))) / "Temp/opencode" if os.name == "nt" else Path(tempfile.gettempdir())
        base.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="openbao-pki-smoke-", dir=base) as parent:
            directory = Path(parent) / "certificates"
            command = [sys.executable, "-B", str(ROOT / "ops/scripts/prepare-local-full-certs.py"), "--output-dir", str(directory), "--oauth-proof"]

            def invoke(extra=(), expected=0):
                result = subprocess.run(command + list(extra), capture_output=True, timeout=180)
                diagnostic = next((line.decode("utf-8") for line in result.stderr.splitlines()
                                   if line.startswith(b"Certificate preparation failed:")), "private tool output not reproduced")
                self.assertEqual(result.returncode, expected, "PKI smoke failed: " + diagnostic)

            def contents():
                return {p.relative_to(directory): p.read_bytes()
                         for p in directory.rglob("*") if p.is_file()}

            invoke()
            before = contents()
            invoke(["--openbao-github", "--registry"])
            extended = contents()
            self.assertTrue(all(extended[p] == data for p, data in before.items()), "old PKI bytes changed")
            invoke(["--openbao-github", "--registry"])
            self.assertTrue(extended == contents(), "idempotent PKI extension changed bytes")
            leaf = directory / "extensions/openbao-github/openbao.key"
            held = leaf.with_suffix(".held")
            leaf.rename(held)
            partial = contents()
            invoke(["--openbao-github"], expected=1)
            # A private diagnostic may be added; all pre-existing bytes are preserved.
            after = contents()
            self.assertTrue(all(after[p] == data for p, data in partial.items()), "partial extension changed old bytes")
            held.rename(leaf)
            leaf = directory / "extensions/registry/registry-reader.key"
            held = leaf.with_suffix(".held")
            leaf.rename(held)
            partial = contents()
            invoke(["--registry"], expected=1)
            after = contents()
            self.assertTrue(all(after[p].startswith(data) if p.as_posix() == "openssl-error.log" else after[p] == data
                                for p, data in partial.items()), "partial registry extension changed old PKI bytes or diagnostic prefix")
            held.rename(leaf)


if __name__ == "__main__":
    unittest.main()
