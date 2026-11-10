"""Offline preservation boundaries: no Docker, cluster or real credentials."""
import copy
import importlib.util
import json
from pathlib import Path
import sys
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
from local_full_tls import PLATFORM_TLS, omit_preserved_tls, preserved_platform_tls, reuse_openbao_tls

_spec = importlib.util.spec_from_file_location("tls_deploy", Path(__file__).resolve().parents[2] / "ops/deploy/deploy-local-full.py")
deploy = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(deploy)


class PlatformTlsTests(unittest.TestCase):
    def setUp(self):
        self.data = {"anchor_state": "anchored", "cluster": "aiforall-local-full",
            "context": "kind-aiforall-local-full", "namespaceUids": {
                "aiforall-local-full": "full-uid", "aiforall-local-plugins": "plugins-uid"},
            "localSecretInputs": {ns + "/" + name: True for ns, name in PLATFORM_TLS}, "ownedResources": []}
        self.deployments = {}
        for name, secret in (("envoy-mesh", "platform-mesh-certs"), ("lazaret", "lazaret-identity-ca")):
            meta = {"name": name, "namespace": "aiforall-local-full", "uid": name + "-uid"}
            self.deployments[name] = {"metadata": meta, "spec": {"template": {"spec": {
                "volumes": [{"name": "tls", "secret": {"secretName": secret}}],
                "containers": [{"volumeMounts": [{"name": "tls", "mountPath": "/tls"}]}]}}}}
            self.data["ownedResources"].append({**meta, "kind": "Deployment",
                "cluster": self.data["cluster"], "context": self.data["context"]})
        self.read = Mock(side_effect=lambda name: copy.deepcopy(self.deployments[name]))

    def preserve(self, **kwargs):
        return preserved_platform_tls(self.data, enabled=True, oauth_connect=True,
                                      read_deployment=self.read, **kwargs)

    def test_default_does_not_read_or_preserve_anything(self):
        self.assertFalse(preserved_platform_tls({}, enabled=False, oauth_connect=False, read_deployment=self.read))
        self.read.assert_not_called()

    def test_deploy_flag_guards_refuse_before_private_or_runtime_reads(self):
        for oauth, isolated in ((False, False), (True, True)):
            args = SimpleNamespace(oauth_connect=oauth, preserve_platform_tls=True, isolated_plugins=isolated)
            with patch.object(deploy, "cert_inputs") as certs, patch.object(deploy, "k") as query:
                with self.assertRaises(ValueError):
                    deploy.deploy_locked(args)
                certs.assert_not_called()
                query.assert_not_called()

    def test_cli_refuses_preservation_outside_oauthconnect_deploy(self):
        root = Path(__file__).resolve().parents[2]
        for args in (("render", "--oauth-connect"), ("deploy",), ("status", "--oauth-connect")):
            result = subprocess.run([sys.executable, "-B", "ops/deploy/deploy-local-full.py", *args,
                                     "--preserve-platform-tls"], cwd=root, capture_output=True, text=True, timeout=15)
            self.assertEqual(result.returncode, 2)
            self.assertIn("only valid for deploy --oauth-connect", result.stderr)

    def test_exact_leased_mounted_consumers_are_required_without_secret_reads(self):
        before = copy.deepcopy(self.data)
        self.assertEqual(self.preserve(), PLATFORM_TLS)
        self.assertEqual([c.args for c in self.read.call_args_list], [("envoy-mesh",), ("lazaret",)])
        self.assertEqual(self.data, before)

    def test_wrong_profile_anchor_context_or_cluster_refused_before_reads(self):
        for field, value in (("anchor_state", "pending_creation"), ("context", "kind-aiforall-local"),
                             ("cluster", "aiforall-local")):
            with self.subTest(field=field):
                data = {**self.data, field: value}
                with self.assertRaises(ValueError):
                    preserved_platform_tls(data, enabled=True, oauth_connect=True, read_deployment=self.read)
        with self.assertRaises(ValueError):
            preserved_platform_tls(self.data, enabled=True, oauth_connect=False, read_deployment=self.read)
        self.read.assert_not_called()

    def test_missing_namespace_or_acknowledgement_refuses_before_reads(self):
        for field in ("namespaceUids", "localSecretInputs"):
            data = {**self.data, field: {}}
            with self.assertRaises(ValueError):
                preserved_platform_tls(data, enabled=True, oauth_connect=True, read_deployment=self.read)
        self.read.assert_not_called()

    def test_unknown_uid_reference_and_unmounted_volume_are_refused(self):
        original = copy.deepcopy(self.deployments)
        for change in ("uid", "reference", "unmounted"):
            self.deployments = copy.deepcopy(original)
            obj = self.deployments["envoy-mesh"]
            if change == "uid":
                obj["metadata"]["uid"] = "foreign"
            elif change == "reference":
                obj["spec"]["template"]["spec"]["volumes"][0]["secret"]["secretName"] = "foreign"
            else:
                obj["spec"]["template"]["spec"]["containers"][0]["volumeMounts"] = []
            with self.assertRaises(ValueError):
                self.preserve()

    def test_omit_only_exact_tls_secrets_preserving_other_namespaces_and_new_tls(self):
        objects = [{"kind": "Secret", "metadata": {"namespace": ns, "name": name}}
                   for ns, name in PLATFORM_TLS]
        keep = [{"kind": "Secret", "metadata": {"namespace": "other", "name": "platform-mesh-certs"}},
                {"kind": "Secret", "metadata": {"namespace": "aiforall-local-full", "name": "oauth-connect-server-tls-run"}},
                {"kind": "ConfigMap", "metadata": {"namespace": "aiforall-local-full", "name": "platform-mesh-certs"}}]
        before = copy.deepcopy(objects + keep)
        self.assertEqual(omit_preserved_tls(objects + keep, self.preserve()), keep)
        self.assertEqual(objects + keep, before)
        self.assertEqual(omit_preserved_tls(objects + keep, set()), before)
        with self.assertRaises(ValueError):
            omit_preserved_tls(objects + keep, {("other", "foreign")})

    def openbao(self):
        obj = copy.deepcopy(self.deployments["envoy-mesh"])
        obj["metadata"].update(name="openbao", uid="bao-uid")
        obj["spec"]["template"]["spec"]["volumes"][0]["secret"]["secretName"] = "openbao-server-tls-original"
        self.deployments["openbao"] = obj
        self.data["ownedResources"] += [
            {"kind": kind, "name": name, "uid": uid, "namespace": "aiforall-local-full",
             "context": self.data["context"], "cluster": self.data["cluster"]}
            for kind, name, uid in (("Deployment", "openbao", "bao-uid"),
                                    ("Secret", "openbao-server-tls-original", "tls-uid"))]
        proposed = copy.deepcopy(obj)
        proposed["spec"]["template"]["spec"]["volumes"][0]["secret"]["secretName"] = "openbao-server-tls-new"
        proposed["kind"] = "Deployment"
        tls = [{"kind": "Secret", "metadata": {"name": name, "namespace": "aiforall-local-full"}}
               for name in ("openbao-server-tls-new", "oauth-connect-server-tls-new")]
        return proposed, tls

    def test_openbao_leaf_reference_is_preserved_and_only_connector_tls_is_created(self):
        proposed, tls = self.openbao()
        original = copy.deepcopy(proposed)
        filtered, refs = reuse_openbao_tls([proposed], tls, self.data, self.read)
        self.assertEqual(filtered, [tls[1]])
        self.assertEqual(refs, {"openbao-server-tls-new": "openbao-server-tls-original"})
        self.assertEqual(proposed, original, "preservation returns references, never rewrites a spec")
        self.read.assert_called_once_with("openbao")

    def test_unknown_openbao_reference_and_ambiguous_proposal_refuse_without_mutation(self):
        for case in ("unrecorded", "ambiguous", "wrong-consumer"):
            with self.subTest(case=case):
                proposed, tls = self.openbao()
                if case == "unrecorded":
                    self.data["ownedResources"] = [x for x in self.data["ownedResources"] if x["kind"] != "Secret"]
                elif case == "wrong-consumer":
                    self.deployments["openbao"]["metadata"]["uid"] = "foreign"
                else:
                    tls.append(copy.deepcopy(tls[0]))
                before = copy.deepcopy((proposed, tls))
                with self.assertRaises(ValueError):
                    reuse_openbao_tls([proposed], tls, self.data, self.read)
                self.assertEqual((proposed, tls), before)

    def test_actual_deploy_omits_preserved_tls_and_creates_only_new_connector_leaf(self):
        import hashlib
        from types import SimpleNamespace
        import local_full_phases as phases
        import local_full_declarations
        import local_full_network
        proposed, _ = self.openbao()
        run_id = "a" * 16
        new_bao = "openbao-server-tls-" + run_id
        proposed["spec"]["template"]["spec"]["volumes"][0]["secret"]["secretName"] = new_bao
        original = copy.deepcopy(proposed)
        foundation = [{"kind": "ConfigMap", "metadata": {"namespace": "aiforall-local-full", "name": "public-fixture"}}]
        foundation += [{"kind": "Secret", "metadata": {"namespace": ns, "name": name}} for ns, name in PLATFORM_TLS]
        tls = [{"kind": "Secret", "metadata": {"namespace": "aiforall-local-full", "name": name + "-" + run_id}}
               for name in ("openbao-server-tls", "oauth-connect-server-tls")]
        def render(*args, **kwargs):
            if kwargs.get("phase") == "foundation":
                return copy.deepcopy(foundation)
            result = copy.deepcopy(proposed)
            result["spec"]["template"]["spec"]["volumes"][0]["secret"]["secretName"] = kwargs.get("secret_refs", {}).get(new_bao, new_bao)
            return [result]
        def query(*args, **kwargs):
            self.assertGreater(deploy.runtime.lease.call_count, 0, "explicit lease binding must precede API reads")
            if args[:2] == ("get", "configmap"):
                return json.dumps({"data": {"ca.crt": "public-root"}})
            return json.dumps(self.deployments[args[2]])
        with tempfile.TemporaryDirectory() as directory:
            lease = Path(directory) / "lease.json"
            lease.write_text(json.dumps(self.data))
            lease.with_name(lease.name + ".local-full-lock").write_text(json.dumps({"operation": "owned-operation", "action": "deploy"}))
            args = SimpleNamespace(lease=lease, cert_directory=Path(directory), oauth_connect=True,
                                   preserve_platform_tls=True, isolated_plugins=False, registry_state=None)
            invocation = Path(directory) / "invocation.json"
            def context(phase, predecessor):
                value = {"operation": "owned-operation", "runId": run_id, "inputs": [],
                    "graphSha256": hashlib.sha256((deploy.ROOT / "ops/deploy/local-full.just").read_bytes()).hexdigest(),
                    "declarationsId": local_full_declarations.identity(), "args": {
                        key: {"path": str(value)} if isinstance(value, Path) else value for key, value in vars(args).items()},
                    "completed": [] if predecessor == "none" else [predecessor],
                    "state": "prepared" if predecessor == "none" else "acknowledged",
                    "publicOptions": {"network": {"apiCidr": "192.0.2.20/32", "ingressCidr": ""},
                        "secret_refs": {new_bao: "openbao-server-tls-original"}}}
                invocation.write_text(json.dumps(value))
            api = SimpleNamespace(**vars(deploy))
            with (patch.object(phases, "private_file", return_value=True), patch.object(phases, "input_stamp", return_value=[]),
                  patch.object(phases, "inputs", return_value=({"network": {"apiCidr": "192.0.2.20/32", "ingressCidr": ""}}, {"ca.crt": b"public-root"}, {"ca.crt": b"identity-root"}, None, None, tls)),
                  patch.object(deploy.renderer, "render", side_effect=render), patch.object(deploy, "k", side_effect=query),
                  patch.object(api, "k", side_effect=query), patch.object(api, "apply_owned") as apply,
                  patch.object(deploy.runtime, "lease", return_value=self.data), patch.object(deploy.runtime, "save_lease"),
                  patch.object(deploy.runtime, "target", return_value=SimpleNamespace(data=self.data)),
                  patch.object(local_full_network, "api_preimages", return_value=[]), patch.object(local_full_network, "apply_changes"),
                  patch.object(deploy.runtime, "create", return_value={"metadata": {"uid": "new-leaf-uid"}}) as create):
                context("foundation", "quiesce")
                phases.guarded_phase(api, invocation, "foundation", "quiesce")
                self.assertEqual([c.args[0]["metadata"]["name"] for c in create.call_args_list], ["oauth-connect-server-tls-" + run_id])
                self.assertTrue(all(o["kind"] != "Secret" for c in apply.call_args_list for o in c.args[0]))
                self.assertEqual(proposed, original, "preservation must not mutate a rendered spec")
                self.assertEqual(json.loads(invocation.read_text())["publicOptions"]["secret_refs"], {new_bao: "openbao-server-tls-original"})
                create.reset_mock(); apply.reset_mock()
                context("plan", "none")
                def mismatch(*arguments, **kwargs):
                    if arguments[:2] == ("get", "configmap"):
                        return json.dumps({"data": {"ca.crt": "different-root"}})
                    return query(*arguments, **kwargs)
                with patch.object(api, "k", side_effect=mismatch):
                    with self.assertRaisesRegex(ValueError, "unchanged OpenBao public CA"):
                        phases.guarded_phase(api, invocation, "plan", "none")
                create.assert_not_called(); apply.assert_not_called()

    def test_each_successful_apply_is_recorded_before_a_later_conflict(self):
        first, second, third = [{"kind": "ConfigMap", "metadata": {"name": name}} for name in ("first", "second", "third")]
        order = []
        def apply(items):
            name = items[0]["metadata"]["name"]
            order.append(("apply", name))
            if name == "second":
                raise ValueError("synthetic conflict")
        def record(items, args, data):
            order.append(("record", items[0]["metadata"]["name"]))
        data = {"ownedResources": [{"kind": "ConfigMap", "name": obj["metadata"]["name"], "namespace": None,
                "uid": "uid-" + obj["metadata"]["name"], "context": deploy.CONTEXT, "cluster": deploy.CLUSTER}
                for obj in (first, second, third)]}
        def current(*args, **kwargs):
            return json.dumps({"metadata": {"uid": "uid-" + args[2], "resourceVersion": "1"}})
        with patch.object(deploy, "k", side_effect=current), patch.object(deploy, "apply", side_effect=apply), patch.object(deploy, "record_resources", side_effect=record):
            with self.assertRaisesRegex(ValueError, "synthetic conflict"):
                deploy.apply_owned([first, second, third], SimpleNamespace(), data)
        self.assertEqual(order, [("apply", "first"), ("record", "first"), ("apply", "second")])


if __name__ == "__main__":
    unittest.main()
