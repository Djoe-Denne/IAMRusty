"""Small offline boundary checks; real mTLS/push is verified separately."""
import io
import copy
import json
import os
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
import local_registry as registry


class RegistryTests(unittest.TestCase):
    def test_mtls_policy_separates_publisher_and_reader(self):
        config = json.loads((registry.DECLARATIONS / "config.json").read_text())
        policy = config["http"]["accessControl"]["repositories"]["apps/**"]
        self.assertEqual(policy["anonymousPolicy"], [])
        self.assertEqual(policy["defaultPolicy"], [])
        self.assertEqual(policy["policies"][1], {"users": ["registry-reader"], "actions": ["read"]})
        self.assertNotIn("delete", policy["policies"][0]["actions"])
        self.assertNotIn("ca.key", json.dumps(config))
        self.assertEqual(config["http"]["tls"]["cacert"], "/runtime/ca.crt")

    def test_archive_hashes_final_binary_not_lower_layer(self):
        def layer(files):
            data = io.BytesIO()
            with tarfile.open(fileobj=data, mode="w") as archive:
                for name, contents in files.items():
                    item = tarfile.TarInfo(name)
                    item.size = len(contents)
                    archive.addfile(item, io.BytesIO(contents))
            return data.getvalue()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "image.tar"
            payloads = {"one.tar": layer({"app/hive-service": b"old", "app/migration": b"old"}),
                        "two.tar": layer({"app/.wh..wh..opq": b"", "app/hive-service": b"new", "app/migration": b"migration"})}
            payloads["manifest.json"] = json.dumps([{"Layers": ["one.tar", "two.tar"]}]).encode()
            with tarfile.open(path, "w") as archive:
                for name, contents in payloads.items():
                    item = tarfile.TarInfo(name)
                    item.size = len(contents)
                    archive.addfile(item, io.BytesIO(contents))
            result = registry.artifact_hashes(path, "hive")
            self.assertEqual({r["binary"]: r["sha256"] for r in result}, {
                "hive-service": registry.digest(b"new"), "hivemigration": registry.digest(b"migration")})

    def test_remote_manifest_and_config_digests_verified_separately(self):
        source = {"RootFS": {"Layers": ["sha256:" + "a" * 64]}, "Config": {"Labels": {"source": "public"}}}
        config = json.dumps({"os": "linux", "architecture": "amd64", "rootfs": {"diff_ids": source["RootFS"]["Layers"]},
                             "config": source["Config"]}).encode()
        manifest = json.dumps({"mediaType": "application/vnd.oci.image.manifest.v1+json",
                               "config": {"digest": registry.digest(config)}}).encode()
        instance = registry.Registry.__new__(registry.Registry)
        instance.request = Mock(side_effect=[(200, manifest, registry.digest(manifest)), (200, config, None)])
        result = instance.remote("apps/hive", registry.digest(manifest), source)
        self.assertNotEqual(result["manifest_digest"], result["config_digest"])
        instance.request = Mock(return_value=(200, manifest, "sha256:" + "b" * 64))
        with self.assertRaises(ValueError):
            instance.remote("apps/hive", registry.digest(manifest), source)

    def test_foreign_container_refused_before_stop(self):
        instance = registry.Registry.__new__(registry.Registry)
        instance.state = {"owner": "a" * 32}
        instance.inspect = Mock(return_value={"Id": "b" * 64, "Config": {"Labels": {"aiforall.registry.owner": "foreign"}}})
        instance.docker = Mock()
        with self.assertRaises(ValueError):
            instance.stop_container("b" * 64)
        instance.docker.assert_not_called()


