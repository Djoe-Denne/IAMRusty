"""Hermetic closed-forward tests: no subprocess, sockets, keys or runtime."""
import copy
import importlib.util
import json
from pathlib import Path
from types import SimpleNamespace
import sys
import tempfile
import time
import unittest
from unittest.mock import Mock, patch
import uuid


DEPLOY = Path(__file__).resolve().parents[1] / "deploy"
sys.path.insert(0, str(DEPLOY))
SPEC = importlib.util.spec_from_file_location("e2e_forward_target_tests", DEPLOY / "local_full_target.py")
target = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = target
SPEC.loader.exec_module(target)
NS = "aiforall-local-full"


def uid():
    return str(uuid.uuid4())


class Fixture:
    def __init__(self, directory, service="iam"):
        self.service = service
        name, port = target._E2E_FORWARD_TARGETS[service]
        self.namespace_uid, self.service_uid, self.pod_uid, self.rs_uid, self.deployment_uid = [uid() for _ in range(5)]
        self.pod = service + "-rs-pod"
        self.calls = []
        self.anchor = target.LocalClusterTargetAnchor.__new__(target.LocalClusterTargetAnchor)
        self.anchor.data = {"cluster": NS, "context": "kind-aiforall-local-full", "task_id": "pure-test",
            "namespaceUids": {NS: self.namespace_uid}, "ownedResources": [
                {"kind": kind, "name": service, "namespace": NS, "uid": value,
                 "cluster": NS, "context": "kind-aiforall-local-full"}
                for kind, value in (("Service", self.service_uid), ("Deployment", self.deployment_uid))]}
        self.anchor.path = directory / "lease.json"
        self.anchor.config = directory / "kubeconfig.yaml"
        self.anchor.local_check = Mock()
        self.anchor.prefix = lambda: ["kubectl", "--kubeconfig", "fixed-private-path", "--context", "kind-aiforall-local-full"]
        self.anchor.environment = lambda: {"FIXED": "test"}
        self.anchor.kubectl = self.kubectl
        self.svc = {"metadata": {"uid": self.service_uid, "namespace": NS}, "spec": {
            "selector": {"app.kubernetes.io/name": service}, "type": "ClusterIP",
            "ports": [{"name": name, "port": port, "targetPort": name, "protocol": "TCP"}]}}
        self.endpoint = {"addresses": ["192.0.2.3"], "conditions": {"ready": True},
            "targetRef": {"kind": "Pod", "namespace": NS, "name": self.pod, "uid": self.pod_uid}}
        self.slices = {"items": [{"metadata": {"namespace": NS, "ownerReferences": [
            {"kind": "Service", "name": service, "uid": self.service_uid, "controller": True}]},
            "ports": [{"name": name, "port": port, "protocol": "TCP"}], "endpoints": [self.endpoint]}]}
        self.pod_lines = [self.pod_uid, self.pod, NS, "", service, "Running", "192.0.2.3", "", "", "True",
            "C\t" + service, f"P\t{name}\t{port}\tTCP",
            "\t".join(["S", service, "containerd://" + "a" * 64, "0", "image@sha256:" + "b" * 64, "true", "2026-11-02T00:00:00Z"]),
            "\t".join(["R", "ReplicaSet", service + "-rs", self.rs_uid, "true"])]
        self.rs_lines = [self.rs_uid, service + "-rs", NS, "",
                         "\t".join(["R", "Deployment", service, self.deployment_uid, "true"])]
        self.deployment_lines = [self.deployment_uid, service, NS, ""]

    def kubectl(self, args, **kwargs):
        self.calls.append(args)
        kind = args[1]
        if kind == "namespace":
            return self.namespace_uid
        if kind == "service":
            return json.dumps(self.svc)
        if kind == "endpointslices":
            return json.dumps(self.slices)
        if kind == "pod":
            return "\n".join(self.pod_lines) + "\n"
        if kind == "replicaset":
            return "\n".join(self.rs_lines) + "\n"
        if kind == "deployment":
            return "\n".join(self.deployment_lines) + "\n"
        raise AssertionError("unapproved fake operation")

    def identity(self):
        return self.anchor._e2e_forward_identity(self.service, self.namespace_uid, self.service_uid)


