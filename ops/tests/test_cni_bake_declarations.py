"""Offline standard-tool oracles; no Docker daemon, Kubernetes API or Cargo."""
import copy
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec_path = ROOT / "ops/deploy/build/targets.json"
INVENTORY = json.loads(spec_path.read_text())["variable"]["inventory"]["default"]
TARGETS = INVENTORY["targets"]
sys.path.insert(0, str(ROOT / "ops/deploy"))
import bootstrap_local_kind as bootstrap


class DeclarativeToolsTests(unittest.TestCase):
    def tool(self, name):
        executable = shutil.which(name)
        if not executable:
            self.skipTest(name + " client unavailable (NOT RUN)")
        return executable

    def bake_print(self, root=ROOT, extra=None, targets=("workspace",), settings=()):
        docker = self.tool("docker")
        env = {k: v for k, v in os.environ.items() if k not in {
            "inventory", "IMAGE_TARGETS", "CARGO_PROFILE", "SOURCE_REVISION", "SDK_REVISION", "ARTIFACTS_REF"}}
        # Prove --print has no usable daemon or ambient kubeconfig.
        env.update(DOCKER_HOST="tcp://127.0.0.1:1", KUBECONFIG="nonexistent-offline-sentinel")
        args = [docker, "buildx", "bake", "-f", "ops/deploy/build/targets.json",
                "-f", "ops/deploy/build/docker-bake.hcl"]
        if extra:
            args += ["-f", str(extra)]
        for setting in settings:
            args += ["--set", setting]
        result = subprocess.run([*args, "--print", *targets], cwd=root, env=env,
                                capture_output=True, text=True, timeout=60)
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def test_bake_resolves_catalogue_and_shared_artifact_graph(self):
        value = self.bake_print()
        for name, item in TARGETS.items():
            with self.subTest(target=name):
                resolved = value["target"][name]
                self.assertEqual(resolved["dockerfile"], item["dockerfile"])
                self.assertEqual(resolved["tags"], [item["source"]])
                self.assertEqual(resolved["contexts"], {"build-artifacts": "target:artifacts"})
                self.assertEqual(resolved["platforms"], ["linux/amd64"])
                self.assertEqual(resolved["args"]["BUILD_IMAGE"], "build-artifacts")
        self.assertEqual(value["target"]["artifacts"]["dockerfile"], "Dockerfile.build")

    def test_direct_groups_and_targets_share_image_superset_not_monolith(self):
        selections = {"default": INVENTORY["groups"]["local-full"], **INVENTORY["groups"],
                      **{name: [name] for name in TARGETS}}
        superset = INVENTORY["groups"]["workspace"]
        self.assertEqual(set(superset), set(TARGETS))
        outputs = {b for name in superset for b in TARGETS[name]["outputs"].values()}
        features = {f for name in superset for f in TARGETS[name]["features"]}
        self.assertNotIn("oodhive-monolith", outputs)
        self.assert_native_selection(superset, outputs, features)
        for entry, expected in selections.items():
            with self.subTest(entry=entry):
                value = self.bake_print(targets=(entry,))
                producers = {k: v for k, v in value["target"].items() if k == "artifacts"}
                self.assertEqual(len(producers), 1)
                producer_name, producer = next(iter(producers.items()))
                compiled = producer["args"]["IMAGE_TARGETS"].split(",")
                self.assertEqual(compiled, superset)
                self.assertEqual(producer["args"]["DELIVERY_PROFILE"], "images")
                consumers = {k: v for k, v in value["target"].items() if k != producer_name}
                self.assertEqual(set(consumers), set(expected))
                for consumer in consumers.values():
                    self.assertEqual(consumer["contexts"], {"build-artifacts": "target:" + producer_name})
                # Only linked outputs are intentionally cache-only in direct group plans.
                self.assertTrue(all(x["type"] == "cacheonly" for x in producer["output"]))

    def test_canonical_overrides_survive_groups_and_overlapping_selections(self):
        combinations = [(name,) for name in ("default", "local-full", "workspace", "openbao")]
        combinations += [("workspace", "github"), ("local-full", "openbao")]
        for entries in combinations:
            expected = set()
            for entry in entries:
                expected.update(INVENTORY["groups"].get(
                    "local-full" if entry == "default" else entry, [entry]))
            name = "github" if "github" in expected else "iam"
            settings = (name + ".tags=review-" + name + ":custom",
                        name + ".labels.review.fixture=public-label",
                        name + ".args.REVIEW_FIXTURE=public-arg")
            with self.subTest(entries=entries):
                isolated = self.bake_print(targets=(name,), settings=settings)
                grouped = self.bake_print(targets=entries, settings=settings)
                self.assertEqual(set(grouped["target"]), expected | {"artifacts"})
                target = grouped["target"][name]
                for property in ("tags", "labels", "args"):
                    self.assertEqual(target[property], isolated["target"][name][property])
                self.assertEqual(target["tags"], ["review-" + name + ":custom"])
                self.assertEqual(target["labels"]["review.fixture"], "public-label")
                self.assertEqual(target["args"]["REVIEW_FIXTURE"], "public-arg")
                tags = [tag for key, value in grouped["target"].items() if key != "artifacts" for tag in value["tags"]]
                self.assertEqual(len(tags), len(set(tags)), "one writer per exported tag")
                for consumer in expected:
                    self.assertEqual(grouped["target"][consumer]["contexts"], {"build-artifacts": "target:artifacts"})

    def assert_native_selection(self, names, outputs, features):
        """Resolve the native Cargo arguments through Bake without a daemon."""
        with tempfile.TemporaryDirectory() as tmp:
            override = Path(tmp) / "compile.json"
            override.write_text(json.dumps({"variable": {"IMAGE_TARGETS": {"default": ",".join(names)}}}))
            value = self.bake_print(extra=override, targets=tuple(names))
            argv = shlex.split(value["target"]["artifacts"]["args"]["CARGO_BUILD_ARGS"])
            binaries = {argv[i + 1] for i, value in enumerate(argv) if value == "--bin"}
            self.assertEqual(binaries, outputs)
            actual_features = set(argv[argv.index("--features") + 1].split(",")) if "--features" in argv else set()
            self.assertEqual(actual_features, features)

    def test_direct_explicit_restriction_is_resolved_by_bake(self):
        with tempfile.TemporaryDirectory() as tmp:
            override = Path(tmp) / "invocation.json"
            override.write_text(json.dumps({"variable": {"IMAGE_TARGETS": {"default": "github"}}}))
            value = self.bake_print(extra=override, targets=("github",))
            self.assertEqual(set(value["target"]), {"github", "artifacts"})
            self.assertEqual(value["target"]["artifacts"]["args"]["IMAGE_TARGETS"], "github")
            self.assert_native_selection(["github"], set(TARGETS["github"]["outputs"].values()), set())

    def test_new_target_and_invocation_provenance_need_no_python_recipe(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            directory = root / "ops/deploy/build"
            directory.mkdir(parents=True)
            shutil.copy2(ROOT / "ops/deploy/build/docker-bake.hcl", directory)
            inventory = json.loads((ROOT / "ops/deploy/build/targets.json").read_text())
            data = inventory["variable"]["inventory"]["default"]
            data["targets"]["synthetic"] = copy.deepcopy(data["targets"]["hive"])
            data["targets"]["synthetic"]["outputs"] = {"synthetic-package": "synthetic-bin"}
            data["targets"]["synthetic"]["features"] = ["synthetic-package/feature"]
            data["groups"]["local-full"].append("synthetic")
            data["groups"]["workspace"].append("synthetic")
            (directory / "targets.json").write_text(json.dumps(inventory))
            direct = self.bake_print(root, targets=("synthetic",))
            self.assertEqual(direct["target"]["artifacts"]["args"]["IMAGE_TARGETS"].split(","), data["groups"]["workspace"])
            grouped = self.bake_print(root, targets=("local-full",))
            self.assertIn("synthetic", grouped["target"])
            override = root / "invocation.json"
            override.write_text(json.dumps({"variable": {"IMAGE_TARGETS": {"default": "synthetic"}},
                "target": {"synthetic": {"tags": ["synthetic:inputs-fixture"],
                                         "labels": {"aiforall.dev.sdk-revision": "d" * 40}}}}))
            value = self.bake_print(root, override, ("synthetic",))
            self.assertEqual(value["target"]["synthetic"]["tags"], ["synthetic:inputs-fixture"])
            self.assertEqual(value["target"]["synthetic"]["labels"]["aiforall.dev.sdk-revision"], "d" * 40)
            self.assertEqual(value["target"]["artifacts"]["args"]["IMAGE_TARGETS"], "synthetic")

    def test_calico_real_kustomize_exact_pin_cidr_and_felix(self):
        self.tool("kubectl")
        import yaml
        with patch.dict(os.environ, {"KUBECONFIG": "nonexistent-offline-sentinel"}):
            rendered = list(yaml.safe_load_all(bootstrap.calico_manifest()))
        node = next(x for x in rendered if x.get("kind") == "DaemonSet" and x["metadata"]["name"] == "calico-node")
        pod = node["spec"]["template"]["spec"]
        container = next(x for x in pod["containers"] if x["name"] == "calico-node")
        env = {x["name"]: x.get("value") for x in container["env"]}
        self.assertEqual(env["CALICO_IPV4POOL_CIDR"], "10.244.0.0/16")
        self.assertEqual(env["FELIX_IGNORELOOSERPF"], "true")
        for item in [*pod["containers"], *pod["initContainers"]]:
            self.assertTrue(item["image"].endswith(":v3.29.7"))
        base = ROOT / "ops/deploy/third-party/calico/base/calico.yaml"
        self.assertEqual(hashlib.sha256(base.read_bytes()).hexdigest(),
                         "b200cb3c2c7341717abe98e4acc80a13117bcd2c9c46ad46e8c209c5753f0b2f")

    def test_calico_checksum_refuses_before_renderer(self):
        with patch.object(bootstrap.Path, "read_bytes", return_value=b"tampered"), \
                patch.object(bootstrap.subprocess, "run") as run:
            with self.assertRaises(bootstrap.authority.AnchorError):
                bootstrap.calico_manifest()
        run.assert_not_called()

    def test_cni_apply_uses_authority_and_unknown_topology_never_renders(self):
        for daemonsets, cidr, allowed in ((["kube-proxy"], "10.244.0.0/24", True),
                                          (["kindnet"], "10.244.0.0/24", False),
                                          (["foreign"], "10.244.0.0/24", False),
                                          (["kube-proxy"], "192.168.0.0/24", False)):
            calls = []
            def kubectl(args, **kwargs):
                calls.append((args, kwargs))
                if "daemonsets" in args:
                    return json.dumps({"items": [{"metadata": {"name": n}} for n in daemonsets]})
                if "jsonpath={.items[*].spec.podCIDR}" in args:
                    return cidr
                if "ippools" in args:
                    return json.dumps({"items": [{"spec": {"cidr": "10.244.0.0/16"}}]})
                return ""
            with self.subTest(daemonsets=daemonsets, cidr=cidr), \
                    patch.object(bootstrap.authority, "bootstrap"), \
                    patch.object(bootstrap.authority, "LocalClusterTargetAnchor") as anchor, \
                    patch.object(bootstrap, "calico_manifest", return_value="offline-document") as render:
                anchor.return_value.kubectl.side_effect = kubectl
                if allowed:
                    bootstrap.initialize(Path("private-fixture"), "aiforall-local-full", Path("cluster-local-full.yaml"), True)
                    apply = [(a, k) for a, k in calls if "apply" in a]
                    self.assertEqual(len(apply), 1)
                    self.assertEqual(apply[0][1]["input"], "offline-document")
                    self.assertFalse(any("set" in a for a, _ in calls))
                else:
                    with self.assertRaises(bootstrap.authority.AnchorError):
                        bootstrap.initialize(Path("private-fixture"), "aiforall-local-full", Path("cluster-local-full.yaml"), True)
                    render.assert_not_called()
                    self.assertFalse(any("apply" in a for a, _ in calls))


if __name__ == "__main__":
    unittest.main()