class ComposeOwnershipTests(unittest.TestCase):
    def instance(self, compose=True):
        value = registry.Registry.__new__(registry.Registry)
        value.state = {"owner": "a" * 32, "helpers": [], "context": "explicit", "port": 15443,
                       "cert_directory": "private-synthetic-not-real"}
        if compose:
            value.state["compose"] = {"receipts": {}}
        value.lock = json.loads(registry.LOCK.read_text())
        value.save = Mock()
        return value

    def transaction(self, *, fail=False, missing=False, occupied=False):
        value = self.instance()
        project = "aiforall-images-" + value.state["owner"]
        model = {"services": {"registry": {"container_name": project + "-registry", "image": value.lock["zot"]["reference"]}},
                 "networks": {"registry": {"name": project + "-network"}},
                 "volumes": {"data": {"name": project + "-data"}}}
        value.compose = Mock(return_value=(["docker", "compose"], {"REGISTRY_PROJECT": project}, model))
        items, events = {}, []
        for kind, name, identity in [("container", project + "-registry", "b" * 64),
                                     ("network", project + "-network", "c" * 64),
                                     ("volume", project + "-data", project + "-data")]:
            labels = {"aiforall.registry.owner": value.state["owner"], "com.docker.compose.project": project,
                      "com.docker.compose.service": "registry"}
            item = {"Id": identity, "Name": name, "Labels": labels, "Driver": "bridge" if kind == "network" else "local"}
            if kind == "container":
                item.update(Config={"Labels": labels}, HostConfig={}, Mounts=[], Image="pin", State={"Status": "created"})
            items[kind, identity] = item
            events.append({"Type": kind, "Action": "create", "Actor": {"ID": identity, "Attributes": {"name": name}}})
        def docker(args, *unused):
            if args[0] == "info":
                return "2026-11-09T00:00:00.000000001Z"
            if args[0] == "events":
                return "\n".join(json.dumps(e) for e in (events[:-1] if missing else events))
            if args[1] == "ls":
                return json.dumps({"Names": project + "-registry", "Labels": ""}) if occupied else ""
            self.fail("unexpected mutator: " + str(args[:2]))
        value.docker = Mock(side_effect=docker)
        value.inspect = Mock(side_effect=lambda kind, identity: items[kind, identity])
        value.service_check = Mock()
        value.image = Mock()
        self.addCleanup(patch.stopall)
        recorder = patch.object(registry, "record").start()
        execute = patch.object(registry, "run", side_effect=ValueError("partial") if fail else None).start()
        return value, execute, recorder

    def test_fresh_create_uses_no_recreate_and_records_before_opaque_creation(self):
        value, execute, recorder = self.transaction()
        order = []
        recorder.side_effect = lambda *a, **kw: order.append((a[1], kw.get("event", "created"), kw.get("safe")))
        execute.side_effect = lambda *a, **kw: order.append(("compose", a[0], None))
        value.compose_create()
        self.assertEqual(value.state["container"], "b" * 64)
        self.assertEqual([entry[0] for entry in order[:3]], ["UNKNOWN"] * 3)
        self.assertTrue(all(entry[2] == "ask" for entry in order[:3]))
        self.assertIn("--no-recreate", order[3][1])
        self.assertIn("--pull", order[3][1])
        self.assertTrue(value.state["compose"]["pending"], "creation ACK alone is not successful initialization")
        self.assertIn("container:" + "b" * 64, value.state["compose"]["receipts"])
        self.assertFalse(any("start" in call.args[0] for call in value.docker.call_args_list))

    def test_occupied_name_refuses_before_creation_even_with_matching_owner(self):
        value, execute, recorder = self.transaction(occupied=True)
        with self.assertRaisesRegex(ValueError, "adoption/recreation"):
            value.compose_create()
        execute.assert_not_called()
        recorder.assert_not_called()

    def test_partial_create_records_all_observed_ids_and_keeps_pending(self):
        value, execute, recorder = self.transaction(fail=True)
        with self.assertRaisesRegex(ValueError, "partial"):
            value.compose_create()
        self.assertTrue(value.state["compose"]["pending"])
        self.assertEqual(value.state["container"], "b" * 64)
        self.assertTrue(any(call.args[1] == "c" * 64 for call in recorder.call_args_list))
        with self.assertRaisesRegex(ValueError, "incomplete registry"):
            value.up(None, 15443)
        self.assertEqual(execute.call_count, 1)

    def test_missing_creation_event_is_not_an_adoption_receipt(self):
        value, execute, recorder = self.transaction(missing=True)
        with self.assertRaisesRegex(ValueError, "incomplete"):
            value.compose_create()
        self.assertNotIn("data", value.state)
        self.assertTrue(value.state["compose"]["pending"])

    def test_postflight_failure_still_logs_every_observed_creation_id(self):
        value, execute, recorder = self.transaction()
        value.inspect("container", "b" * 64)["Config"]["Labels"]["aiforall.registry.owner"] = "foreign"
        with self.assertRaisesRegex(ValueError, "identity mismatch"):
            value.compose_create()
        observed = {c.args[1] for c in recorder.call_args_list if c.kwargs.get("event") == "observed"}
        self.assertEqual(observed, {"b" * 64, "c" * 64, "aiforall-images-" + "a" * 32 + "-data"})
        self.assertNotIn("container", value.state)
        self.assertTrue(value.state["compose"]["pending"])

    def test_lost_ack_and_drift_refuse_stop_before_any_effect(self):
        for receipt in (None, "sha256:wrong"):
            with self.subTest(receipt=receipt):
                value = self.instance()
                value.state["container"] = "b" * 64
                value.inspect = Mock(return_value={"Id": "b" * 64, "Config": {"Labels": {"aiforall.registry.owner": value.state["owner"]}}})
                if receipt:
                    value.state["compose"]["receipts"]["container:" + "b" * 64] = receipt
                value.docker = Mock()
                with self.assertRaisesRegex(ValueError, "drift or lost"):
                    value.stop_container("b" * 64)
                value.docker.assert_not_called()

    def test_label_and_id_without_private_ack_refuse_stop(self):
        value = self.instance(False)
        value.inspect = Mock(return_value={"Id": "b" * 64, "Config": {"Labels": {"aiforall.registry.owner": value.state["owner"]}}})
        value.docker = Mock()
        with self.assertRaisesRegex(ValueError, "foreign"):
            value.stop_container("b" * 64)
        value.docker.assert_not_called()

    def test_legacy_reuse_never_invokes_compose_or_creates(self):
        for running in (True, False):
            with self.subTest(running=running):
                value = self.instance(False)
                value.state["container"] = "b" * 64
                value.check = Mock(return_value={"Id": "b" * 64, "State": {"Running": running}})
                value.certificates = Mock()
                value.docker = Mock()
                value.compose_create = Mock(side_effect=AssertionError("no adoption"))
                value.request = Mock(return_value=(200, b"", None))
                with patch.object(registry, "record"):
                    value.up(None, 15443)
                value.compose_create.assert_not_called()
                self.assertEqual(value.docker.call_count, int(not running))
                if not running:
                    value.docker.assert_called_once_with(["container", "start", "b" * 64])

    def test_legacy_missing_registry_id_is_not_recreated(self):
        value = self.instance(False)
        value.compose_create = Mock()
        with self.assertRaisesRegex(ValueError, "incomplete setup"):
            value.up(None, 15443)
        value.compose_create.assert_not_called()

    def test_changed_bootstrap_inputs_do_not_start_existing_id(self):
        value = self.instance(False)
        value.state["container"] = "b" * 64
        value.docker = Mock()
        with self.assertRaisesRegex(ValueError, "inputs changed"):
            value.up(None, 15444)
        value.docker.assert_not_called()

    def test_reader_403_is_failure_not_absence(self):
        value = self.instance()
        value.request = Mock(return_value=(403, b"PRIVATE_SENTINEL", None))
        with self.assertRaisesRegex(ValueError, "manifest identity mismatch") as raised:
            value.remote("apps/hive", "sha256:" + "b" * 64)
        self.assertNotIn("PRIVATE_SENTINEL", str(raised.exception))

    def test_compose_ignores_ambient_env_and_uses_only_private_path_references(self):
        value = self.instance()
        with patch.dict(os.environ, {"COMPOSE_FILE": "evil", "REGISTRY_CERTS": "SECRET_SENTINEL"}), patch.object(registry, "run", return_value='{"services":{}}') as execute:
            command, env, _ = value.compose()
        self.assertNotIn("COMPOSE_FILE", env)
        self.assertEqual(env["REGISTRY_CERTS"], "private-synthetic-not-real")
        self.assertEqual(command[command.index("--env-file") + 1], os.devnull)
        self.assertNotIn("SECRET_SENTINEL", str(execute.call_args))

    def test_declaration_drift_refuses_render_and_mutation(self):
        value = self.instance()
        value.state["compose"]["sources"] = {"config.json": "wrong"}
        with patch.object(registry, "run") as execute:
            with self.assertRaisesRegex(ValueError, "declarations changed"):
                value.compose()
        execute.assert_not_called()

    def test_service_postflight_compares_mounts_pin_permissions_and_command(self):
        value = self.instance()
        pin = value.lock["zot"]["reference"]
        service = {"image": pin, "user": "10001:10001", "cap_drop": ["ALL"], "network_mode": "none", "command": ["serve", "/runtime/config.json"],
                   "volumes": [{"type": "volume", "source": "runtime", "target": "/runtime", "read_only": True},
                               {"type": "bind", "source": "C:/synthetic/file", "target": "/input", "read_only": True}]}
        model = {"volumes": {"runtime": {"name": "owned-runtime"}}}
        item = {"Image": "sha256:pin", "Config": {"User": "10001:10001", "Cmd": service["command"]},
                "HostConfig": {"ReadonlyRootfs": True, "CapDrop": ["ALL"], "NetworkMode": "none", "SecurityOpt": ["no-new-privileges"]},
                "Mounts": [{"Type": "volume", "Name": "owned-runtime", "Destination": "/runtime", "RW": False},
                           {"Type": "bind", "Source": "/run/desktop/mnt/host/c/synthetic/file", "Destination": "/input", "RW": False}]}
        value.inspect = Mock(return_value={"Id": "sha256:pin", "RepoDigests": [pin]})
        value.service_check(item, service, model)
        for mutate in [lambda i: i["Mounts"][0].update(RW=True), lambda i: i["Mounts"][0].update(Name="foreign"),
                       lambda i: i["Mounts"][1].update(Source="/foreign/c/synthetic/file"),
                       lambda i: i["Config"].update(User="0:0"), lambda i: i["HostConfig"].update(ReadonlyRootfs=False),
                       lambda i: i["HostConfig"].update(CapAdd=["SYS_ADMIN"]),
                       lambda i: i["HostConfig"].update(Privileged=True), lambda i: i["HostConfig"].update(NetworkMode="host"),
                       lambda i: i["Config"].update(Cmd=["serve", "untrusted"]), lambda i: i.update(Image="foreign")]:
            changed = copy.deepcopy(item)
            mutate(changed)
            with self.assertRaisesRegex(ValueError, "differs from declaration"):
                value.service_check(changed, service, model)

    def test_failed_initialization_cannot_be_bypassed_by_existing_container(self):
        value = self.instance()
        value.state["container"] = "b" * 64
        value.state["compose"]["pending"] = True
        value.inspect = Mock()
        value.docker = Mock()
        with self.assertRaisesRegex(ValueError, "incomplete registry"):
            value.up(None, 15443)
        value.inspect.assert_not_called()
        value.docker.assert_not_called()

    def test_pending_publication_cannot_be_bypassed_by_new_transfer(self):
        value = self.instance()
        value.state["compose"].update(initialized=True, pending=True)
        value.docker = Mock()
        value.inspect = Mock()
        with self.assertRaisesRegex(ValueError, "incomplete registry operation"):
            value.copy({}, "apps/hive", Mock())
        value.inspect.assert_not_called()
        value.docker.assert_not_called()

    def test_network_and_volume_drift_fail_before_start(self):
        for kind in ("network", "volume"):
            value = self.instance()
            item = {"Name": "owned", "Labels": {"aiforall.registry.owner": value.state["owner"]},
                    "Driver": "bridge" if kind == "network" else "local"}
            if kind == "network":
                item["Id"] = "owned"
            value.state["compose"]["receipts"][kind + ":owned"] = registry.fingerprint(item)
            value.receipt(kind, "owned", item)
            for key, new in (("Labels", {}), ("Driver", "foreign"), ("Name", "foreign")):
                changed = {**item, key: new}
                with self.assertRaises(ValueError):
                    value.receipt(kind, "owned", changed)

    def test_port_collision_prevents_compose_creation(self):
        value = self.instance()
        value.directory = Path("synthetic-state")
        value.compose_create = Mock()
        value.image = Mock()
        with patch.object(registry.pki, "checked_path", return_value=Path("synthetic-certs")), \
             patch.object(registry.pki, "private_directory"), patch.object(registry.pki, "OpenSSL"), \
             patch.object(registry.pki, "openssl_path"), patch.object(registry.pki, "validate_registry"), \
             patch.object(registry.pki, "registry_profiles", return_value=[]), \
             patch.object(Path, "read_bytes", return_value=b"synthetic-public-ca"), \
             patch.object(registry.socket, "socket") as socket:
            socket.return_value.__enter__.return_value.bind.side_effect = OSError("occupied")
            with self.assertRaises(OSError):
                value.up(Path("synthetic-certs"), 15443)
        value.compose_create.assert_not_called()
        value.image.assert_not_called()

    def test_helper_lost_ack_or_replay_refuses_start(self):
        value = self.instance()
        value.owned = Mock(return_value={"State": {"Status": "exited", "ExitCode": 0}})
        value.docker = Mock()
        value.compose_create = Mock(side_effect=lambda *a, **kw: value.state["helpers"].append("b" * 64))
        value.stop_container = Mock()
        with self.assertRaisesRegex(ValueError, "replay refused"):
            value.helper()
        value.docker.assert_not_called()

    def test_fresh_up_waits_for_permission_success_before_registry_start(self):
        for failure in (False, True):
            with self.subTest(failure=failure):
                value = self.instance()
                value.directory = Path("synthetic-state")
                value.docker = Mock(return_value="")
                def create():
                    value.state["container"] = "b" * 64
                    return {"permissions": "c" * 64, "registry": "b" * 64}
                value.compose_create = Mock(side_effect=create)
                value.check = Mock()
                value.request = Mock(return_value=(200, b"", None))
                value.certificates = Mock()
                def permission():
                    value.state["compose"]["pending"] = True
                    self.assertFalse(any("start" in c.args[0] for c in value.docker.call_args_list))
                    if failure:
                        raise ValueError("permission failure")
                    value.state["container"] = "b" * 64
                value.helper = Mock(side_effect=permission)
                with patch.object(registry.pki, "checked_path", return_value=Path("synthetic-certs")), \
                     patch.object(registry.pki, "private_directory"), patch.object(registry.pki, "OpenSSL"), \
                     patch.object(registry.pki, "openssl_path"), patch.object(registry.pki, "validate_registry"), \
                     patch.object(registry.pki, "registry_profiles", return_value=[]), \
                     patch.object(Path, "read_bytes", return_value=b"synthetic-public-ca"), \
                     patch.object(registry.socket, "socket"):
                    if failure:
                        with self.assertRaisesRegex(ValueError, "permission failure"):
                            value.up(Path("synthetic-certs"), 15443)
                        self.assertTrue(value.state["compose"]["pending"])
                    else:
                        value.up(Path("synthetic-certs"), 15443)
                        self.assertFalse(value.state["compose"]["pending"])
                starts = [c.args[0] for c in value.docker.call_args_list if "start" in c.args[0]]
                self.assertEqual(starts, [] if failure else [["container", "start", "b" * 64]])