class Child:
    def __init__(self, service):
        self.service = service
        self.process = SimpleNamespace(pid=12345)
        self.deadline = time.monotonic() + 30
        self.closed = False
        self.commands = []
        self.fail_ready = self.fail_close = False

    def start(self, command, env):
        self.commands.append(command)

    def ready(self):
        if self.fail_ready:
            raise TimeoutError("pure timeout")
        return 44111

    def mapping(self, **kwargs):
        return 44111

    def close(self):
        self.closed = True
        if self.fail_close:
            raise target.AnchorError("pure cleanup blocked")


class ForwardTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="local-full-forward-pure-")
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)

    def test_four_fixed_targets_attest_public_metadata_only(self):
        for service, (_, port) in target._E2E_FORWARD_TARGETS.items():
            with self.subTest(service=service):
                f = Fixture(self.directory, service)
                identity = f.identity()
                self.assertEqual(identity["remote_port"], port)
                self.assertEqual(identity["pod_uid"], f.pod_uid)
                self.assertTrue(all(call[0] == "get" and "secret" not in call for call in f.calls))
                self.assertTrue(all(call[-1].startswith("jsonpath=") for call in f.calls if call[1] in {"pod", "replicaset", "deployment"}))

    def test_identity_rejects_uid_backend_owner_port_and_incarnation_drift(self):
        mutations = {
            "service UID": lambda f: f.svc["metadata"].update(uid=uid()),
            "ownership": lambda f: f.anchor.data["ownedResources"].pop(),
            "selector": lambda f: f.svc["spec"].update(selector={}),
            "service port": lambda f: f.svc["spec"]["ports"][0].update(port=443),
            "slice owner": lambda f: f.slices["items"][0]["metadata"]["ownerReferences"][0].update(uid=uid()),
            "slice port": lambda f: f.slices["items"][0]["ports"][0].update(port=443),
            "multiple backends": lambda f: f.slices["items"][0]["endpoints"].append(copy.deepcopy(f.endpoint)),
            "no backend": lambda f: f.slices["items"][0].update(endpoints=[]),
            "not Ready": lambda f: f.endpoint["conditions"].update(ready=False),
            "terminating": lambda f: f.endpoint["conditions"].update(terminating=True),
            "pod replacement": lambda f: f.endpoint["targetRef"].update(uid=uid()),
            "pod label": lambda f: f.pod_lines.__setitem__(4, "foreign"),
            "pod phase": lambda f: f.pod_lines.__setitem__(5, "Pending"),
            "pod not Ready": lambda f: f.pod_lines.__setitem__(9, "False"),
            "pod IP": lambda f: f.pod_lines.__setitem__(6, "192.0.2.4"),
            "sidecar": lambda f: f.pod_lines.insert(10, "C\tforeign"),
            "not running": lambda f: f.pod_lines.__setitem__(12, f.pod_lines[12].rsplit("\t", 1)[0] + "\t"),
            "wrong controller": lambda f: f.rs_lines.__setitem__(4, f.rs_lines[4].replace("\ttrue", "\tfalse")),
            "deployment replacement": lambda f: f.deployment_lines.__setitem__(0, uid()),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                f = Fixture(self.directory)
                mutate(f)
                with self.assertRaises((target.AnchorError, KeyError)):
                    f.identity()

    def test_declared_iam_oauth_trust_init_must_have_completed_successfully(self):
        for mode in ("declared", "failed", "not terminated", "foreign name", "other service"):
            with self.subTest(mode=mode):
                f = Fixture(self.directory, "hive" if mode == "other service" else "iam")
                f.pod_lines[7] = "foreign-init" if mode == "foreign name" else "oauth-proof-trust"
                status = ["I", "oauth-proof-trust", "containerd://" + "c" * 64, "0",
                          "image@sha256:" + "d" * 64, "0", "2026-11-02T00:00:00Z"]
                if mode == "failed":
                    status[5] = "1"
                elif mode == "not terminated":
                    status[5:] = ["", ""]
                f.pod_lines.insert(-1, "\t".join(status))
                if mode == "declared":
                    self.assertEqual(f.identity()["pod_uid"], f.pod_uid)
                else:
                    with self.assertRaises(target.AnchorError):
                        f.identity()

    def test_lifecycle_targets_pod_loopback_and_closes_after_success_or_caller_failure(self):
        for service in target._E2E_FORWARD_TARGETS:
            for failure in (False, True):
                with self.subTest(service=service, failure=failure):
                    f = Fixture(self.directory, service)
                    identity = f.identity()
                    f.anchor._e2e_forward_identity = Mock(return_value=identity)
                    child = Child(service)
                    with patch.object(target, "_E2EForward", return_value=child), patch.object(target, "_admin_marker_private"):
                        def use():
                            with f.anchor.e2e_forward(service, namespace_uid=f.namespace_uid, service_uid=f.service_uid) as port:
                                self.assertEqual(port, 44111)
                                if failure:
                                    raise AssertionError("caller oracle failed")
                        if failure:
                            with self.assertRaisesRegex(AssertionError, "caller oracle failed"):
                                use()
                        else:
                            use()
                    self.assertTrue(child.closed)
                    self.assertEqual(f.anchor._e2e_forward_identity.call_count, 4)
                    self.assertFalse((self.directory / ".e2e-forward").exists())
                    command = child.commands[0]
                    self.assertIn("pod/" + f.pod, command)
                    self.assertEqual(command[-3:], ["--address", "127.0.0.1", ":" + str(identity["remote_port"])])

    def test_foreign_target_or_marker_never_launches_or_adopts(self):
        f = Fixture(self.directory)
        with patch.object(target, "_E2EForward") as child:
            with self.assertRaises(target.AnchorError):
                with f.anchor.e2e_forward("postgres", namespace_uid=f.namespace_uid, service_uid=f.service_uid):
                    self.fail("foreign target yielded")
            marker = self.directory / ".e2e-forward"
            marker.write_text("foreign")
            with self.assertRaises(target.AnchorError):
                with f.anchor.e2e_forward("iam", namespace_uid=f.namespace_uid, service_uid=f.service_uid):
                    self.fail("foreign marker adopted")
            self.assertEqual(marker.read_text(), "foreign")
            child.assert_not_called()

    def test_timeout_and_identity_changes_close_the_child(self):
        for mode in ("timeout", "before-spawn", "before-yield", "after-yield"):
            with self.subTest(mode=mode):
                f = Fixture(self.directory)
                identity = f.identity()
                responses = [identity] * 4
                if mode != "timeout":
                    index = {"before-spawn": 1, "before-yield": 2, "after-yield": 3}[mode]
                    responses[index] = {**identity, "pod_uid": uid()}
                f.anchor._e2e_forward_identity = Mock(side_effect=responses)
                child = Child("iam")
                child.fail_ready = mode == "timeout"
                with patch.object(target, "_E2EForward", return_value=child), patch.object(target, "_admin_marker_private"):
                    with self.assertRaises((target.AnchorError, TimeoutError)):
                        with f.anchor.e2e_forward("iam", namespace_uid=f.namespace_uid, service_uid=f.service_uid):
                            pass
                self.assertTrue(child.closed)
                self.assertFalse((self.directory / ".e2e-forward").exists())

    def test_cleanup_failure_retains_its_marker(self):
        f = Fixture(self.directory)
        child = Child("iam")
        child.fail_close = True
        with patch.object(target, "_E2EForward", return_value=child), patch.object(target, "_admin_marker_private"):
            with self.assertRaises(target.AnchorError):
                with f.anchor.e2e_forward("iam", namespace_uid=f.namespace_uid, service_uid=f.service_uid):
                    pass
        self.assertTrue((self.directory / ".e2e-forward").exists())

    def test_mapping_rejects_wrong_remote_public_bind_extra_connections_and_stderr(self):
        for service, (_, remote_port) in target._E2E_FORWARD_TARGETS.items():
            for mode in ("valid", "wrong remote", "public bind", "extra connections", "stderr"):
                with self.subTest(service=service, mode=mode):
                    child = target._E2EForward(service)
                    child.process = SimpleNamespace(poll=lambda: None)
                    text = f"Forwarding from 127.0.0.1:44111 -> {remote_port}\nHandling connection for 44111\n"
                    if mode == "wrong remote":
                        text = text.replace(str(remote_port), "9901")
                    elif mode == "public bind":
                        text = text.replace("127.0.0.1", "0.0.0.0")
                    elif mode == "extra connections":
                        text += "Handling connection for 44111\n"
                    child.output[0].extend(text.encode())
                    if mode == "stderr":
                        child.output[1].extend(b"failure")
                    if mode == "valid":
                        self.assertEqual(child.mapping(complete=True), 44111)
                    else:
                        with self.assertRaises(target.AnchorError):
                            child.mapping(complete=True)


if __name__ == "__main__":
    unittest.main()
