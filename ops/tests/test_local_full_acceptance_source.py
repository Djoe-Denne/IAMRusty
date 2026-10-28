"""Pure source/fake-exec checks; no Docker, Kubernetes, sockets or Secrets."""
import ast
import argparse
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import tomllib
import unittest
from unittest.mock import Mock, patch

SOURCE = Path(__file__).with_name("local-full-acceptance.py")


class AcceptanceGuardTests(unittest.TestCase):
    def test_linux_delivery_build_keeps_24_jobs_and_its_own_target(self):
        root = SOURCE.parents[2]
        config = tomllib.loads((root / ".cargo/config.toml").read_text(encoding="utf-8"))
        self.assertEqual(config["build"]["jobs"], 24)
        self.assertEqual(config["build"]["target-dir"], r"E:\cargo-target\AIForAll")
        build = (root / "Dockerfile.build").read_text(encoding="utf-8")
        self.assertIn("ENV CARGO_TARGET_DIR=/usr/src/target", build)
        self.assertIn("target=/usr/src/target", build)
        self.assertIn("find /usr/src/target/release", build)
        self.assertIn("cargo build --release --locked", build)
        self.assertNotIn("CARGO_BUILD_JOBS", build)

    def load(self):
        spec = importlib.util.spec_from_file_location("acceptance_guard_test", SOURCE)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_no_direct_process_or_ambient_kubectl_path(self):
        tree = ast.parse(SOURCE.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                self.assertFalse(any(alias.name in {"subprocess", "os"} for alias in node.names))
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and node.func.attr == "kubectl":
                self.assertIsInstance(node.func.value, ast.Name)
                self.assertEqual(node.func.value.id, "runtime")

    def test_remote_timeout_is_reported_not_replaced_by_raw_host_exec(self):
        module = self.load()
        with patch.object(module.runtime, "kubectl", return_value="28 000") as fake:
            self.assertEqual(module.guarded_curl("aiforall-local-full", "owned-probe", ["https://example.test"], 20), (28, "000"))
        self.assertEqual(fake.call_args.args[:7], ("exec", "-n", "aiforall-local-full", "owned-probe", "--", "sh", "-c"))

    def test_anchor_failure_never_falls_back_to_another_transport(self):
        module = self.load()
        with patch.object(module.runtime, "kubectl", side_effect=module.anchor_module.AnchorError("fake changed endpoint")) as fake:
            with self.assertRaises(module.anchor_module.AnchorError):
                module.guarded_curl("aiforall-local-full", "owned-probe", [], 20)
        self.assertEqual(fake.call_count, 1)

    def test_legacy_context_rejected_before_lease_or_api(self):
        module = self.load()
        args = Mock(context="kind-aiforall-local")
        with patch.object(module.runtime, "lease") as lease:
            with self.assertRaises(ValueError):
                module.run(args)
        lease.assert_not_called()

    def environment(self, root):
        return {"version": 1, "profile": "local-full", "context": "kind-aiforall-local-full",
                "cluster": "aiforall-local-full", "lease_path": str(root / "lease.json"),
                "namespace": "aiforall-local-full", "apparatus_namespace": "aiforall-local-apparatus",
                "plugins_namespace": "aiforall-local-plugins", "base_url": "http://127.0.0.1:18080"}

    def test_environment_schema_valid_and_rejects_unknown_foreign_or_contradictory(self):
        module = self.load()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "environment.json"
            value = self.environment(Path(directory))
            path.write_text(json.dumps(value))
            self.assertEqual(module.read_environment(path), value)
            for key, bad in [("unknown", "ignored"), ("version", True), ("version", 2),
                             ("profile", "legacy"), ("context", "kind-aiforall-local"),
                             ("context", "kind-apparatus-p4-it"), ("cluster", "foreign"),
                             ("namespace", "aiforall-platform"), ("plugins_namespace", "foreign"),
                             ("apparatus_namespace", "foreign"), ("lease_path", "relative.json"),
                             ("public_ca_path", "relative.pem"), ("base_url", "http://localhost:18080"),
                             ("base_url", "http://127.0.0.1:18080@foreign.test"),
                             ("base_url", "https://127.0.0.1:18080"), ("base_url", "http://127.0.0.1:8080")]:
                with self.subTest(key=key, bad=bad):
                    path.write_text(json.dumps({**value, key: bad}))
                    with self.assertRaises(ValueError):
                        module.read_environment(path)
            path.write_text(json.dumps(value)[:-1] + ',"base_url":"http://127.0.0.1:18080"}')
            with self.assertRaises(ValueError):
                module.read_environment(path)
            for content in ["NaN", "[]", "{}"]:
                path.write_text(content)
                with self.assertRaises(ValueError):
                    module.read_environment(path)

    def test_environment_rejects_even_identical_legacy_overrides(self):
        module = self.load()
        for key, value in [("lease", Path("lease.json")), ("namespace", "aiforall-local-full"),
                           ("context", module.CONTEXT)]:
            args = argparse.Namespace(environment=Path("environment.json"), lease=None, namespace=None,
                                      context=None, action="smoke")
            setattr(args, key, value)
            with patch.object(module, "read_environment") as read:
                with self.assertRaises(ValueError):
                    module.bind_environment(args)
                read.assert_not_called()

    def test_legacy_guard_bodies_and_baseline_counts_preserved(self):
        baseline = json.loads(SOURCE.with_name("local-full-acceptance-legacy-baseline.json").read_text())
        self.assertEqual(baseline["head"], "427416b")
        functions = {n.name: n for n in ast.parse(SOURCE.read_text()).body if isinstance(n, ast.FunctionDef)}
        for name, counts in baseline["functions"].items():
            with self.subTest(function=name):
                function = functions[name]
                self.assertGreaterEqual(sum(isinstance(n, ast.Raise) for n in ast.walk(function)), counts["raise"])
                self.assertGreaterEqual(sum(isinstance(n, ast.If) for n in ast.walk(function)), counts["if"])
                if name != "main":
                    self.assertEqual(hashlib.sha256(ast.dump(function, include_attributes=False).encode()).hexdigest(), counts["sha256"])
        self.assertIn("operation_lock", ast.unparse(functions["main"]))

    def test_smoke_is_get_wait_only_and_proves_each_component(self):
        module = self.load()
        args = argparse.Namespace(namespace="aiforall-local-full", executed=0)
        deployments = [(args.namespace, name) for name in module.SMOKE_DEPLOYMENTS] + [
            (module.NAMESPACES["apparatus_namespace"], "apparatus-operator")]
        data = {"namespaceUids": {name: name + "-uid" for name in module.NAMESPACES.values()},
                "ownedResources": [{"kind": "Deployment", "name": name, "namespace": namespace,
                                    "uid": namespace + name} for namespace, name in deployments]}
        def command(*parts):
            self.assertIn(parts[0], {"get", "wait"})
            if parts[0] == "wait":
                return "condition met"
            if parts[1] == "namespace":
                return json.dumps({"metadata": {"uid": parts[2] + "-uid"}})
            if parts[1] == "pvc":
                return json.dumps({"items": [{"status": {"phase": "Bound"}}]})
            self.assertEqual(parts[1], "Deployment")
            namespace = parts[parts.index("-n") + 1]
            return json.dumps({"metadata": {"uid": namespace + parts[2], "generation": 1},
                               "spec": {"replicas": 1}, "status": {"observedGeneration": 1,
                               "availableReplicas": 1, "readyReplicas": 1, "updatedReplicas": 1}})
        with patch.object(module.runtime, "kubectl", side_effect=command) as fake:
            module.smoke(args, data)
        self.assertEqual(args.executed, len(module.NAMESPACES) + len(deployments) + 1)
        self.assertEqual(sum(call.args[0] == "wait" for call in fake.call_args_list), len(deployments))
        with patch.object(module.runtime, "kubectl", side_effect=command):
            with self.assertRaises(ValueError):
                module.smoke(args, {**data, "ownedResources": []})
        tree = ast.parse(SOURCE.read_text())
        for function in [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name in {"smoke", "public_route_matrix", "route_matrix"}]:
            self.assertFalse(any(isinstance(n, ast.Call) and isinstance(n.func, ast.Name) and
                                 n.func.id == "explicit_probe_action" for n in ast.walk(function)))

    def test_public_route_matrix_rechecks_anchor_and_never_redirects_or_uses_legacy_transport(self):
        module = self.load()
        args = argparse.Namespace(namespace="aiforall-local-full", executed=0,
                                  route_cases=[{"id": "jwks", "method": "GET", "path": "/iam/.well-known/jwks.json", "statuses": [303]}])
        connection = Mock()
        connection.getresponse.return_value.status = 303
        with patch.object(module, "namespace_owned"), patch.object(module.runtime, "target") as target, patch.object(
                module.http.client, "HTTPConnection", return_value=connection) as http:
            module.public_route_matrix(args, {})
            target.return_value.check.assert_called_once_with()
            http.assert_called_once_with("127.0.0.1", 18080, timeout=20)
            connection.request.assert_called_once_with("GET", "/iam/.well-known/jwks.json", body=None, headers={})
            connection.close.assert_called_once_with()
        self.assertEqual(args.executed, 1)
        with patch.object(module, "namespace_owned"), patch.object(module.runtime, "target") as target, patch.object(
                module.http.client, "HTTPConnection") as http:
            target.return_value.check.side_effect = module.anchor_module.AnchorError("private payload")
            with self.assertRaises(module.anchor_module.AnchorError):
                module.public_route_matrix(args, {})
            http.assert_not_called()

    def test_route_cases_reject_hidden_bootstrap_credentials_and_foreign_paths(self):
        module = self.load()
        case = {"id": "check", "method": "GET", "path": "/iam/api/me", "statuses": [401]}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "cases.json"
            for change in [{"path": "http://foreign/iam/api/me"}, {"path": "//foreign/iam/api/me"},
                           {"path": "/iam/api/me\r\nInjected: secret"}, {"json": {"password": "secret"}},
                           {"seed": "private SQL"}, {"headers": {"Authorization": "secret"}}, {"statuses": [True]}]:
                path.write_text(json.dumps([{**case, **change}]))
                with self.assertRaises(ValueError):
                    module.public_cases(path)

    def test_route_cases_coherent_with_deployed_full_profile(self):
        module = self.load()
        root = SOURCE.resolve().parents[2]
        config = tomllib.loads((root / "ops/deploy/apps/overlays/local-full/config/iam.toml").read_text())
        self.assertEqual(config["idp"]["connectors"], [])
        full = module.public_cases(SOURCE.with_name("mesh-public-route-cases-local-full.json"))
        connected = module.public_cases(SOURCE.with_name("mesh-public-route-cases.json"))
        login_paths = {"/iam/api/auth/github/login", "/iam/api/auth/gitlab/login"}
        # These are public OAuth entrypoints; the unknown-provider denial is not a login positive.
        logins = [case for case in full if case["path"] in login_paths]
        self.assertEqual({case["path"] for case in logins}, login_paths)
        for case in full:
            if case["path"].endswith("/login") and case["method"] in {"GET", "HEAD"}:
                self.assertNotIn(303, case["statuses"], "redirect requires a configured connector")
        self.assertEqual([case["statuses"] for case in logins], [[422], [422]])
        self.assertEqual([case["statuses"] for case in connected if case["path"] in login_paths], [[303], [303]])
        disabled_relinks = {"/iam/api/auth/github/relink-callback?code=fixed-invalid-test-code",
                            "/iam/api/auth/gitlab/relink-callback?code=fixed-invalid-test-code"}
        self.assertEqual([case["statuses"] for case in full if case["path"] in disabled_relinks], [[401, 403], [401, 403]])
        self.assertEqual([case["statuses"] for case in connected if case["path"] in disabled_relinks], [[400], [400]])
        self.assertEqual([case for case in full if case["path"] not in login_paths | disabled_relinks],
                         [case for case in connected if case["path"] not in login_paths | disabled_relinks])
        args = argparse.Namespace(namespace="aiforall-local-full", executed=0, route_cases=logins,
                                  action="route-matrix", environment=Path("environment.json"))
        connection = Mock()
        connection.getresponse.return_value.status = 422  # Handler oracle with connectors=[], not case-derived.
        with patch.object(module, "namespace_owned"), patch.object(module.runtime, "target"), patch.object(
                module.http.client, "HTTPConnection", return_value=connection):
            module.public_route_matrix(args, {})
            self.assertEqual((args.executed, args.oauth_login_unavailable), (2, 2))
            self.assertEqual(module.public_coverage_report(args, "PASS", "LIVE")["oauth_login_unavailable"], 2)
            connection.getresponse.return_value.status = 303
            with self.assertRaisesRegex(AssertionError, "route matrix status mismatch"):
                module.public_route_matrix(args, {})

    def test_public_pass_never_claims_authenticated_coverage(self):
        module = self.load()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cases = root / "cases.json"
            cases.write_text(json.dumps([{"id": "jwks", "method": "GET",
                                         "path": "/iam/.well-known/jwks.json", "statuses": [200]}]))
            for status, code, public_status in [(200, 0, "PASS"), (401, 1, "FAIL")]:
                with self.subTest(status=status):
                    output = io.StringIO()
                    connection = Mock()
                    connection.getresponse.return_value.status = status
                    with patch("sys.argv", [str(SOURCE), "route-matrix", "--environment", "environment.json",
                                            "--cases", str(cases)]), patch.object(
                            module, "read_environment", return_value=self.environment(root)), patch.object(
                            module.runtime, "operation_lock", return_value=contextlib.nullcontext()), patch.object(
                            module.runtime, "lease", return_value={}), patch.object(module, "namespace_owned"), patch.object(
                            module.runtime, "target"), patch.object(module.http.client, "HTTPConnection", return_value=connection), contextlib.redirect_stdout(output):
                        self.assertEqual(module.main(), code)
                    report = json.loads(output.getvalue())
                    self.assertEqual(report["coverage"]["public"], {"status": public_status, "evidence": "LIVE"})
                    self.assertEqual(report["coverage"]["authenticated"], {
                        "status": "NOT RUN", "setup": "BLOCKED", "channel": "e2e --suite --inputs",
                        "reason": "approved full authenticated setup unavailable"})
                    self.assertEqual(report["oauth_login_unavailable"], 0)

    def test_kind_overlay_cannot_relink_validation_probe(self):
        root = SOURCE.resolve().parents[2]
        overlay = root / "ops/deploy/apps/overlays/kind/kustomization.yaml"
        self.assertNotIn("invoke-probe", overlay.read_text())
        self.assertTrue((root / "ops/tests/manifests/invoke-probe-job.yaml").is_file())

    def test_openfga_fallback_uses_contract_mount_and_pinned_cli(self):
        root = SOURCE.resolve().parents[2]
        recipe = (root / "justfile").read_text().split("regenerate-openfga-model-json:\n", 1)[1].split("\n\n", 1)[0]
        fallback = recipe.split("} else {", 1)[1]
        command = next(line for line in fallback.splitlines() if "$model = docker run" in line)
        self.assertIn('"${PWD}/ops/openfga:/work"', command)
        self.assertIn("openfga/cli@sha256:31a565d14d1c5ca8170d3a2225eb66517d9f490539967bcb3421613c5e2538a4", command)
        self.assertLess(fallback.index("if ($LASTEXITCODE -ne 0)"), fallback.index("Set-Content"))

    def test_nominal_python_sources_have_no_historical_inputs(self):
        root = SOURCE.resolve().parents[2]
        # Lead ruling: test_*.py carry assertions about paths, not nominal imports.
        files = sorted((root / "ops/deploy").glob("*.py")) + sorted(SOURCE.parent.glob("*.py"))
        construction = ast.parse('self.root / ("cluster-anchor-" + d["task_id"]) / "kubeconfig.yaml"', mode="eval").body
        construction_dump = ast.dump(construction, include_attributes=False)
        for path in files:
            if path.name.startswith("test_"):
                continue
            tree = ast.parse(path.read_text(encoding="utf-8"))
            allowed = set()
            if path.name == "local_full_target.py":
                for node in ast.walk(tree):
                    if ast.dump(node, include_attributes=False) == construction_dump:
                        allowed.update(id(child) for child in ast.walk(node) if isinstance(child, ast.Constant)
                                       and child.value == "cluster-anchor-")
                self.assertEqual(len(allowed), 1)
            for node in ast.walk(tree):
                if isinstance(node, ast.Constant) and isinstance(node.value, str):
                    with self.subTest(path=path.name, line=node.lineno):
                        for forbidden in ("tests/", ".tmp", ".cursor", "pool-", "cluster-anchor-"):
                            if forbidden == "cluster-anchor-" and id(node) in allowed:
                                continue
                            self.assertNotIn(forbidden, node.value)

    def test_e2e_prepare_blocked_cleanup_is_separate_noop_never_deletes_unknown_resources(self):
        module = self.load()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "inputs.json"
            path.write_text('{"version":1,"state":{}}')
            with patch.object(module.runtime, "kubectl") as kubectl:
                for action in ("e2e", "prepare-probes"):
                    args = argparse.Namespace(action=action, inputs=path, executed=0)
                    with self.assertRaises(module.ValidationBlocked):
                        module.explicit_probe_action(args, {})
                args = argparse.Namespace(action="cleanup-probes", inputs=None, executed=0)
                module.explicit_probe_action(args, {})
                self.assertEqual(args.executed, 1)
                args.inputs = path
                path.write_text('{"version":1,"created_resources":[{"name":"foreign","uid":"unknown"}]}')
                with self.assertRaises(module.ValidationBlocked):
                    module.explicit_probe_action(args, {})
                kubectl.assert_not_called()

    def test_blocked_main_nonzero_counts_and_does_not_log_secret(self):
        module = self.load()
        output = io.StringIO()
        with patch("sys.argv", [str(SOURCE), "smoke", "--environment", "environment.json"]), patch.object(
                module, "bind_environment", side_effect=ValueError("SECRET-CONTENT")), contextlib.redirect_stdout(output):
            self.assertEqual(module.main(), 2)
        report = json.loads(output.getvalue())
        self.assertEqual(report["status"], "BLOCKED")
        self.assertEqual((report["expected"], report["executed"], report["failed"], report["blocked"]), (1, 0, 0, 1))
        self.assertNotIn("SECRET-CONTENT", output.getvalue())

    def test_checker_preserves_eleven_paths_and_adds_moved_validation_paths(self):
        root = SOURCE.resolve().parents[2]
        source = (root / "ops/scripts/validate-deploy-static.ps1").read_text()
        baseline = ["workers/apparatus-operator/Dockerfile.controller", "workers/ext-authz/Dockerfile",
                    "runtime/monolith/Dockerfile", "ops/scripts/generate-platform-mesh-certs.sh",
                    "ops/deploy/kind/cluster.yaml", "ops/deploy/kind/cluster-local-full.yaml",
                    "ops/deploy/apps/overlays/kind/kustomization.yaml",
                    "ops/deploy/apps/overlays/kind-mesh/kustomization.yaml",
                    "ops/deploy/apps/overlays/kind-demo-monolith/kustomization.yaml",
                    "crates/apparatus-contracts/Cargo.toml", "crates/apparatus-reference-kv/Dockerfile.http"]
        self.assertEqual(len(baseline), 11)
        for path in baseline + ["ops/tests/manifests/invoke-probe-job.yaml", "ops/tests/it-native.ps1",
                                "ops/tests/test_local_full_target.py", "ops/tests/legacy/prove-gold-path.ps1"]:
            self.assertIn("'" + path + "'", source)
            self.assertTrue((root / path).is_file())
        self.assertIn("(Join-Path $root 'ops/tests')", source)

    def test_it_native_has_exact_suite_mapping_and_no_runtime_or_workspace(self):
        source = SOURCE.with_name("it-native.ps1").read_text()
        for suite in ("iam", "hive", "manifesto", "telegraph", "lazaret"):
            self.assertIn(f"{suite} = '{suite}-service'", source)
        self.assertIn("foreach ($selected in $Suite)", source)
        self.assertIn("& cargo test -p $packages[$selected] --tests", source)
        self.assertIn("$LASTEXITCODE -ne 0", source)
        for forbidden in ("--workspace", "docker run", "kind create", "compose up", "seed"):
            self.assertNotIn(forbidden, source)
        self.assertIn("E:\\cargo-target\\AIForAll", source)

    def test_closed_offline_render_inventory_is_exactly_ten(self):
        root = SOURCE.resolve().parents[2]
        source = (root / "ops/tests/infra-static.sh").read_text()
        blocks = source.split("<<'PY'\n")[1:]
        inventory = ast.parse(blocks[-1].split("\nPY", 1)[0])
        expected = next(ast.literal_eval(node.value) for node in inventory.body if isinstance(node, ast.Assign)
                        and any(isinstance(t, ast.Name) and t.id == "expected" for t in node.targets))
        actual = {p.parent.relative_to(root).as_posix() for p in (root / "ops/deploy").rglob("kustomization.yaml")}
        self.assertEqual(len(expected), 10)
        self.assertIn("ops/deploy/apps/overlays/local-full/oauth-proof", expected)
        self.assertIn("ops/deploy/p4/knative", expected)
        self.assertEqual(expected, actual)
        self.assertIn('[ "$renders" -eq 10 ]', source)
        self.assertIn("len(actual) != len(expected)", source)

    def test_legacy_gold_root_and_private_test_key_helper_migration(self):
        root = SOURCE.resolve().parents[2]
        gold = (root / "ops/tests/legacy/prove-gold-path.ps1").read_text(encoding="utf-8-sig")
        self.assertIn("(Join-Path $PSScriptRoot '../../deploy/common.ps1')", gold)
        self.assertIn("Get-AIForAllRoot $PSScriptRoot", gold)
        self.assertIn("non nominal", gold)
        helper_path = "services/IAMRusty/infra/tests/fixtures/generate-probe-keys.sh"
        helper = (root / helper_path).read_text()
        self.assertIn("umask 077", helper)
        self.assertIn("mktemp -d", helper)
        self.assertIn('mkdir -m 700 -- "$output"', helper)
        self.assertIn("IAMRUSTY_TEST_RSA8192_PEM_B64", helper)
        self.assertNotIn(">>", helper)
        self.assertNotIn('"$root/.env"', helper)
        probe = (root / "services/IAMRusty/infra/src/signing/probe.rs").read_text(encoding="utf-8")
        self.assertIn(helper_path, probe)
        self.assertNotIn("ops/scripts/generate-test-keys.sh", probe)

    def test_modern_modes_refuse_ignored_legacy_options_before_lease(self):
        module = self.load()
        for action, option in [("smoke", ["--service", "ignored"]), ("route-matrix", ["--port", "10000"]),
                               ("smoke", ["--suite", "apparatus"]), ("smoke", ["--path", "/iam/api/me"])]:
            with self.subTest(action=action, option=option):
                output = io.StringIO()
                with patch("sys.argv", [str(SOURCE), action, "--environment", "environment.json", *option]), patch.object(
                        module, "read_environment", return_value=self.environment(SOURCE.parent.resolve())), patch.object(
                        module.runtime, "lease") as lease, contextlib.redirect_stdout(output):
                    self.assertEqual(module.main(), 2)
                    lease.assert_not_called()
                self.assertEqual(json.loads(output.getvalue())["status"], "BLOCKED")


if __name__ == "__main__":
    unittest.main()