class LegacyTransportTests(unittest.TestCase):
    """Engine/Compose protocol fake: production guards, ACK, lifecycle and OCI reader run unchanged."""
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.value = ComposeOwnershipTests.instance(self, False)
        v = self.value
        v.directory = Path(self.temp.name)
        v.state.update(container="b" * 64, network="c" * 64, data="old-data", runtime="old-runtime")
        self.owner = {"aiforall.registry.owner": v.state["owner"]}
        self.images = {v.lock[k]["reference"]: {"Id": "sha256:" + k, "RepoDigests": [v.lock[k]["reference"]]} for k in ("zot", "skopeo")}
        self.items = {
            ("container", v.state["container"]): {"Id": v.state["container"], "Name": "/old-registry", "Image": "sha256:zot",
                "Config": {"Labels": self.owner.copy(), "User": "10001:10001"},
                "HostConfig": {"PortBindings": {"5000/tcp": [{"HostIp": "127.0.0.1", "HostPort": "15443"}]}},
                "Mounts": [{"Type": "volume", "Name": name} for name in ("old-data", "old-runtime")],
                "State": {"Running": True, "Status": "running", "ExitCode": 0}},
            ("network", v.state["network"]): {"Id": v.state["network"], "Name": "old-network", "Driver": "bridge",
                "Labels": self.owner.copy(), "Containers": {v.state["container"]: {"Name": "old-registry"}}}}
        self.items.update({("volume", name): {"Name": name, "Driver": "local", "Labels": self.owner.copy()} for name in ("old-data", "old-runtime")})
        self.local = {"Id": "sha256:local", "RootFS": {"Layers": ["sha256:layer"]}, "Config": {"User": "10001:10001"}}
        self.config = json.dumps({"os": "linux", "architecture": "amd64", "rootfs": {"diff_ids": self.local["RootFS"]["Layers"]}, "config": self.local["Config"]}).encode()
        self.manifest = json.dumps({"mediaType": "application/vnd.oci.image.manifest.v1+json", "config": {"digest": registry.digest(self.config)}}).encode()
        self.expected_digest = registry.digest(self.manifest)
        self.events, self.saved, self.actions, self.models = [], [], [], []
        self.fail_create = self.fail_run = self.fail_stop = self.bad_external = self.bad_digest = False
        self.start_error = ValueError("helper execution failed")
        self.allow_registry_lifecycle = False
        self.after_create = lambda: None
        v.save.side_effect = lambda: self.saved.append(copy.deepcopy(v.state))
        v.docker = Mock(side_effect=self.docker)
        v.image = Mock()
        v.certificates = Mock()
        v.request = Mock(side_effect=lambda path, **kw: (200, self.manifest, "sha256:wrong" if self.bad_digest else self.expected_digest)
                         if "/manifests/" in path else (200, self.config, None))
        self.record = patch.object(registry, "record").start()
        self.execute = patch.object(registry, "run", side_effect=self.compose_run).start()
        self.private_guard = patch.object(registry.pki, "private_directory", side_effect=lambda path, create=False:
            path.mkdir(mode=0o777 if os.name == "nt" else 0o700) if create else None)
        self.private_guard.start()
        self.addCleanup(patch.stopall)

    @property
    def network(self):
        return self.items["network", self.value.state["network"]]

    def model(self, env):
        mounts = [{"type": "bind", "source": env[key], "target": target, "read_only": ro} for key, target, ro in
                  (("REGISTRY_INPUTS", "/inputs", True), ("REGISTRY_OUTPUTS", "/outputs", False), ("REGISTRY_CERTS", "/certs", True))]
        mounts += [{"type": "bind", "source": str(registry.DECLARATIONS / name), "target": "/scripts/" + name, "read_only": True}
                   for name in ("publish.sh", "policy.json")]
        return {"services": {"publisher": {"container_name": env["REGISTRY_PROJECT"] + "-publisher",
                    "image": self.value.lock["skopeo"]["reference"], "user": env["REGISTRY_USER"],
                    "entrypoint": ["/bin/sh", "/scripts/publish.sh"], "cap_drop": ["ALL"], "networks": {"registry": {}}, "volumes": mounts}},
                "networks": {"registry": {"external": not self.bad_external, "name": env["REGISTRY_NETWORK"]}}}

    def compose_run(self, args, *unused, env=None, **kwargs):
        self.assertEqual(args[args.index("-f") + 1], str(registry.DECLARATIONS / "publish.compose.yaml"))
        if "config" in args:
            self.current_model = self.model(env)
            self.models.append(copy.deepcopy(self.current_model))
            return json.dumps(self.current_model)
        self.assertIn("--no-recreate", args)
        self.assertIn("--no-build", args)
        self.assertEqual(args[-2:], ["--pull", "never"])
        service = self.current_model["services"]["publisher"]
        identity = format(len(self.events) + 10, "064x")
        labels = {**self.owner, "com.docker.compose.project": env["REGISTRY_PROJECT"],
                  "com.docker.compose.service": "publisher", "aiforall.registry.role": "publisher"}
        self.items["container", identity] = {"Id": identity, "Name": "/" + service["container_name"], "Image": "sha256:skopeo",
            "Config": {"Labels": labels, "User": service["user"], "Entrypoint": service["entrypoint"]},
            "HostConfig": {"ReadonlyRootfs": True, "CapDrop": ["ALL"], "SecurityOpt": ["no-new-privileges"], "NetworkMode": env["REGISTRY_NETWORK"]},
            "Mounts": [{"Type": "bind", "Source": m["source"], "Destination": m["target"], "RW": not m["read_only"]} for m in service["volumes"]],
            "State": {"Running": False, "Status": "created", "ExitCode": 0}}
        self.events.append({"Type": "container", "Action": "create", "Actor": {"ID": identity, "Attributes": {"name": service["container_name"]}}})
        self.actions.append(("create", identity))
        self.after_create()
        if self.fail_create:
            raise ValueError("partial transport create")
        return ""

    def docker(self, args, *unused):
        v = self.value
        if args[0] == "info":
            return "2026-11-09T00:00:00.000000001Z"
        if args[0] == "events":
            return "\n".join(json.dumps(e) for e in self.events)
        if args[:2] == ["image", "save"]:
            Path(args[3]).write_bytes(b"SYNTHETIC_ARCHIVE")
            self.actions.append(("export", args[-1]))
            return ""
        if args[1] == "inspect":
            return json.dumps([self.images[args[2]] if args[0] == "image" else self.items[args[0], args[2]]])
        if args[1] == "ls":
            rows = [{"Names" if kind == "container" else "Name": item["Name"].lstrip("/"),
                     "Labels": ",".join(k + "=" + value for k, value in (item.get("Config", {}).get("Labels") or item.get("Labels", {})).items())}
                    for (kind, _), item in self.items.items() if kind == args[0]]
            return "\n".join(json.dumps(row) for row in rows)
        self.assertEqual(args[0], "container")
        identity = args[-1]
        if identity == v.state["container"] and self.allow_registry_lifecycle:
            self.actions.append((args[1], identity))
            self.items["container", identity]["State"].update(Running=args[1] == "start")
            if args[1] == "start":
                self.network["Containers"][identity] = {"Name": "old-registry"}
            else:
                self.network["Containers"].pop(identity, None)
            return ""
        self.assertNotEqual(identity, v.state["container"], "transport must never start/stop the registry")
        item = self.items["container", identity]
        if args[1] == "start":
            self.assertEqual(self.saved[-1]["transport"]["receipts"]["container:" + identity], registry.fingerprint(item), "ACK must be durable before start")
            self.actions.append(("start", identity))
            item["State"].update(Running=True, Status="running")
            self.network["Containers"][identity] = {}
            if self.fail_run:
                raise self.start_error
            outputs = next(m["Source"] for m in item["Mounts"] if m["Destination"] == "/outputs")
            (Path(outputs) / "digest").write_text(self.expected_digest)
        elif args[1] == "stop":
            self.actions.append(("stop", identity))
            if self.fail_stop:
                raise ValueError("helper stop failed")
        else:
            self.fail("unexpected Docker command " + str(args[:2]))
        item["State"].update(Running=False, Status="exited")
        self.network["Containers"].pop(identity, None)
        return ""

    def publish(self):
        archive = Mock(side_effect=lambda p: self.assertEqual(p.read_bytes(), b"SYNTHETIC_ARCHIVE"))
        result = self.value.copy(self.local, "apps/hive", archive)
        archive.assert_called_once()
        return result

    def test_two_legacy_publications_use_new_helpers_and_preserve_persistent_identities(self):
        before = copy.deepcopy(self.items)
        self.assertEqual(self.publish()["manifest_digest"], self.expected_digest)
        self.assertEqual(self.publish()["manifest_digest"], self.expected_digest)
        self.assertNotIn("compose", self.value.state)
        helpers = self.value.state["transport"]["helpers"]
        self.assertEqual(len(set(helpers)), 2)
        self.assertEqual([identity for action, identity in self.actions if action == "start"], helpers)
        for key, item in before.items():
            self.assertEqual(self.items[key], item)
        for helper in helpers:
            self.assertFalse(self.items["container", helper]["State"]["Running"])
            mounts = self.items["container", helper]["Mounts"]
            self.assertEqual({m["Destination"] for m in mounts if m["RW"]}, {"/outputs"})
            self.assertFalse(any(m["Type"] == "volume" or "docker.sock" in m["Destination"] for m in mounts))

    def native_private_staging(self, compose):
        # Keep Registry.copy and its actual recursive owner/SID/mode validator.
        # Only Engine/OCI boundaries stay simulated; all bytes are synthetic.
        self.private_guard.stop()
        self.assertNotIsInstance(registry.pki.private_directory, Mock)
        v = self.value
        certs = v.directory / "synthetic-certificates"
        registry.pki.private_directory(certs, create=True)
        for path in (certs / "mesh", certs / "extensions", certs / "extensions/registry"):
            path.mkdir(mode=0o777 if os.name == "nt" else 0o700)
        files = {"ca": certs / "mesh/ca.crt", **{name: certs / "extensions/registry" / (name + ".crt")
                 for name in registry.pki.registry_profiles()}}
        for path in files.values():
            path.write_bytes(b"SYNTHETIC_PUBLIC_CERTIFICATE")
            path.chmod(0o600)
        v.state.update(cert_directory=str(certs), certificates={name: registry.digest(path.read_bytes()) for name, path in files.items()})
        v.certificates = registry.Registry.certificates.__get__(v)
        if compose:
            v.state["compose"] = {"receipts": {}}
            v.declarations(v.state["compose"])
            v.state["compose"]["receipts"] = {kind + ":" + identity: registry.fingerprint(item)
                for (kind, identity), item in self.items.items()}
        staging = []
        def inspect_staging():
            helper = self.items["container", self.events[-1]["Actor"]["ID"]]
            mounts = {m["Destination"]: m for m in helper["Mounts"]}
            inputs, outputs = Path(mounts["/inputs"]["Source"]), Path(mounts["/outputs"]["Source"])
            self.assertEqual(inputs.parent, outputs.parent)
            self.assertEqual({m["Destination"] for m in helper["Mounts"] if m["RW"]}, {"/outputs"})
            self.assertTrue((outputs / "tmp").is_dir())
            self.assertEqual((inputs / "image.tar").read_bytes(), b"SYNTHETIC_ARCHIVE")
            self.assertTrue((inputs / "destination").read_text().startswith("docker://" + registry.AUTHORITY + "/apps/hive:"))
            staging.append(inputs.parent)
        self.after_create = inspect_staging
        for _ in range(2):
            self.assertEqual(self.publish()["manifest_digest"], self.expected_digest)
        self.assertEqual(len(set(staging)), 2)
        self.assertTrue(all(not path.exists() for path in staging))
        self.assertEqual(len(set(v.state["transport"]["helpers"])), 2)
        self.assertEqual("compose" in v.state, compose)

    def test_real_private_staging_legacy_two_publications(self):
        self.native_private_staging(compose=False)

    def test_real_private_staging_compose_two_publications(self):
        self.native_private_staging(compose=True)

    def test_legacy_up_status_stop_do_not_create_or_adopt(self):
        v = self.value
        v.check()
        v.up(None, 15443)
        self.assertEqual(self.actions, [])
        self.allow_registry_lifecycle = True
        v.stop_container(v.state["container"])
        v.up(None, 15443)
        self.assertEqual(self.actions, [("stop", "b" * 64), ("start", "b" * 64)])
        self.assertFalse(self.events)
        self.assertFalse(self.models)
        self.assertNotIn("compose", v.state)
        self.assertNotIn("transport", v.state)
        self.assertEqual((v.state["network"], v.state["data"], v.state["runtime"]), ("c" * 64, "old-data", "old-runtime"))

    def test_helper_execution_failure_stops_only_owned_helper(self):
        self.fail_run = True
        with self.assertRaisesRegex(ValueError, "execution failed"):
            self.publish()
        helper = self.value.state["transport"]["helpers"][0]
        self.assertIn(("stop", helper), self.actions)
        self.assertTrue(self.items["container", self.value.state["container"]]["State"]["Running"])

    def test_partial_creation_is_logged_acknowledged_not_started_and_not_replayed(self):
        self.fail_create = True
        with self.assertRaisesRegex(ValueError, "partial transport"):
            self.publish()
        self.assertTrue(self.value.state["transport"]["pending"])
        helper = self.value.state["transport"]["helpers"][0]
        self.assertFalse(any(action == "start" for action, _ in self.actions))
        self.assertTrue(any(c.args[1] == "UNKNOWN" and c.kwargs.get("safe") == "ask" for c in self.record.call_args_list))
        self.assertTrue(any(c.args[1] == helper and c.kwargs.get("event") == "stopped" for c in self.record.call_args_list))
        count = len(self.actions)
        with self.assertRaisesRegex(ValueError, "incomplete"):
            self.publish()
        self.assertEqual(len(self.actions), count)

    def test_stop_failure_is_logged_and_blocks_next_operation(self):
        self.fail_run = self.fail_stop = True
        with self.assertRaisesRegex(ValueError, "stop failed"):
            self.publish()
        self.assertTrue(self.value.state["transport"]["pending"])
        self.assertTrue(any(c.kwargs.get("event") == "stop-failed" for c in self.record.call_args_list))

    def test_foreign_member_or_wrong_network_id_refuses_before_export(self):
        for invalid in ("member", "id"):
            with self.subTest(invalid=invalid):
                original = copy.deepcopy(self.network)
                if invalid == "member":
                    self.network["Containers"]["d" * 64] = {}
                else:
                    self.network["Id"] = "d" * 64
                with self.assertRaises(ValueError):
                    self.publish()
                self.assertEqual(self.actions, [])
                self.items["network", self.value.state["network"]] = original

    def test_transport_model_may_not_create_the_existing_network(self):
        self.bad_external = True
        with self.assertRaisesRegex(ValueError, "verified external network"):
            self.publish()
        self.assertFalse(self.events)

    def test_postcreate_network_drift_blocks_start_and_future_transport(self):
        self.after_create = lambda: self.network.update(Driver="foreign")
        with self.assertRaisesRegex(ValueError, "changed during transport"):
            self.publish()
        self.assertFalse(any(action == "start" for action, _ in self.actions))
        self.assertTrue(self.value.state["transport"]["pending"])

    def test_lost_transport_ack_refuses_before_next_export(self):
        self.publish()
        ledger = copy.deepcopy(self.value.state["transport"])
        count = len(self.actions)
        for missing in ("receipt", "ledger"):
            with self.subTest(missing=missing):
                self.value.state["transport"] = copy.deepcopy(ledger)
                if missing == "receipt":
                    self.value.state["transport"]["receipts"].clear()
                else:
                    self.value.state.pop("transport")
                with self.assertRaisesRegex(ValueError, "ACK"):
                    self.publish()
                self.assertEqual(len(self.actions), count)

    def test_helper_timeout_stops_only_owned_helper(self):
        self.fail_run = True
        self.start_error = registry.subprocess.TimeoutExpired(["docker"], 1, output=b"PRIVATE_SENTINEL")
        with self.assertRaises(registry.subprocess.TimeoutExpired):
            self.publish()
        helper = self.value.state["transport"]["helpers"][0]
        self.assertIn(("stop", helper), self.actions)
        self.assertFalse(self.items["container", helper]["State"]["Running"])

    def test_ack_lost_between_creation_and_start_is_not_rebuilt_by_inspect(self):
        def lose_ack():
            self.saved.append(copy.deepcopy(self.value.state))
            ledger = self.value.state.get("transport", {})
            if ledger.get("helpers") and not ledger.get("pending"):
                ledger["receipts"].clear()
        self.value.save.side_effect = lose_ack
        with self.assertRaisesRegex(ValueError, "ACK"):
            self.publish()
        self.assertFalse(any(action == "start" for action, _ in self.actions))
        self.assertTrue(self.value.state["transport"]["pending"])

    def test_occupied_transport_project_is_not_adopted(self):
        from types import SimpleNamespace
        suffix = "d" * 32
        project = "aiforall-images-" + self.value.state["owner"] + "-" + suffix
        self.items["container", "e" * 64] = {"Name": "/foreign-container", "Config": {"Labels": {"com.docker.compose.project": project}}}
        with patch.object(registry.uuid, "uuid4", return_value=SimpleNamespace(hex=suffix)):
            with self.assertRaisesRegex(ValueError, "adoption/recreation"):
                self.publish()
        self.assertFalse(self.events)

    def test_external_network_name_must_resolve_to_verified_id(self):
        original = self.model
        def changed(env):
            model = original(env)
            model["networks"]["registry"]["name"] = "foreign-network"
            return model
        with patch.object(self, "model", side_effect=changed):
            with self.assertRaisesRegex(ValueError, "verified external network"):
                self.publish()
        self.assertFalse(self.events)

    def test_bad_remote_digest_is_rejected_after_helper_stop(self):
        self.bad_digest = True
        with self.assertRaisesRegex(ValueError, "manifest identity mismatch"):
            self.publish()
        self.assertTrue(all(not self.items["container", identity]["State"]["Running"] for identity in self.value.state["helpers"]))

    def test_pki_or_declaration_drift_blocks_new_transfer(self):
        self.publish()
        count = len(self.actions)
        self.value.certificates.side_effect = ValueError("registry certificates changed")
        with self.assertRaisesRegex(ValueError, "certificates changed"):
            self.publish()
        self.value.certificates.side_effect = None
        self.value.state["transport"]["sources"]["publish.sh"] = "drift"
        with self.assertRaisesRegex(ValueError, "declarations changed"):
            self.publish()
        self.assertEqual(len(self.actions), count)


if __name__ == "__main__":
    unittest.main()
