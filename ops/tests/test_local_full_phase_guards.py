"""Offline failure/ownership oracles, including the real just dependency engine."""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ops/deploy"))
import local_full_declarations
import local_full_helm as helm
import local_full_phases as phases

spec = importlib.util.spec_from_file_location("phase_deployer", ROOT / "ops/deploy/deploy-local-full.py")
deploy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deploy)


class PhaseGuards(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.lease = self.directory / "lease.json"
        self.data = {"context": deploy.CONTEXT, "cluster": deploy.CLUSTER, "anchor_state": "anchored"}
        self.lease.write_text(json.dumps(self.data))
        self.lock = self.lease.with_name(self.lease.name + ".local-full-lock")
        self.lock.write_text(json.dumps({"action": "deploy", "operation": "owned-operation"}))
        self.path = self.directory / "invocation.json"
        self.context = {"operation": "owned-operation", "args": {"lease": {"path": str(self.lease)}},
            "runId": "syntheticattempt", "inputs": [], "completed": ["foundation"], "state": "acknowledged",
            "graphSha256": hashlib.sha256((ROOT / "ops/deploy/local-full.just").read_bytes()).hexdigest(),
            "declarationsId": local_full_declarations.identity()}
        self.api = SimpleNamespace(ROOT=ROOT, CLUSTER=deploy.CLUSTER, CONTEXT=deploy.CONTEXT,
            runtime=SimpleNamespace(lease=Mock(return_value=self.data), save_lease=Mock()),
            renderer=SimpleNamespace(render=Mock(return_value=[])), apply_owned=Mock(), k=Mock())

    def test_missing_predecessor_crash_nonce_graph_and_input_drift_refuse_before_reads(self):
        cases = ("predecessor", "crash", "nonce", "graph", "declarations", "inputs", "replay")
        for case in cases:
            with self.subTest(case=case):
                context = copy.deepcopy(self.context)
                predecessor = "foundation"
                if case == "predecessor":
                    predecessor = "none"
                    context["completed"] = []
                if case == "crash": context["state"] = "in-progress"
                if case == "nonce": context["operation"] = "other-operation"
                if case == "graph": context["graphSha256"] = "0" * 64
                if case == "declarations": context["declarationsId"] = "0" * 64
                if case == "replay": context["completed"].append("dependencies")
                self.path.write_text(json.dumps(context))
                with patch.object(phases, "private_file", return_value=True), \
                     patch.object(phases, "input_stamp", return_value=["changed"] if case == "inputs" else []), \
                     patch.object(phases, "inputs") as inputs:
                    with self.assertRaises(ValueError):
                        phases.guarded_phase(self.api, self.path, "dependencies", predecessor)
                    inputs.assert_not_called()
                self.api.runtime.lease.assert_not_called()
                self.api.apply_owned.assert_not_called()

    def test_mutator_failure_retains_incomplete_receipt_and_never_waits_or_retries(self):
        self.path.write_text(json.dumps(self.context))
        self.api.renderer.render.return_value = [{"kind": "Deployment", "metadata": {"name": "synthetic-dependency",
            "namespace": deploy.CLUSTER, "annotations": {"aiforall.dev/category": "dependencies"}}}]
        self.api.apply_owned.side_effect = ValueError("synthetic mutation refused")
        with patch.object(phases, "private_file", return_value=True), patch.object(phases, "input_stamp", return_value=[]), \
             patch.object(phases, "inputs", return_value=({}, {}, {}, None, None, [])) as inputs, \
             patch.object(phases, "wait_workloads") as wait:
            with self.assertRaisesRegex(ValueError, "synthetic mutation"):
                phases.guarded_phase(self.api, self.path, "dependencies", "foundation")
            self.assertEqual(json.loads(self.path.read_text())["state"], "in-progress")
            self.assertEqual(self.data["localFullOperation"]["state"], "in-progress")
            wait.assert_not_called()
            with self.assertRaises(ValueError):
                phases.guarded_phase(self.api, self.path, "dependencies", "foundation")
            self.assertEqual(inputs.call_count, 1)
            self.api.apply_owned.assert_called_once()

    def test_foreign_uid_never_applies_or_adopts(self):
        obj = {"kind": "Deployment", "metadata": {"name": "synthetic-writer", "namespace": deploy.CLUSTER}, "spec": {"replicas": 1}}
        data = {"ownedResources": [{"kind": "Deployment", "name": "synthetic-writer", "namespace": deploy.CLUSTER,
            "uid": "owned", "context": deploy.CONTEXT, "cluster": deploy.CLUSTER}]}
        original = copy.deepcopy(obj)
        with patch.object(deploy, "k", return_value=json.dumps({"metadata": {"uid": "foreign", "resourceVersion": "1"}})), \
             patch.object(deploy, "apply") as apply, patch.object(deploy.runtime, "create") as create:
            with self.assertRaises(ValueError): deploy.apply_owned([obj], SimpleNamespace(), data)
            apply.assert_not_called(); create.assert_not_called()
        self.assertEqual(obj, original)

    def test_native_render_refusal_precedes_any_mutator_or_in_progress_receipt(self):
        self.path.write_text(json.dumps(self.context))
        self.api.renderer.render.side_effect = RuntimeError("native declaration refused")
        with patch.object(phases, "private_file", return_value=True), patch.object(phases, "input_stamp", return_value=[]), \
             patch.object(phases, "inputs", return_value=({}, {}, {}, None, None, [])):
            with self.assertRaisesRegex(RuntimeError, "native declaration refused"):
                phases.guarded_phase(self.api, self.path, "dependencies", "foundation")
        self.api.apply_owned.assert_not_called()
        self.api.runtime.save_lease.assert_not_called()
        self.assertEqual(json.loads(self.path.read_text())["state"], "acknowledged")

    def test_namespace_and_cluster_authority_maps_cannot_be_overridden_by_inventory_rows(self):
        for kind, authority in (("Namespace", "namespaceUids"), ("CustomResourceDefinition", "clusterResourceUids")):
            with self.subTest(kind=kind):
                obj = {"kind": kind, "metadata": {"name": "synthetic"}}
                key = "synthetic" if kind == "Namespace" else kind + "/synthetic"
                data = {authority: {key: "original"}, "ownedResources": [{"kind": kind, "name": "synthetic",
                    "namespace": None, "uid": "foreign", "context": deploy.CONTEXT, "cluster": deploy.CLUSTER}]}
                with patch.object(deploy, "k", return_value=json.dumps({"metadata": {"uid": "foreign"}})), \
                     patch.object(deploy, "apply") as apply, patch.object(deploy.runtime, "create") as create:
                    with self.assertRaisesRegex(ValueError, "not explicitly owned"):
                        deploy.preflight_resources([obj], data)
                    apply.assert_not_called(); create.assert_not_called()

    def test_declarative_native_cli_preserves_types_defaults_and_flags(self):
        args = helm.parser("deploy").parse_args(["deploy", "--lease", "lease.json", "--oauth-connect", "--preserve-platform-tls",
            "--simulated-input-file", "sim.json", "--github-input-file", "gh.json", "--after-final-it", "--confirm-quiesce"])
        self.assertEqual(args.lease, Path("lease.json"))
        self.assertEqual(args.environment_dir, Path.home() / ".aiforall-local-full")
        self.assertTrue(args.oauth_connect and args.preserve_platform_tls and args.after_final_it and args.confirm_quiesce)
        self.assertFalse(args.oauth_proof or args.github_openbao or args.isolated_plugins)
        self.assertEqual(args.simulated_input_file, Path("sim.json"))
        args = helm.parser("render", choices={"--phase": helm.PHASES}).parse_args(["--output-dir", "out", "--phase", "foundation"])
        self.assertEqual((args.output_dir, args.phase, args.run_id), (Path("out"), "foundation", "staticvalidation"))
        args = helm.parser("storage", choices={"--volume": list(deploy.runtime.STORAGE_SOURCES)}).parse_args([
            "backup", "--lease", "lease.json", "--archive", "backup.bin", "--volume", "admission", "--confirm-quiesce"])
        self.assertEqual((args.archive, args.database, args.volume), (Path("backup.bin"), "manifesto_dev", "admission"))
        self.assertTrue(args.confirm_quiesce)

    def test_target_uses_declared_namespaces_and_stops_on_concurrent_job(self):
        declaration = {"kind": "Namespace", "metadata": {"name": "synthetic-namespace"}}
        for active in ("0", "1"):
            with self.subTest(active=active):
                context = copy.deepcopy(self.context)
                context["completed"] = ["plan"]
                context["args"]["after_final_it"] = True
                context["args"]["registry_state"] = None
                self.path.write_text(json.dumps(context))
                self.data.pop("localFullOperation", None)
                api = SimpleNamespace(**vars(self.api))
                api.renderer = SimpleNamespace(render=Mock(return_value=[declaration]))
                for name in ("preflight_resources", "prepare_namespaces", "verify_images"):
                    setattr(api, name, Mock())
                api.bootstrap_local_kind = SimpleNamespace(initialize=Mock())
                api.k = Mock(return_value=active)
                with patch.object(phases, "private_file", return_value=True), patch.object(phases, "input_stamp", return_value=[]), \
                     patch.object(phases, "inputs", return_value=({"images": {}}, {}, {}, None, None, [])), \
                     patch.object(phases, "network_inputs", return_value={}):
                    if active == "1":
                        with self.assertRaisesRegex(ValueError, "active job"):
                            phases.guarded_phase(api, self.path, "target", "plan")
                    else:
                        phases.guarded_phase(api, self.path, "target", "plan")
                self.assertEqual(api.k.call_args.args[:4], ("get", "jobs", "-n", "synthetic-namespace"))
        with patch.object(deploy, "k", return_value=""), patch.object(deploy.runtime, "create") as create:
            deploy.prepare_namespaces(SimpleNamespace(), {}, [declaration])
        self.assertEqual(create.call_args.args[0], declaration)

    def test_changed_lock_is_not_unlinked_or_reclaimed_by_pid(self):
        self.lock.unlink()  # Synthetic fixture, not a runtime/parent lock.
        with self.assertRaisesRegex(ValueError, "ownership changed"):
            with deploy.runtime.operation_lock(self.lease, "synthetic"):
                self.lock.write_text(json.dumps({"operation": "foreign", "pid": os.getpid()}))
        self.assertTrue(self.lock.exists())

    def test_just_child_environment_and_argv_never_carry_private_values(self):
        args = SimpleNamespace(lease=self.lease, oauth_proof=False)
        def child(command, **kwargs):
            env = kwargs["env"]
            for name in ("GITHUB_CLIENT_SECRET", "KUBECONFIG", "DOCKER_HOST", "PYTHONPATH"):
                self.assertNotIn(name, env)
            self.assertNotIn("PRIVATE_SENTINEL", json.dumps(command) + json.dumps(env))
            path = Path(env["AIFORALL_LOCAL_FULL_INVOCATION"])
            value = json.loads(path.read_text())
            value["state"] = "complete"
            path.write_text(json.dumps(value))
            return SimpleNamespace(returncode=0)
        with patch.dict(os.environ, GITHUB_CLIENT_SECRET="PRIVATE_SENTINEL", KUBECONFIG="PRIVATE_SENTINEL", DOCKER_HOST="PRIVATE_SENTINEL", PYTHONPATH="PRIVATE_SENTINEL"), \
             patch.object(phases, "private_file", return_value=True), patch.object(phases, "input_stamp", return_value=[]), \
             patch.object(phases.shutil, "which", return_value="just-verified"), patch.object(phases.subprocess, "run", side_effect=child):
            phases.run(self.api, args, "owned-operation")
            self.assertEqual(os.environ["GITHUB_CLIENT_SECRET"], "PRIVATE_SENTINEL", "caller environment must remain untouched")

    def test_real_just_engine_stops_at_every_failed_phase(self):
        just = shutil.which("just")
        self.assertIsNotNone(just)
        expected = ["plan", "target", "quiesce", "foundation", "dependencies", "bootstrap", "applications-first", "final"]
        source = (ROOT / "ops/deploy/local-full.just").read_text()
        recipes = re.findall(r"^([a-z-]+):(?: ([a-z-]+))?\s*$", source, re.MULTILINE)
        probe = self.directory / "probe.py"
        probe.write_text("import os,sys\nfrom pathlib import Path\np=Path(os.environ['TRACE'])\nwith p.open('a',encoding='utf-8') as f:f.write(sys.argv[1]+'\\n')\nraise SystemExit(7 if sys.argv[1]==os.environ.get('FAIL_PHASE') else 0)\n")
        graph = self.directory / "graph.just"
        text = "set shell := " + json.dumps([sys.executable, "-c"]) + "\nset dotenv-load := false\n"
        for name, predecessor in recipes:
            text += "\n" + name + ":" + (" " + predecessor if predecessor else "") + "\n"
            text += "    @import os,subprocess,sys; sys.exit(subprocess.run([sys.executable,os.environ['PROBE']," + repr(name) + "]).returncode)\n"
        graph.write_text(text)
        trace = self.directory / "trace.txt"
        for phase in [*expected, "success"]:
            with self.subTest(phase=phase):
                trace.unlink(missing_ok=True)
                env = dict(os.environ, PROBE=str(probe), TRACE=str(trace), FAIL_PHASE=phase)
                result = subprocess.run([just, "--justfile", str(graph), "run"], env=env, capture_output=True, text=True, timeout=30)
                called = trace.read_text().splitlines()
                self.assertEqual(called, expected + ["run"] if phase == "success" else expected[:expected.index(phase) + 1])
                self.assertEqual(result.returncode == 0, phase == "success")


if __name__ == "__main__": unittest.main()
