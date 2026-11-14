"""Offline transport/query evidence tests; no MCP server or runtime is started."""

import argparse
import asyncio
from contextlib import asynccontextmanager, contextmanager, redirect_stderr, redirect_stdout
import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import AsyncMock, Mock, patch


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "mcp-code-evidence.py"
sys.path.insert(0, str(SCRIPT.parent))
SPEC = importlib.util.spec_from_file_location("mcp_code_evidence", SCRIPT)
collector = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(collector)


class Model:
    def __init__(self, data):
        self.data = data

    def model_dump(self, **kwargs):
        return self.data


def tool(name="workspace_stats", exact=None):
    fields = collector.SELECTORS.get(name, set()) | {"directory"}
    if name in collector.PAGED:
        fields |= {"limit", "offset", "summary"}
    return {
        "name": exact or "rust-code-mcp_1mcp_" + name,
        "inputSchema": {"properties": {key: {} for key in fields},
                        "required": list(collector.SELECTORS.get(name, set()) | {"directory"})},
        "annotations": {"readOnlyHint": True},
    }


def manifest(root, name="workspace_stats"):
    args = {"directory": str(root)}
    args.update({key: "example::Symbol" for key in collector.SELECTORS[name]})
    return {"workspace": str(root), "requests": [
        {"id": "first", "tool": name, "arguments": args},
    ]}


def result(structured=None, text=None, error=False):
    return Model({"isError": error, "structuredContent": structured,
                  "content": [] if text is None else [{"type": "text", "text": text}]})


def session(tools=None, results=None):
    return SimpleNamespace(
        initialize=AsyncMock(return_value=SimpleNamespace(
            serverInfo=Model({"name": "fixture-server", "version": "fixture"}))),
        list_tools=AsyncMock(return_value=SimpleNamespace(
            tools=[Model(item) for item in (tools or [tool()])], nextCursor=None)),
        call_tool=AsyncMock(side_effect=results or [result(text='{"nodes":{"workspace":1}}')]),
    )


class ValidationTests(unittest.TestCase):
    def test_loopback_urls(self):
        for url in ("http://127.0.0.1:3062/mcp?tags=rust-code-mcp",
                    "https://[::1]/mcp", "http://127.0.0.2/mcp"):
            with self.subTest(url=url):
                self.assertEqual(url, collector.validate_url(url))
        self.assertEqual("http://127.0.0.1/mcp",
                         collector.validate_url("http://localhost/mcp"))

    def test_external_and_credential_urls_rejected(self):
        for url in ("http://example.com/mcp", "http://192.168.1.1/mcp",
                    "http://127.0.0.1.evil/mcp", "file:///mcp",
                    "http://user:password@127.0.0.1/mcp",
                    "http://@127.0.0.1/mcp", "http://127.0.0.1/mcp#",
                    "http://[::1%25lo]/mcp", "http://127.0.0.1:bad",
                    "http://127.1/mcp", "http://127.0.0.1/\n", "http:///mcp"):
            with self.subTest(url=url), self.assertRaises(collector.EvidenceError):
                collector.validate_url(url)

    def test_defaults_and_root(self):
        data = collector.validate_manifest(manifest(collector.ROOT, "who_uses"))
        self.assertEqual(50, data["requests"][0]["arguments"]["limit"])
        self.assertEqual(0, data["requests"][0]["arguments"]["offset"])

    def test_root_mismatch(self):
        data = manifest(collector.ROOT)
        for location in ("workspace", "directory"):
            changed = json.loads(json.dumps(data))
            if location == "workspace":
                changed["workspace"] = str(collector.ROOT.parent)
            else:
                changed["requests"][0]["arguments"]["directory"] = str(collector.ROOT.parent)
            with self.subTest(location=location), self.assertRaises(collector.EvidenceError):
                collector.validate_manifest(changed)

    def test_unknown_tools_and_arbitrary_fields_rejected(self):
        for name in ("build_hypergraph", "activate_project", "clear_cache",
                     "search", "rust-code-mcp_1mcp_who_uses", "who_uses_extra"):
            data = manifest(collector.ROOT)
            data["requests"][0]["tool"] = name
            with self.subTest(name=name), self.assertRaises(collector.EvidenceError):
                collector.validate_manifest(data)
        data = manifest(collector.ROOT, "who_uses")
        data["requests"][0]["arguments"]["file_path"] = "../secret"
        with self.assertRaises(collector.EvidenceError):
            collector.validate_manifest(data)

    def test_selector_traversal_and_urls_rejected(self):
        for value in ("../secret", r"..\secret", "/absolute", "http://localhost/a",
                      "name\nother", "name\x00", ""):
            data = manifest(collector.ROOT, "who_uses")
            data["requests"][0]["arguments"]["target"] = value
            with self.subTest(value=value), self.assertRaises(collector.EvidenceError):
                collector.validate_manifest(data)

    def test_bounds_and_types(self):
        for key, value in (("limit", 0), ("limit", 201), ("limit", True),
                           ("offset", -1), ("offset", 100001),
                           ("offset", 0.5), ("summary", "true")):
            data = manifest(collector.ROOT, "who_uses")
            data["requests"][0]["arguments"][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(collector.EvidenceError):
                collector.validate_manifest(data)

    def test_request_count_ids_and_shapes(self):
        cases = []
        data = manifest(collector.ROOT)
        data["requests"] *= 33
        cases.append(data)
        data = manifest(collector.ROOT)
        data["requests"] *= 2
        cases.append(data)
        cases.extend([{}, [], {"workspace": str(collector.ROOT), "requests": []}])
        data = manifest(collector.ROOT)
        data["requests"][0]["arguments"] = []
        cases.append(data)
        for data in cases:
            with self.subTest(data=data), self.assertRaises(collector.EvidenceError):
                collector.validate_manifest(data)

    def test_manifest_strict_json_and_size(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "requests.json"
            for content in ('{"workspace":1,"workspace":2}', '{"value":NaN}', "{",
                            "\udcff"):
                path.write_bytes(content.encode("utf-8", errors="surrogateescape"))
                with self.subTest(content=repr(content)), self.assertRaises(collector.EvidenceError):
                    collector.read_manifest(path)
            path.write_bytes(b" " * (collector.MAX_INPUT_BYTES + 1))
            with self.assertRaises(collector.EvidenceError):
                collector.read_manifest(path)

    def test_all_curated_argument_contracts(self):
        for name in collector.SELECTORS:
            with self.subTest(name=name):
                validated = collector.validate_manifest(manifest(collector.ROOT, name))
                resolved = collector.resolve_requests(validated["requests"], [tool(name)])
                self.assertEqual("rust-code-mcp_1mcp_" + name, resolved[0]["exact_tool"])

    def test_namespace_ambiguity_and_schema_drift(self):
        requests = collector.validate_manifest(manifest(collector.ROOT))["requests"]
        for advertised in ([], [tool(), tool(exact="evil_1mcp_workspace_stats")],
                           [tool(exact="evil_1mcp_workspace_stats")],
                           [{"name": tool()["name"], "inputSchema": {}}]):
            with self.subTest(advertised=advertised), self.assertRaises(collector.EvidenceError):
                collector.resolve_requests(requests, advertised)

    def test_dependency_missing_clear_error(self):
        with patch.object(collector, "version", side_effect=collector.PackageNotFoundError):
            with self.assertRaisesRegex(collector.EvidenceError, "Missing dependency"):
                collector.load_sdk()

    def test_dependency_version_rejected(self):
        with patch.object(collector, "version", return_value="2.0.0"):
            with self.assertRaisesRegex(collector.EvidenceError, "mcp==1.21.1"):
                collector.load_sdk()


class AsyncTests(unittest.IsolatedAsyncioTestCase):
    async def test_catalog_pages(self):
        fake = session()
        fake.list_tools.side_effect = [
            SimpleNamespace(tools=[Model(tool())], nextCursor="page2"),
            SimpleNamespace(tools=[Model(tool("who_uses"))], nextCursor=None),
        ]
        found = await collector.catalog(fake)
        self.assertEqual(2, len(found))
        self.assertEqual([{"cursor": None}, {"cursor": "page2"}],
                         [call.kwargs for call in fake.list_tools.call_args_list])

    async def test_catalog_cursor_loop(self):
        fake = session()
        fake.list_tools.return_value.nextCursor = "repeated"
        with self.assertRaisesRegex(collector.EvidenceError, "cursor"):
            await collector.catalog(fake)
        self.assertEqual(2, fake.list_tools.await_count)
        fake.call_tool.assert_not_awaited()

    async def test_catalog_page_budget(self):
        fake = session()
        fake.list_tools.side_effect = [
            SimpleNamespace(tools=[], nextCursor=str(i)) for i in range(collector.MAX_PAGES)]
        with self.assertRaisesRegex(collector.EvidenceError, "page budget"):
            await collector.catalog(fake)

    async def test_catalog_tool_budget(self):
        fake = session()
        with patch.object(collector, "MAX_TOOLS", 0):
            with self.assertRaisesRegex(collector.EvidenceError, "tool budget"):
                await collector.catalog(fake)

    async def test_text_only_stats_raw_and_unknown_coverage(self):
        fake = session()
        data = collector.validate_manifest(manifest(collector.ROOT))
        requests = collector.resolve_requests(data["requests"], [tool()])
        evidence = {"server": {"name": "fixture"}, "workspace": data["workspace"], "requests": []}
        await collector.collect(fake, requests, evidence)
        record = evidence["requests"][0]
        self.assertEqual("received", record["status"])
        self.assertIsNone(record["result"]["structuredContent"])
        self.assertEqual('{"nodes":{"workspace":1}}', record["result"]["content"][0]["text"])
        self.assertEqual("unknown", record["pagination"]["status"])
        self.assertIn("unknown", record["coverage"])

    async def test_tool_errors_stop_sequence(self):
        for response in (result(error=True),
                         result(structured={"error": "graph unavailable"}),
                         result(text='{"status":"failed"}'),
                         result(text='{"success":false}'),
                         result(text="Error: permission denied")):
            fake = session(results=[response])
            data = collector.validate_manifest(manifest(collector.ROOT))
            requests = collector.resolve_requests(data["requests"], [tool()]) * 2
            evidence = {"server": {}, "workspace": data["workspace"], "requests": []}
            with self.subTest(response=response.data), self.assertRaises(collector.EvidenceError):
                await collector.collect(fake, requests, evidence)
            self.assertEqual(1, fake.call_tool.await_count)
            self.assertEqual("tool_error", evidence["requests"][0]["status"])
            self.assertEqual(response.data, evidence["requests"][0]["result"])
            self.assertEqual("not_executed", evidence["requests"][1]["status"])

    async def test_errors_in_both_envelopes_checked(self):
        raw = result(structured={"nodes": {}}, text='{"error":"denied"}').data
        self.assertTrue(collector.result_failed(raw))
        self.assertFalse(collector.result_failed(
            result(text='{"source":"contains the word error", "items":[]}').data))

    async def test_transport_failure_and_timeout(self):
        for exc, expected in ((OSError("private failure"), "transport_error"),
                              (TimeoutError(), "timeout")):
            fake = session()
            fake.call_tool.side_effect = exc
            data = collector.validate_manifest(manifest(collector.ROOT))
            evidence = {"server": {}, "workspace": data["workspace"], "requests": []}
            with self.subTest(expected=expected), self.assertRaises(type(exc)):
                await collector.collect(fake, collector.resolve_requests(
                    data["requests"], [tool()]), evidence)
            self.assertEqual(expected, evidence["requests"][0]["status"])
            fake.call_tool.assert_awaited_once()

    async def test_oversize_result_fails(self):
        fake = session()
        evidence = {"server": {}, "workspace": str(collector.ROOT), "requests": []}
        requests = collector.resolve_requests(
            collector.validate_manifest(manifest(collector.ROOT))["requests"], [tool()])
        with patch.object(collector, "MAX_RESULT_BYTES", 1):
            with self.assertRaisesRegex(collector.EvidenceError, "8 MiB"):
                await collector.collect(fake, requests, evidence)

    async def test_preflight_all_before_first_tool(self):
        fake = session()
        data = collector.validate_manifest(manifest(collector.ROOT))
        data["requests"].append({"id": "missing", "tool": "who_uses",
                                 "arguments": {"directory": str(collector.ROOT),
                                               "target": "a::b", "limit": 50, "offset": 0}})

        @asynccontextmanager
        async def fake_connect(*args):
            yield fake

        with patch.object(collector, "connect", fake_connect):
            with self.assertRaisesRegex(collector.EvidenceError, "Unknown"):
                await collector.run(SimpleNamespace(command="collect", timeout=1,
                                                     url=collector.DEFAULT_URL), data, {})
        fake.call_tool.assert_not_awaited()

    async def test_whole_operation_deadline(self):
        fake = session()

        async def blocked():
            await asyncio.Future()

        fake.initialize.side_effect = blocked

        @asynccontextmanager
        async def fake_connect(*args):
            yield fake

        with patch.object(collector, "connect", fake_connect):
            with self.assertRaises(TimeoutError):
                await collector.run(SimpleNamespace(command="catalog", timeout=0.01,
                                                     url=collector.DEFAULT_URL), None, {})
        fake.call_tool.assert_not_awaited()

    async def test_cancelled_tool_retains_timeout_and_unexecuted_records(self):
        fake = session()

        async def blocked(*args, **kwargs):
            await asyncio.Future()

        fake.call_tool.side_effect = blocked
        data = collector.validate_manifest(manifest(collector.ROOT))
        requests = collector.resolve_requests(data["requests"], [tool()]) * 2
        evidence = {"server": {}, "workspace": data["workspace"], "requests": []}
        with self.assertRaises(TimeoutError):
            async with asyncio.timeout(0.01):
                await collector.collect(fake, requests, evidence)
        self.assertEqual(["timeout", "not_executed"],
                         [record["status"] for record in evidence["requests"]])

    async def test_sdk_transport_factory_no_proxy_no_redirect(self):
        captured = {}

        @asynccontextmanager
        async def stream(url, **kwargs):
            captured.update(kwargs)
            kwargs["httpx_client_factory"](headers={"x": "y"}, timeout=4, auth=None)
            yield (None, None, None)

        class Session:
            def __init__(self, *args):
                pass

            async def __aenter__(self):
                return self

            async def __aexit__(self, *args):
                pass

        httpx = SimpleNamespace(AsyncClient=Mock())
        with patch.object(collector, "load_sdk", return_value=(Session, stream, httpx)):
            async with collector.connect(collector.DEFAULT_URL, 2):
                pass
        kwargs = httpx.AsyncClient.call_args.kwargs
        self.assertIs(kwargs["trust_env"], False)
        self.assertIs(kwargs["follow_redirects"], False)
        self.assertIs(captured["terminate_on_close"], True)
        guard = kwargs["event_hooks"]["request"][0]
        with self.assertRaises(collector.EvidenceError):
            await guard(SimpleNamespace(url="http://external.example/mcp"))


class OutputTests(unittest.TestCase):
    def test_atomic_replace_failure_preserves_output_and_cleans_temp(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "accepted.json"
            path.write_text("old", encoding="utf-8")
            with patch.object(collector.os, "replace", side_effect=OSError("failure")):
                with self.assertRaises(OSError):
                    collector.atomic_json(path, {"new": True})
            self.assertEqual("old", path.read_text(encoding="utf-8"))
            self.assertEqual([path], list(Path(directory).iterdir()))

    def test_atomic_serialization_failure_preserves_output(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "accepted.json"
            path.write_text("old", encoding="utf-8")
            with self.assertRaises(ValueError):
                collector.atomic_json(path, {"number": float("nan")})
            self.assertEqual("old", path.read_text(encoding="utf-8"))
            collector.atomic_json(path, {"success": True})
            self.assertEqual({"success": True}, json.loads(path.read_text(encoding="utf-8")))

    def test_paths_protected(self):
        path = (collector.ROOT / ".tmp" / "input.json").resolve()
        with self.assertRaises(collector.EvidenceError):
            collector.output_path(str(path), {path})
        with self.assertRaises(collector.EvidenceError):
            collector.output_path(str(SCRIPT), set())

    def test_cli_transport_failure_report_preserves_accepted(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            output, report = base / "accepted.json", base / "failure.json"
            output.write_text("previous accepted", encoding="utf-8")

            async def fail(args, manifest, evidence):
                evidence["requests"].append({"id": "first", "status": "received"})
                raise OSError("secret server diagnostic")

            stderr = io.StringIO()
            with patch.object(collector, "run", fail), redirect_stderr(stderr):
                code = collector.main(["catalog", "--output", str(output),
                                       "--report", str(report)])
            self.assertEqual(1, code)
            self.assertEqual("previous accepted", output.read_text(encoding="utf-8"))
            failed = json.loads(report.read_text(encoding="utf-8"))
            self.assertEqual("failed", failed["status"])
            self.assertEqual(1, len(failed["requests"]))
            self.assertNotIn("secret", stderr.getvalue())

    def test_cli_output_cannot_overwrite_input_or_report(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "manifest.json"
            original = json.dumps(manifest(collector.ROOT))
            path.write_text(original, encoding="utf-8")
            for argv in (["collect", "--requests", str(path), "--output", str(path)],
                         ["collect", "--requests", str(path), "--report", str(path)],
                         ["catalog", "--output", str(path), "--report", str(path)]):
                with patch.object(collector, "connect") as connect, redirect_stderr(io.StringIO()):
                    self.assertEqual(1, collector.main(argv))
                    connect.assert_not_called()
                self.assertEqual(original, path.read_text(encoding="utf-8"))

    def test_cli_invalid_inputs_before_connection(self):
        for argv in (["--url", "http://external.example/mcp", "catalog"],
                     ["--timeout", "nan", "catalog"], ["--timeout", "0", "catalog"]):
            with patch.object(collector, "connect") as connect, redirect_stderr(io.StringIO()):
                self.assertEqual(1, collector.main(argv))
                connect.assert_not_called()

    def test_cli_catalog_stdout_sdk_session_mock(self):
        fake = session()

        @asynccontextmanager
        async def fake_connect(*args):
            yield fake

        stdout = io.StringIO()
        with patch.object(collector, "connect", fake_connect), redirect_stdout(stdout):
            self.assertEqual(0, collector.main(["catalog"]))
        received = json.loads(stdout.getvalue())
        self.assertEqual("received", received["status"])
        self.assertEqual("complete", received["pagination"]["status"])
        self.assertEqual("fixture-server", received["server"]["name"])
        fake.call_tool.assert_not_awaited()

    def test_cli_collect_text_json_accepted_atomic(self):
        fake = session()

        @asynccontextmanager
        async def fake_connect(*args):
            yield fake

        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            source, output = base / "manifest.json", base / "accepted.json"
            source.write_text(json.dumps(manifest(collector.ROOT)), encoding="utf-8")
            original = source.read_bytes()
            with patch.object(collector, "connect", fake_connect):
                self.assertEqual(0, collector.main(["collect", "--requests", str(source),
                                                    "--output", str(output)]))
            data = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual("received", data["requests"][0]["status"])
            self.assertFalse(data["requests"][0]["result"]["isError"])
            self.assertEqual(original, source.read_bytes())
            self.assertEqual("rust-code-mcp_1mcp_workspace_stats",
                             fake.call_tool.call_args.args[0])
            self.assertEqual({"directory": str(collector.ROOT.resolve())},
                             fake.call_tool.call_args.kwargs["arguments"])

    def test_no_process_or_opencode_execution(self):
        import ast
        tree = ast.parse(SCRIPT.read_text(encoding="utf-8"))
        imports = {node.names[0].name for node in ast.walk(tree) if isinstance(node, ast.Import)}
        self.assertNotIn("subprocess", imports)
        self.assertNotIn("opencode", imports)
        for node in ast.walk(tree):
            if isinstance(node, ast.Call):
                self.assertNotIn(ast.unparse(node.func), {"os.system", "os.popen", "eval", "exec"})


class ProducerTests(unittest.TestCase):
    def test_review_foreign_item_and_selected_inner_cfg_cannot_export_exact_proof(self):
        cases = (
            ('fn f() {\n    unsafe extern "C" {\n        fn foreign(x: i32);\n    }\n}\n', [3, 8, 3, 25]),
            ('fn f() {\n    #![cfg(feature = "x")]\n    1;\n}\n', [1, 0, 1, 8]),
        )
        for text, location in cases:
            with self.subTest(location=location), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "file.rs").write_text(text, encoding="utf-8")
                sources = collector.capture_sources(root, ["file.rs"], 1)
                native = {"name_path": "f", "kind": "Function", "relative_path": "file.rs",
                          "body_location": {"start_line": 0, "end_line": len(text.splitlines()) - 1}}
                collector.atomic_json(root / "q1.json", result(text=json.dumps([native])).data)
                sources["file.rs"]["range_witnesses"] = collector.decode_serena_witnesses([native], "q1")
                symbol = {"id": "s1", "qualified_name": "serena::file.rs::f", "kind": "Function",
                          "path": "file.rs", "range": collector.native_function_extent(native, text),
                          "source_sha256": sources["file.rs"]["sha256"]}
                inventory = {"schema_version": 1, "clippy": [{"id": "C1", "primary_spans": [
                    {"file": "file.rs", "range": location}]}], "sonar": []}
                mappings = collector.map_diagnostics(inventory, root, sources, [symbol],
                                                     {"s1": ["q1"]}, {"file.rs"})
                queries = [{"id": "q1", "tool": "serena_1mcp_find_symbol", "status": "ok",
                            "artifact": "q1.json", "artifact_sha256": collector.sha256(root / "q1.json")}]
                collector.localization_proofs(mappings, queries, root, root, sources, {}, inventory)
                self.assertNotEqual("exact", mappings[0]["status"])
                self.assertEqual([], list(root.glob("localization-*.json")))

    def test_c102_mixed_source_produces_range_local_proof_from_hashed_query(self):
        source_path = collector.ROOT / "services/Manifesto/tests/common/migration_roundtrip.rs"
        original = source_path.read_bytes()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / ".git").mkdir()
            (root / ".git" / "HEAD").write_text("a" * 40 + "\n")
            source = root / "migration.rs"
            source.write_bytes(original)
            inventory, plan, binding, proof = [root / name for name in
                                               ("inventory.json", "seeds.json", "binding.json", "binding-proof.json")]
            inventory.write_text(json.dumps({"schema_version": 1, "clippy": [
                {"id": "C000102", "primary_spans": [{"file": "migration.rs", "range": [5, 0, 5, 95]}]}],
                "sonar": [{"id": "S000316", "file": "migration.rs", "range": [5, 0, 5, 95]}]}))
            plan.write_text(json.dumps({"schema_version": 1, "source_files": ["migration.rs"],
                                       "symbols": [{"path": "migration.rs",
                                                    "name_path": "assert_initial_schema_roundtrip"}]}))
            proof.write_text('{"fixture":"independent synthetic binding, not a real review"}')
            binding.write_text(json.dumps({
                "inventory_sha256": collector.sha256(inventory), "head": "a" * 40,
                "method": "independent_equivalence",
                "files": [{"path": "migration.rs", "sha256": collector.sha256(source)}],
                "evidence_refs": [{"path": "binding-proof.json", "sha256": collector.sha256(proof),
                                   "locator": "/"}]}))
            tools = [{"name": "serena_1mcp_" + name, "inputSchema": {
                "properties": {key: {} for key in properties}, "required": []}}
                for name, properties in (
                    ("initial_instructions", []), ("get_current_config", []),
                    ("get_symbols_overview", ["relative_path", "depth"]),
                    ("find_symbol", ["name_path_pattern", "relative_path", "include_body", "depth", "max_matches"]))]
            hir = session()
            serena = session(tools=tools, results=[
                result(text="The project with name 'AIForAll' at " + str(root) + " is activated."),
                result(text="Active project: AIForAll\nLanguage server status: ready"),
                result(text='{"Function":["assert_initial_schema_roundtrip"]}'),
                result(text=json.dumps([{"name_path": "assert_initial_schema_roundtrip",
                    "kind": "Function", "relative_path": "migration.rs",
                    "body_location": {"start_line": 4, "end_line": 60}}])),
            ])

            @asynccontextmanager
            async def fake_connect(url, timeout):
                yield serena if "serena" in url else hir

            output, snapshot = root / "evidence.json", root / "snapshot.json"
            with patch.object(collector, "ROOT", root), patch.object(
                    collector, "CANONICAL_WORKSPACE", root), patch.object(collector, "connect", fake_connect):
                self.assertEqual(0, collector.main(["produce", "--workspace", str(root),
                    "--inventory", str(inventory), "--plan", str(plan), "--binding", str(binding),
                    "--output", str(output), "--snapshot", str(snapshot),
                    "--artifacts", str(root / "artifacts"), "--serena-url",
                    "http://127.0.0.1:3062/mcp?tags=serena"]))
            evidence = json.loads(output.read_text())
            self.assertEqual(["exact", "unknown"], [m["status"] for m in evidence["mappings"]])
            refs = evidence["mappings"][0]["evidence_refs"]
            ref = next(ref for ref in refs if isinstance(ref, dict) and ref["locator"] == "/")
            local = json.loads((root / ref["path"]).read_text())
            self.assertEqual("source_crosschecked_function_range_v1", local["local_discovery"]["method"])
            self.assertTrue(all(local["local_discovery"]["checked_predicates"].values()))
            self.assertTrue(all(q["coverage"]["freshness"] == "unknown" for q in evidence["queries"]))
            self.assertEqual([], evidence["relations"])
            self.assertEqual(original, source.read_bytes())
            symbol_query = next(q for q in evidence["queries"] if q["tool"] == "serena_1mcp_find_symbol")
            (root / symbol_query["artifact"]).write_text("{}")
            with self.assertRaisesRegex(collector.EvidenceError, "witness hash changed"):
                collector.localization_proofs(
                    evidence["mappings"], evidence["queries"], root, root / "artifacts",
                    collector.capture_sources(root, ["migration.rs"], 1), {}, json.loads(inventory.read_text()))

    def test_serena_decoder_retains_only_advertised_children(self):
        child = {"name_path": "impl A/f", "kind": "Method", "relative_path": "file.rs",
                 "body_location": {"start_line": 1, "end_line": 3}}
        parent = {"name_path": "impl A", "kind": "Object", "relative_path": "file.rs",
                  "body_location": {"start_line": 0, "end_line": 4}, "children": [child]}
        witnesses = collector.decode_serena_witnesses([parent], "q1")
        self.assertEqual(2, len(witnesses))
        self.assertEqual("impl A", witnesses[1]["parent_name_path"])
        self.assertEqual("q1", witnesses[1]["query_id"])
        self.assertEqual(child["body_location"], witnesses[1]["body_location"])
        del parent["children"]
        self.assertEqual(1, len(collector.decode_serena_witnesses([parent], "q1")))
        parent["children"] = {"not": "a returned child list"}
        with self.assertRaises(collector.EvidenceError):
            collector.decode_serena_witnesses([parent], "q1")

    def test_output_descendants_of_artifact_directory_rejected_before_transport(self):
        for destination, aliased in ((d, a) for d in ("output", "snapshot") for a in (False, True)):
            with self.subTest(destination=destination, resolved_alias=aliased), tempfile.TemporaryDirectory() as directory:
                root = Path(directory).resolve()
                (root / ".git").mkdir()
                (root / ".git" / "HEAD").write_text("a" * 40 + "\n")
                inventory, plan = root / "inventory.json", root / "seeds.json"
                inventory.write_text('{"schema_version":1,"clippy":[],"sonar":[]}')
                plan.write_text('{"schema_version":1,"source_files":[]}')
                artifacts = root / "artifacts"
                output = artifacts / "hir-catalog.json" if destination == "output" else root / "evidence.json"
                snapshot = artifacts / "q000001.json" if destination == "snapshot" else root / "snapshot.json"
                if aliased:
                    if destination == "output":
                        output = output.parent / "unused" / ".." / output.name
                    else:
                        snapshot = snapshot.parent / "unused" / ".." / snapshot.name
                with patch.object(collector, "ROOT", root), patch.object(
                        collector, "CANONICAL_WORKSPACE", root), patch.object(
                        collector, "connect", side_effect=collector.EvidenceError("reached transport")) as connect:
                    with redirect_stderr(io.StringIO()):
                        self.assertEqual(1, collector.main(["produce", "--workspace", str(root),
                            "--inventory", str(inventory), "--plan", str(plan),
                            "--snapshot", str(snapshot), "--output", str(output),
                            "--artifacts", str(artifacts)]))
                    connect.assert_not_called()
                self.assertFalse(artifacts.exists())

    def test_discovered_source_collision_preserves_both_accepted_outputs(self):
        for destination in ("output", "snapshot"):
            with self.subTest(destination=destination), tempfile.TemporaryDirectory() as directory:
                root = Path(directory).resolve()
                (root / ".git").mkdir()
                (root / ".git" / "HEAD").write_text("a" * 40 + "\n")
                inventory, plan = root / "inventory.json", root / "seeds.json"
                inventory.write_text('{"schema_version":1,"clippy":[],"sonar":[]}')
                plan.write_text('{"schema_version":1,"source_files":[],"targets":["crate::target"]}')
                consumer = root / "consumer.json"
                consumer.write_text("target", encoding="utf-8")
                output = consumer if destination == "output" else root / "accepted-evidence.json"
                snapshot = consumer if destination == "snapshot" else root / "accepted-snapshot.json"
                if output != consumer:
                    output.write_text("previous accepted evidence")
                if snapshot != consumer:
                    snapshot.write_text("previous accepted snapshot")
                before_output, before_snapshot = output.read_bytes(), snapshot.read_bytes()
                fake = session(tools=[tool("who_uses")], results=[result(text=json.dumps({
                    "target": "crate::target", "offset": 0, "limit": 50, "summary": False,
                    "total_match_count": 1, "returned_match_count": 1, "usages": [{
                        "file": "consumer.json", "start": 0, "end": 6, "category": "Other",
                        "consumer_module": "crate", "consumer_function": "crate::f"}]}))])

                @asynccontextmanager
                async def fake_connect(*args):
                    yield fake

                with patch.object(collector, "ROOT", root), patch.object(
                        collector, "CANONICAL_WORKSPACE", root), patch.object(collector, "connect", fake_connect):
                    with redirect_stderr(io.StringIO()):
                        self.assertEqual(1, collector.main(["produce", "--workspace", str(root),
                            "--inventory", str(inventory), "--plan", str(plan),
                            "--snapshot", str(snapshot), "--output", str(output),
                            "--artifacts", str(root / "artifacts")]))
                self.assertEqual(before_output, output.read_bytes())
                self.assertEqual(before_snapshot, snapshot.read_bytes())

    def test_producer_to_offline_planner_smoke(self):
        planner_path = Path(os.environ.get(
            "CLIPPY_PLANNER_SCRIPT", str(SCRIPT.with_name("clippy-constraint-plan.py"))))
        if not planner_path.is_file():
            self.skipTest("Set CLIPPY_PLANNER_SCRIPT to the read-only B script before integration")

        def load(name, path):
            spec = importlib.util.spec_from_file_location(name, path)
            module = importlib.util.module_from_spec(spec)
            sys.modules[name] = module
            spec.loader.exec_module(module)
            return module

        planner = load("evidence_smoke_planner", planner_path)
        overlap = load("evidence_smoke_overlap", SCRIPT.with_name("clippy-sonar-overlap.py"))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / ".git").mkdir()
            (root / ".git" / "HEAD").write_text("a" * 40 + "\n")
            source = root / "file.rs"
            source.write_text("/// Doc.\npub fn f() -> u32 {\n    1\n}\n", encoding="utf-8")
            for name in ("consumer0.rs", "consumer1.rs"):
                (root / name).write_text("pub fn use_f() {\n    f();\n}\n", encoding="utf-8")
            cargo, sonar = root / "cargo.jsonl", root / "sonar.json"
            cargo.write_text(json.dumps({
                "reason": "compiler-message", "message": {
                    "level": "warning", "code": {"code": "clippy::doc_markdown"},
                    "message": "doc example", "spans": [{
                        "is_primary": True, "file_name": "file.rs", "line_start": 1,
                        "column_start": 5, "line_end": 1, "column_end": 8}]},
                "package_id": "fixture", "target": {"kind": ["lib"]}}) + "\n"
                + '{"reason":"build-finished","success":true}\n', encoding="utf-8")
            sonar.write_text(json.dumps({
                "paging": {"pageIndex": 1, "pageSize": 100, "total": 1},
                "issues": [{"key": "fixture-issue", "rule": "external_clippy:doc_markdown",
                            "component": "fixture:file.rs", "project": "fixture",
                            "message": "doc example", "textRange": {
                                "startLine": 1, "startOffset": 4, "endLine": 1, "endOffset": 7}}]
            }), encoding="utf-8")
            inventory = root / "inventory.json"
            inventory.write_text(json.dumps(overlap.inventory([cargo], sonar)), encoding="utf-8")
            context = root / "sonar-context.json"
            context.write_text(json.dumps({
                "diagnostic_key": "sonar:S000001", "source_sha256": collector.sha256(source),
                "inventory_column_unit": "unicode_scalar"}), encoding="utf-8")
            proof = root / "proof.json"
            proof.write_text('{"fixture":"synthetic source equivalence, not a real approval"}')
            binding = root / "binding.json"
            binding.write_text(json.dumps({
                "inventory_sha256": collector.sha256(inventory), "head": "a" * 40,
                "method": "independent_equivalence",
                "files": [{"path": "file.rs", "sha256": collector.sha256(source)}],
                "evidence_refs": [{"path": "proof.json", "sha256": collector.sha256(proof),
                                   "locator": "fixture"}]}), encoding="utf-8")
            plan = root / "seeds.json"
            plan.write_text(json.dumps({"schema_version": 1,
                "source_files": ["file.rs", "consumer0.rs", "consumer1.rs"],
                "symbols": [{"path": "file.rs", "name_path": "f"}], "targets": ["fixture::f"]}))
            hir_results = []
            for offset in range(2):
                consumer = root / f"consumer{offset}.rs"
                start = consumer.read_bytes().index(b"f();")
                hir_results.append(result(text=json.dumps({
                    "target": "fixture::f", "offset": offset, "limit": 1, "summary": False,
                    "total_match_count": 2, "returned_match_count": 1, "usages": [{
                        "file": consumer.name, "start": start, "end": start + 1,
                        "category": "Other", "consumer_module": "fixture",
                        "consumer_function": f"fixture::consumer{offset}"}]})))
            hir = session(tools=[tool("who_uses")], results=hir_results)
            serena_tools = []
            for name, properties in (
                ("initial_instructions", []), ("get_current_config", []),
                ("get_symbols_overview", ["relative_path", "depth"]),
                ("find_symbol", ["name_path_pattern", "relative_path", "include_body", "depth", "max_matches"]),
            ):
                serena_tools.append({"name": "serena_1mcp_" + name, "inputSchema": {
                    "properties": {key: {} for key in properties}, "required": []}})
            serena = session(tools=serena_tools, results=[
                result(text="The project with name 'AIForAll' at " + str(root) + " is activated."),
                result(text="Active project: AIForAll\nLanguage server status: ready"),
                result(text='{"Function":["f"]}'),
                result(text=json.dumps([{"name_path": "f", "kind": "Function",
                    "relative_path": "file.rs", "body_location": {"start_line": 0, "end_line": 3}}])),
            ])

            @asynccontextmanager
            async def fake_connect(url, timeout):
                yield serena if "serena" in url else hir

            snapshot, evidence = root / "snapshot.json", root / "evidence.json"
            with patch.object(collector, "ROOT", root), patch.object(
                    collector, "CANONICAL_WORKSPACE", root), patch.object(collector, "connect", fake_connect):
                self.assertEqual(0, collector.main(["produce", "--workspace", str(root),
                    "--inventory", str(inventory), "--plan", str(plan), "--binding", str(binding),
                    "--context", str(context), "--snapshot", str(snapshot), "--output", str(evidence),
                    "--artifacts", str(root / "artifacts"), "--page-size", "1", "--serena-url",
                    "http://127.0.0.1:3062/mcp?tags=serena"]))
            emitted = json.loads(evidence.read_text())
            snap = json.loads(snapshot.read_text())
            self.assertEqual(["exact", "exact"], [m["status"] for m in emitted["mappings"]])
            for mapping in emitted["mappings"]:
                proofs = [ref for ref in mapping["evidence_refs"]
                          if isinstance(ref, dict) and ref["locator"] == "/"]
                self.assertEqual(1, len(proofs))
                proof_data = json.loads((root / proofs[0]["path"]).read_text())
                self.assertEqual("clippy-localization-proof", proof_data["kind"])
                self.assertEqual(mapping["diagnostic_key"], proof_data["diagnostic_key"])
                self.assertEqual([mapping["symbols"][0]["id"]],
                                 proof_data["local_discovery"]["candidate_symbol_ids"])
                self.assertTrue(all(proof_data["local_discovery"]["checked_predicates"].values()))
                self.assertTrue(all(q in mapping["evidence_refs"]
                                    for q in proof_data["local_discovery"]["query_ids"]))
                self.assertIn(proofs[0]["path"], [item["path"] for item in snap["inputs"]
                                                if item["role"] == "localization_proof"])
            self.assertEqual(2, emitted["queries"][0]["pagination"]["pages"])
            self.assertEqual(2, emitted["queries"][0]["pagination"]["returned"])
            for path in [q["artifact"] for q in emitted["queries"]] + [i["path"] for i in snap["inputs"]]:
                self.assertFalse(Path(path).is_absolute())
                self.assertTrue((root / path).is_file())
            report = root / "plan.json"
            self.assertEqual(0, planner.main(["--inventory", str(inventory), "--snapshot", str(snapshot),
                                             "--evidence", str(evidence), "--output", str(report)]))
            planned = json.loads(report.read_text())
            self.assertEqual(2, len(planned["diagnostics"]))
            self.assertEqual(0, planned["confirmed_multi_file_components"])
            self.assertEqual("unproven", planned["global_independence"])
            # Physical localization is distinct from unknown reference closure.
            self.assertEqual({"exact": 2}, planned["mapping_status_counts"])

    def test_verified_serena_and_hir_envelopes(self):
        data = [{"name_path": "f", "body_location": {"start_line": 0, "end_line": 2}}]
        raw = result(structured={"result": json.dumps(data)}, text=json.dumps(data)).data
        self.assertEqual(data, collector.decode_wire(raw, True))
        self.assertEqual({"usages": []}, collector.decode_wire(result(text='{"usages":[]}').data))
        raw["content"][0]["text"] = "[]"
        with self.assertRaisesRegex(collector.EvidenceError, "conflicting"):
            collector.decode_wire(raw, True)
        for raw in (result(error=True).data, result(text="not JSON").data,
                    result(structured={"error": "denied"}).data):
            with self.assertRaises(collector.EvidenceError):
                collector.decode_wire(raw)

    def test_root_confined_repeated_separators_and_aliases(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / "src").mkdir()
            (root / "src" / "file.rs").write_text("fn f() {}\n", encoding="utf-8")
            self.assertEqual("src/file.rs", collector.source_path(root, r"src\\file.rs")[0])
            for path in ("../file.rs", "src/./file.rs", "//device/file.rs",
                         "src/file.rs.", "SRC/file.rs", "src/missing.rs"):
                with self.subTest(path=path), self.assertRaises(collector.EvidenceError):
                    collector.source_path(root, path)

    def test_source_hash_mismatch_and_independent_binding(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "file.rs"
            path.write_text("fn f() {}\n", encoding="utf-8")
            sources = collector.capture_sources(root, ["file.rs"], 1)
            self.assertEqual(set(), collector.bound_sources(None, "inv", "head", sources))
            proof = root / "proof.json"
            proof.write_text('{"proof":"synthetic independent context"}', encoding="utf-8")
            binding = {"inventory_sha256": "inv", "head": "head",
                       "method": "independent_equivalence", "evidence_refs": [
                           {"path": str(proof), "sha256": collector.sha256(proof), "locator": "proof"}],
                       "files": [{"path": "file.rs", "sha256": sources["file.rs"]["sha256"]}]}
            self.assertEqual({"file.rs"}, collector.bound_sources(binding, "inv", "head", sources))
            with self.assertRaises(collector.EvidenceError):
                collector.bound_sources(binding, "different", "head", sources)
            path.write_text("fn changed() {}\n", encoding="utf-8")
            with self.assertRaisesRegex(collector.EvidenceError, "changed"):
                collector.verify_sources(sources)

    def test_unicode_columns_and_character_boundaries(self):
        text = "aé😀z\r\nnext\n"
        for unit, col in (("unicode_scalar", 3), ("utf8_byte", 7), ("utf16", 4)):
            self.assertEqual(3, collector.offset_in_source(text, 1, col, unit))
        self.assertEqual(7, collector.offset_in_source(text, 2, 1, "unicode_scalar"))
        for unit, col in (("utf8_byte", 2), ("utf16", 3), ("unicode_scalar", 8)):
            with self.subTest(unit=unit), self.assertRaises(collector.EvidenceError):
                collector.offset_in_source(text, 1, col, unit)
        self.assertEqual(3, collector.offset_in_source("a\u2028b\n", 1, 3, "unicode_scalar"))
        self.assertEqual(4, collector.offset_in_source("a\u2028b\nx", 2, 0, "unicode_scalar"))

    def test_inclusive_serena_endpoint_crosschecked_against_source(self):
        text = "/// Documentation.\npub fn f() -> u32 {\n    1\n}\n"
        native = {"name_path": "f", "kind": "Function",
                  "body_location": {"start_line": 0, "end_line": 3}}
        extent = collector.serena_extent(native, text)
        self.assertEqual((1, 5, 0), (extent["start_line"], extent["end_line"], extent["end_column"]))
        native["body_location"]["end_line"] = 2
        with self.assertRaises(collector.EvidenceError):
            collector.serena_extent(native, text)
        native["body_location"]["end_line"] = 3
        for source in (text.replace("    1", "    macro_call!()"),
                       text.replace("    1", "    fn nested() {}"),
                       text.replace("    1", "    let closure = || 1;"),
                       text.replace("    1", "    /* unsupported comment */ 1"),
                       text.replace("/// Documentation.", "#[external_macro]")):
            with self.assertRaises(collector.EvidenceError):
                collector.serena_extent(native, source)

    def test_localization_proof_rejects_forged_nested_completeness(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "file.rs").write_text("pub fn f() {\n    fn nested() {}\n}\n", encoding="utf-8")
            sources = collector.capture_sources(root, ["file.rs"], 1)
            mapping = {"diagnostic_key": "clippy:C1", "status": "exact", "evidence_refs": ["q1"], "reasons": [],
                       "symbols": [{"id": "s1", "path": "file.rs", "kind": "Function",
                           "qualified_name": "serena::file.rs::f",
                           "source_sha256": sources["file.rs"]["sha256"], "range": {
                               "start_line": 1, "start_column": 0, "end_line": 4,
                               "end_column": 0, "column_unit": "unicode_scalar"}}]}
            collector.localization_proofs([mapping], [{"id": "q1", "status": "ok",
                "tool": "serena_1mcp_find_symbol"}], root, root, sources, {})
            self.assertEqual("unknown", mapping["status"])
            self.assertEqual([], list(root.glob("localization-*.json")))

    def test_mapping_exact_stale_ambiguous_missing_and_sonar_unknown(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            text = "pub fn f() -> u32 {\n    1\n}\n"
            (root / "file.rs").write_text(text, encoding="utf-8")
            sources = collector.capture_sources(root, ["file.rs"], 1)
            symbol = {"id": "s1", "qualified_name": "serena::file.rs::f", "kind": "Function",
                      "path": "file.rs", "source_sha256": sources["file.rs"]["sha256"],
                      "range": collector.serena_extent({"name_path": "f", "kind": "Function",
                                "body_location": {"start_line": 0, "end_line": 2}}, text)}
            inv = {"schema_version": 1, "clippy": [{"id": "C1", "primary_spans": [
                {"file": "file.rs", "range": [2, 4, 2, 5]}]}],
                "sonar": [{"id": "S1", "file": "file.rs", "range": [2, 4, 2, 5]}]}
            args = (inv, root, sources, [symbol], {"s1": ["q1"]})
            mapped = collector.map_diagnostics(*args, {"file.rs"})
            self.assertEqual(["exact", "unknown"], [m["status"] for m in mapped])
            self.assertEqual("stale", collector.map_diagnostics(*args, set())[0]["status"])
            duplicate = dict(symbol, id="s2")
            mapped = collector.map_diagnostics(inv, root, sources, [symbol, duplicate],
                                               {"s1": ["q1"], "s2": ["q2"]}, {"file.rs"})
            self.assertEqual("ambiguous", mapped[0]["status"])
            inv["clippy"][0]["primary_spans"][0]["file"] = "./file.rs"
            self.assertEqual("ambiguous", collector.map_diagnostics(*args, {"file.rs"})[0]["status"])
            inv["clippy"][0]["primary_spans"][0]["file"] = "missing.rs"
            self.assertEqual("unknown", collector.map_diagnostics(*args, {"file.rs"})[0]["status"])

    def test_native_byte_mismatch_never_repaired(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "file.rs").write_text("x target mismatch", encoding="utf-8")
            sources = collector.capture_sources(root, ["file.rs"], 1)
            limitations = []
            row = {"file": "file.rs", "start": 2, "end": 8}
            self.assertTrue(collector.check_usage_sources(
                [row], "crate::target", root, sources, 1, limitations))
            row["start"], row["end"] = 4, 10
            self.assertFalse(collector.check_usage_sources(
                [row], "crate::target", root, sources, 1, limitations))
            self.assertIn("no offset repair", limitations[0])

    def test_unindexed_worktree_rejected_before_any_connection(self):
        with patch.object(collector, "connect") as connect, redirect_stderr(io.StringIO()):
            self.assertEqual(1, collector.main(["produce", "--inventory", "missing.json",
                "--plan", "missing.json", "--workspace", str(collector.ROOT),
                "--snapshot", "snapshot.json", "--output", "evidence.json",
                "--artifacts", "new-artifacts"]))
            connect.assert_not_called()

    def test_producer_contract_artifacts_with_mock_session(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / ".git").mkdir()
            (root / ".git" / "HEAD").write_text("a" * 40 + "\n")
            (root / "file.rs").write_text("target\n", encoding="utf-8")
            inventory = root / "inventory.json"
            inventory.write_text(json.dumps({"schema_version": 1, "clippy": [
                {"id": "C1", "primary_spans": [{"file": "file.rs", "range": [1, 0, 1, 6]}]}],
                "sonar": []}), encoding="utf-8")
            plan = root / "plan.json"
            plan.write_text(json.dumps({"schema_version": 1, "source_files": ["file.rs"],
                                       "targets": ["crate::target"]}), encoding="utf-8")
            fake = session(tools=[tool("who_uses")], results=[result(text=json.dumps({
                "target": "crate::target", "offset": 0, "limit": 50, "summary": False,
                "returned_match_count": 1, "total_match_count": 1,
                "usages": [{"file": "file.rs", "start": 0, "end": 6, "category": "Other",
                            "consumer_module": "crate", "consumer_function": "crate::f"}]}))])

            @asynccontextmanager
            async def fake_connect(*args):
                yield fake

            with patch.object(collector, "CANONICAL_WORKSPACE", root), patch.object(
                    collector, "ROOT", root), patch.object(collector, "connect", fake_connect):
                self.assertEqual(0, collector.main(["produce", "--workspace", str(root),
                    "--inventory", str(inventory), "--plan", str(plan),
                    "--snapshot", str(root / "snapshot.json"), "--output", str(root / "evidence.json"),
                    "--artifacts", str(root / "artifacts")]))
            evidence = json.loads((root / "evidence.json").read_text())
            snapshot = json.loads((root / "snapshot.json").read_text())
            self.assertEqual("clippy-code-evidence", evidence["kind"])
            self.assertEqual(collector.sha256(root / "snapshot.json"), evidence["snapshot_sha256"])
            self.assertEqual("clippy:C1", evidence["mappings"][0]["diagnostic_key"])
            self.assertEqual("complete", evidence["queries"][0]["pagination"]["status"])
            sequence = json.loads((root / evidence["queries"][0]["artifact"]).read_text())
            self.assertEqual(1, len(sequence["pages"]))
            raw_path = root / sequence["pages"][0]["artifact"]
            self.assertFalse(json.loads(raw_path.read_text())["isError"])
            self.assertEqual(collector.sha256(raw_path), sequence["pages"][0]["artifact_sha256"])
            self.assertEqual([], evidence["relations"])
            self.assertEqual("unknown", snapshot["source_binding"])
            self.assertEqual("file.rs", snapshot["files"][0]["path"])


class ObservationTests(unittest.TestCase):
    @contextmanager
    def origin_case(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / ".git").mkdir()
            (root / ".git" / "HEAD").write_text("a" * 40 + "\n")
            files = {
                collector.ORIGIN_MANIFEST: '[package]\nname = "apparatus-contracts"\n',
                collector.ORIGIN_MODULE: "pub mod runtime;\n",
                collector.ORIGIN_SOURCE: ("#![allow(missing_docs)]\n/// Validator.\n"
                    "pub fn validate_runtime_uid(value: &str) -> Result<(), RuntimeErrorCodeV1> {\n"
                    "    Ok(())\n}\n"),
                "services/sample/src/consumer.rs": "validate_runtime_uid(value);\nvalidate_runtime_uid(other);\n",
                "services/sample/src/stale.rs": "not_a_matching_reference\n",
            }
            for name, text in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text, encoding="utf-8")
            spec = importlib.util.spec_from_file_location("observation_overlap", SCRIPT.with_name("clippy-sonar-overlap.py"))
            overlap = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(overlap)
            cargo, sonar = root / "cargo.jsonl", root / "sonar.json"
            cargo.write_text(json.dumps({"reason": "compiler-message", "package_id": "fixture",
                "target": {"kind": ["lib"]}, "message": {"level": "warning",
                "code": {"code": "clippy::uninlined_format_args"}, "message": "fixture",
                "spans": [{"is_primary": True, "file_name": "services/sample/src/consumer.rs",
                           "line_start": 1, "column_start": 1, "line_end": 1, "column_end": 21}]}}) +
                '\n{"reason":"build-finished","success":true}\n')
            sonar.write_text('{"paging":{"pageIndex":1,"pageSize":100,"total":0},"issues":[]}')
            inventory, plan = root / "inventory.json", root / "plan.json"
            inventory.write_text(json.dumps(overlap.inventory([cargo], sonar)))
            plan.write_text(json.dumps({"schema_version": 1, "source_files": list(files),
                "symbols": [{"path": collector.ORIGIN_SOURCE, "name_path": collector.ORIGIN_NAME}],
                "targets": [collector.ORIGIN_TARGET]}))
            rows = []
            second = (root / "services/sample/src/consumer.rs").read_bytes().index(
                collector.ORIGIN_NAME.encode("ascii"), 1)
            for index, (file, start) in enumerate((
                (r"services\\sample\\src\\consumer.rs", 0),
                (r"services\\sample\\src\\consumer.rs", second),
                (r"services\\sample\\src\\stale.rs", 0),
            )):
                row = {"file": file, "start": start, "end": start + len(collector.ORIGIN_NAME),
                       "category": "Other", "consumer_module": "sample"}
                if index == 0:
                    row["consumer_function"] = "sample::consumer"
                rows.append(row)
            case = SimpleNamespace(root=root, inventory=inventory, plan=plan, rows=rows,
                output=root / "evidence.json", snapshot=root / "snapshot.json",
                observations=root / "observations.json", artifacts=root / "captures",
                drift=None, extra=[],
                signature={"target": collector.ORIGIN_TARGET, "signature": {
                    "is_async": False, "self_param": None, "params": [
                        {"name": "value", "ty": "&str", "by_ref": True, "mutability": False}],
                    "return_type": "Result<(), RuntimeErrorCodeV1>", "generics": []}},
                definition="Found 1 definition(s) for 'validate_runtime_uid':\n"
                    + "\\\\?\\" + str(root / collector.ORIGIN_SOURCE)
                    + ":3:8 (validate_runtime_uid, exact=true)",
                native={"name_path": collector.ORIGIN_NAME, "kind": "Function",
                    "relative_path": collector.ORIGIN_SOURCE,
                    "body_location": {"start_line": 1, "end_line": 4}})
            yield case

    def run_case(self, case):
        hir_tools = [tool("who_uses"), tool("function_signature"), {
            "name": "rust-code-mcp_1mcp_find_definition", "inputSchema": {
                "properties": {key: {} for key in ("directory", "symbol_name", "exact")},
                "required": ["directory", "symbol_name"]}}]
        hir = session(tools=hir_tools, results=[
            result(text=json.dumps({"target": collector.ORIGIN_TARGET, "offset": 0, "limit": 50,
                "summary": False, "total_match_count": len(case.rows),
                "returned_match_count": len(case.rows), "usages": case.rows})),
            result(text=json.dumps(case.signature)), result(text=case.definition)])
        serena_tools = [{"name": "serena_1mcp_" + name, "inputSchema": {
            "properties": {key: {} for key in properties}, "required": []}}
            for name, properties in (
                ("initial_instructions", []), ("get_current_config", []),
                ("get_symbols_overview", ["relative_path", "depth"]),
                ("find_symbol", ["name_path_pattern", "relative_path", "include_body", "depth", "max_matches"]))]
        serena = session(tools=serena_tools)
        responses = [
            result(text="The project with name 'AIForAll' at " + str(case.root) + " is activated."),
            result(text="Active project: AIForAll\nLanguage server status: ready"),
            result(text=json.dumps({"Function": [collector.ORIGIN_NAME]})),
            result(text=json.dumps([case.native]))]
        async def serena_call(*args, **kwargs):
            value = responses.pop(0)
            if not responses and case.drift:
                case.drift()
            return value
        serena.call_tool.side_effect = serena_call

        @asynccontextmanager
        async def fake_connect(url, timeout):
            yield serena if "serena" in url else hir

        connect = Mock(wraps=fake_connect)
        argv = ["produce", "--workspace", str(case.root), "--inventory", str(case.inventory),
            "--plan", str(case.plan), "--output", str(case.output), "--snapshot", str(case.snapshot),
            "--observations", str(case.observations), "--artifacts", str(case.artifacts),
            "--serena-url", "http://127.0.0.1:3062/mcp?tags=serena", "--allow-partial"] + case.extra
        stderr = io.StringIO()
        with patch.object(collector, "ROOT", case.root), patch.object(
                collector, "CANONICAL_WORKSPACE", case.root), patch.object(
                collector, "connect", connect), redirect_stderr(stderr):
            code = collector.main(argv)
        return code, stderr.getvalue(), connect

    def test_observation_export_hash_direction_positive_and_unresolved_rows(self):
        with self.origin_case() as case:
            code, error, _ = self.run_case(case)
            self.assertEqual(0, code, error)
            observed = json.loads(case.observations.read_text())
            snapshot = json.loads(case.snapshot.read_text())
            evidence = json.loads(case.output.read_text())
            self.assertEqual(["file_candidate", "file_candidate", "unresolved"],
                             [o["identity_status"] for o in observed["observations"]])
            self.assertEqual(["source_verified", "source_verified", "path_only"],
                             [o["endpoint_status"] for o in observed["observations"]])
            self.assertEqual(collector.sha256(case.snapshot), observed["snapshot_sha256"])
            self.assertEqual(collector.sha256(case.output), observed["evidence_sha256"])
            self.assertNotIn("observations.json", [i["path"] for i in snapshot["inputs"]])
            origin_ref = observed["observations"][0]["origin_binding_ref"]
            self.assertIn(origin_ref["path"], [i["path"] for i in snapshot["inputs"]])
            origin = json.loads((case.root / origin_ref["path"]).read_text())
            self.assertEqual("direct_crate_module_function_v1", origin["method"])
            self.assertEqual(collector.ORIGIN_TARGET, origin["target"])
            self.assertNotIn("snapshot_sha256", origin)
            self.assertTrue(all(o["destination"]["symbol_id"] is None and o["destination"]["range"] is None
                                for o in observed["observations"]))
            self.assertIn("No consumer_function", " ".join(observed["observations"][1]["reasons"]))
            self.assertIn("no offset repair", " ".join(observed["observations"][2]["reasons"]))
            self.assertEqual([], evidence["relations"])
            self.assertTrue(all(q["coverage"]["private"] == "unknown" for q in evidence["queries"]))
            self.assertFalse(any(m["symbols"] for m in evidence["mappings"]))  # origin has no diagnostic

    def test_partial_b_library_validates_emitted_origin_and_usage_observations(self):
        planner_path = SCRIPT.with_name("clippy-constraint-plan.py")
        if not planner_path.is_file():
            self.skipTest("Offline consumer not integrated")
        spec = importlib.util.spec_from_file_location("observation_partial_planner", planner_path)
        planner = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = planner
        spec.loader.exec_module(planner)
        with self.origin_case() as case:
            code, error, _ = self.run_case(case)
            self.assertEqual(0, code, error)
            seen = {}
            # B's current partial reader does not export these collections yet.
            # Observe only this library return to distinguish real validation
            # from silently queued invalid proofs; no graph/CLI claim is made.
            def trace(frame, event, arg):
                if frame.f_code is planner.build_plan.__code__ and event == "return":
                    seen["candidate"] = len(frame.f_locals.get("candidate_observations", []))
                    seen["unresolved"] = len(frame.f_locals.get("unresolved_observations", []))
                return trace
            previous = sys.gettrace()
            try:
                sys.settrace(trace)
                planned = planner.build_plan(case.inventory, case.output, case.snapshot, None, case.observations)
            finally:
                sys.settrace(previous)
            self.assertEqual({"candidate": 2, "unresolved": 1}, seen)
            self.assertEqual(0, planned["confirmed_multi_file_components"])

    def test_ambiguous_definition_or_signature_mismatch_never_produces_origin_edge(self):
        for variant in ("multiple", "column", "signature", "device"):
            with self.subTest(variant=variant), self.origin_case() as case:
                if variant == "multiple":
                    case.definition = case.definition.replace("Found 1", "Found 2")
                elif variant == "column":
                    case.definition = case.definition.replace(":3:8 ", ":3:7 ")
                elif variant == "device":
                    case.definition = case.definition.replace("\\\\?\\", "\\\\?\\UNC\\")
                else:
                    case.signature["signature"]["params"][0]["ty"] = "&mut str"
                code, error, _ = self.run_case(case)
                self.assertEqual(0, code, error)
                observations = json.loads(case.observations.read_text())["observations"]
                self.assertTrue(all(o["identity_status"] == "unresolved" and o["origin_binding_ref"] is None
                                    for o in observations))

    def test_origin_cfg_module_or_library_override_remains_unresolved(self):
        for variant in ("cfg", "override", "name", "macro_module"):
            with self.subTest(variant=variant), self.origin_case() as case:
                if variant == "cfg":
                    (case.root / collector.ORIGIN_MODULE).write_text('#[cfg(feature = "x")]\npub mod runtime;\n')
                elif variant == "override":
                    with (case.root / collector.ORIGIN_MANIFEST).open("a") as stream:
                        stream.write('[lib]\npath = "custom.rs"\n')
                elif variant == "name":
                    (case.root / collector.ORIGIN_MANIFEST).write_text('[package]\nname = "different"\n')
                else:
                    (case.root / collector.ORIGIN_MODULE).write_text("make_module! { pub mod runtime; }\n")
                code, error, _ = self.run_case(case)
                self.assertEqual(0, code, error)
                self.assertTrue(all(o["identity_status"] == "unresolved" for o in
                                    json.loads(case.observations.read_text())["observations"]))

    def test_origin_raw_query_hash_change_is_not_partial_success(self):
        with self.origin_case() as case:
            code, error, _ = self.run_case(case)
            self.assertEqual(0, code, error)
            evidence = json.loads(case.output.read_text())
            query = next(q for q in evidence["queries"] if q["tool"].endswith("_function_signature"))
            (case.root / query["artifact"]).write_text("{}")
            sources = collector.capture_sources(case.root, json.loads(case.plan.read_text())["source_files"], 10)
            with self.assertRaises(collector.CaptureIntegrityError):
                collector.build_origin_proof(case.root, case.artifacts, sources, evidence["queries"])

    def test_late_discovered_endpoint_is_path_only(self):
        with self.origin_case() as case:
            plan = json.loads(case.plan.read_text())
            plan["source_files"].remove("services/sample/src/consumer.rs")
            case.plan.write_text(json.dumps(plan))
            code, error, _ = self.run_case(case)
            self.assertEqual(0, code, error)
            records = json.loads(case.observations.read_text())["observations"]
            self.assertTrue(all(o["identity_status"] == "unresolved" for o in records))
            self.assertIn("subsequent bounded capture", " ".join(records[0]["reasons"]))

    def test_usage_error_prefix_cannot_be_promoted_to_candidate(self):
        with self.origin_case() as case:
            code, error, _ = self.run_case(case)
            self.assertEqual(0, code, error)
            observed = json.loads(case.observations.read_text())
            ref = observed["observations"][0]["origin_binding_ref"]
            origin = json.loads((case.root / ref["path"]).read_text())
            files = json.loads(case.plan.read_text())["source_files"]
            sources = collector.capture_sources(case.root, files, 10)
            state = {"prehashed": set(sources), "origin_reasons": [], "usages": [{
                "query_id": observed["observations"][0]["query_ids"][0], "rows": case.rows,
                "status": "error", "pagination": {"status": "unknown"}}]}
            records = collector.file_observations(case.root, sources, state, origin, ref)
            self.assertTrue(all(row["identity_status"] == "unresolved" for row in records))

    def test_observation_aliases_rejected_before_transport(self):
        for variant in ("input", "snapshot", "evidence", "artifact", "hardlink", "source_hardlink"):
            with self.subTest(variant=variant), self.origin_case() as case:
                if variant == "input":
                    case.observations = case.inventory
                elif variant == "snapshot":
                    case.observations = case.snapshot
                elif variant == "evidence":
                    case.observations = case.output
                elif variant == "artifact":
                    case.observations = case.artifacts / "hir-catalog.json"
                elif variant == "source_hardlink":
                    os.link(case.root / collector.ORIGIN_SOURCE, case.observations)
                else:
                    os.link(case.inventory, case.observations)
                code, _, connect = self.run_case(case)
                self.assertEqual(1, code)
                connect.assert_not_called()

    def test_discovered_source_alias_and_source_drift_preserve_observation_output(self):
        for variant in ("alias", "drift"):
            with self.subTest(variant=variant), self.origin_case() as case:
                case.observations.write_text(collector.ORIGIN_NAME)
                before = case.observations.read_bytes()
                if variant == "alias":
                    case.rows[0]["file"] = "observations.json"
                else:
                    case.drift = lambda: (case.root / "services/sample/src/consumer.rs").write_text("changed")
                code, _, _ = self.run_case(case)
                self.assertEqual(1, code)
                self.assertEqual(before, case.observations.read_bytes())
                self.assertFalse(case.output.exists())
                self.assertFalse(case.snapshot.exists())

    def test_observation_atomic_failure_preserves_previous_file(self):
        with self.origin_case() as case:
            case.observations.write_text("previous accepted observations")
            replace = collector.os.replace
            def fail_observation(source, target):
                if Path(target) == case.observations:
                    raise OSError("injected final observation replace failure")
                return replace(source, target)
            with patch.object(collector.os, "replace", side_effect=fail_observation):
                code, _, _ = self.run_case(case)
            self.assertEqual(1, code)
            self.assertEqual("previous accepted observations", case.observations.read_text())
            self.assertEqual([], list(case.root.rglob(".mcp-evidence-*.tmp")))


class ProducerAsyncTests(unittest.IsolatedAsyncioTestCase):
    async def test_origin_lookups_cannot_forward_arbitrary_or_fuzzy_requests(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            fake = session()
            definition = {"name": "rust-code-mcp_1mcp_find_definition", "inputSchema": {
                "properties": {k: {} for k in ("directory", "symbol_name", "exact")}, "required": []}}
            capture = collector.ProducerCapture(fake, [definition, tool("function_signature")], root, 10)
            for suffix, arguments in (
                ("find_definition", {"directory": str(root), "symbol_name": "validate_*", "exact": True}),
                ("find_definition", {"directory": str(root), "symbol_name": collector.ORIGIN_NAME, "exact": False}),
                ("find_definition", {"directory": str(root.parent), "symbol_name": collector.ORIGIN_NAME, "exact": True}),
                ("function_signature", {"directory": str(root), "target": "other::function"}),
            ):
                with patch.object(collector, "CANONICAL_WORKSPACE", root), self.assertRaises(collector.EvidenceError):
                    await capture.query("rust-code-mcp", suffix, arguments)
            fake.call_tool.assert_not_awaited()

    async def test_full_page_preserves_observed_missing_consumer_function(self):
        page = self.page(0, 14, list(range(14)), limit=50)
        for index in (5, 6):
            del page["usages"][index]["consumer_function"]
        # Sanitized full-page wire shape: TextContent JSON, no structured payload,
        # fourteen usages, with consumer_function omitted at the observed rows.
        fake = session(tools=[tool("who_uses")], results=[result(text=json.dumps(page))])
        with tempfile.TemporaryDirectory() as directory:
            capture = collector.ProducerCapture(fake, [tool("who_uses")], Path(directory), 1)
            rows, _, pagination = await collector.paged_who_uses(
                capture, Path("root"), "crate::target", limit=50)
            raw = json.loads((Path(directory) / "q000001.json").read_text())
            self.assertIsNone(raw["structuredContent"])
            self.assertEqual(page, json.loads(raw["content"][0]["text"]))
        self.assertEqual(("complete", 14, 14),
                         (pagination["status"], pagination["returned"], pagination["total"]))
        self.assertNotIn("consumer_function", rows[5])
        self.assertNotIn("consumer_function", rows[6])
        self.assertEqual("crate::f4", rows[4]["consumer_function"])
        fake.call_tool.assert_awaited_once()

    async def test_usage_missing_required_or_invalid_optional_fields_stays_unsupported(self):
        for field, value, remove in (("consumer_module", None, True),
                                     ("file", 42, False), ("start", True, False),
                                     ("consumer_function", None, False)):
            page = self.page(0, 1, [0])
            if remove:
                del page["usages"][0][field]
            else:
                page["usages"][0][field] = value
            with self.subTest(field=field):
                (_, _, pagination), _ = await self.capture_pages([page])
                self.assertNotEqual("complete", pagination["status"])
                self.assertEqual(0, pagination["returned"])

    async def test_serena_root_verified_on_owned_connection(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / "file.rs").write_text("pub fn f() -> u32 {\n    1\n}\n", encoding="utf-8")
            sources = collector.capture_sources(root, ["file.rs"], 1)
            queries = [
                ("The project with name 'AIForAll' at " + str(root) + " is activated.", {"id": "q1"}),
                ("Active project: AIForAll\nLanguage server status: ready", {"id": "q2"}),
                ({"Function": ["f"]}, {"id": "q3"}),
                ([{"name_path": "f", "kind": "Function", "relative_path": "file.rs",
                   "body_location": {"start_line": 0, "end_line": 2}}], {"id": "q4"}),
            ]
            capture = SimpleNamespace(query=AsyncMock(side_effect=queries))
            symbols, refs = await collector.discover_serena(
                capture, root, sources, [{"path": "file.rs", "name_path": "f"}], [])
            self.assertEqual("serena::file.rs::f", symbols[0]["qualified_name"])
            self.assertEqual(["q3", "q4"], refs[symbols[0]["id"]])
            self.assertEqual("initial_instructions", capture.query.call_args_list[0].args[1])
            self.assertEqual("get_current_config", capture.query.call_args_list[1].args[1])
            bad = SimpleNamespace(query=AsyncMock(side_effect=[
                ("The project at DIFFERENT is activated.", {"id": "q1"}), queries[1]]))
            with self.assertRaisesRegex(collector.EvidenceError, "root/readiness"):
                await collector.discover_serena(bad, root, sources, [], [])
            self.assertEqual(2, bad.query.await_count)

    async def test_capture_failure_keeps_raw_and_cannot_call_arbitrary_tool(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake = session(results=[result(error=True, text="Error: not ready")])
            capture = collector.ProducerCapture(fake, [tool("who_uses")], root, 1)
            with self.assertRaises(collector.EvidenceError):
                await capture.query("rust-code-mcp", "who_uses", {
                    "directory": "root", "target": "crate::f", "limit": 2, "offset": 0})
            self.assertTrue((root / "q000001.json").exists())
            self.assertEqual("error", capture.queries[0]["status"])
            with self.assertRaises(collector.EvidenceError):
                await capture.query("rust-code-mcp", "clear_cache", {})
            self.assertEqual(1, fake.call_tool.await_count)

    def page(self, offset, total, rows, limit=2):
        return {"target": "crate::target", "offset": offset, "total_match_count": total,
                "returned_match_count": len(rows), "limit": limit, "summary": False,
                "usages": [{"file": f"src/{i}.rs", "start": i, "end": i + 1,
                            "category": "Other", "consumer_module": "crate",
                            "consumer_function": f"crate::f{i}"} for i in rows]}

    async def capture_pages(self, pages, max_pages=8):
        fake = SimpleNamespace(query=AsyncMock(side_effect=[
            (page, {"id": f"q{i}", "status": "ok"}) for i, page in enumerate(pages)]))
        result = await collector.paged_who_uses(fake, Path("root"), "crate::target", 2, max_pages)
        return result, fake

    async def test_verified_multi_page_and_zero_scope(self):
        (rows, queries, pagination), fake = await self.capture_pages([
            self.page(0, 3, [0, 1]), self.page(2, 3, [2])])
        self.assertEqual(3, len(rows))
        self.assertEqual("complete", pagination["status"])
        self.assertEqual([0, 2], [c.args[2]["offset"] for c in fake.query.call_args_list])
        (_, _, pagination), _ = await self.capture_pages([self.page(0, 0, [])])
        self.assertEqual(("complete", 0), (pagination["status"], pagination["total"]))

    async def test_duplicate_gapped_truncated_inconsistent_pages_partial(self):
        for second in (self.page(2, 4, [0, 1]), self.page(1, 4, [2, 3]),
                       self.page(2, 5, [2, 3]), self.page(2, 4, [])):
            with self.subTest(second=second):
                (rows, queries, pagination), _ = await self.capture_pages([
                    self.page(0, 4, [0, 1]), second])
                self.assertEqual(2, len(rows))
                self.assertEqual("partial", pagination["status"])
                self.assertEqual("error", queries[-1]["status"])

    async def test_budget_and_unsupported_never_complete(self):
        (_, _, pagination), _ = await self.capture_pages([self.page(0, 3, [0, 1])], 1)
        self.assertEqual("partial", pagination["status"])
        (_, queries, pagination), _ = await self.capture_pages([{"items": []}])
        self.assertEqual("unknown", pagination["status"])
        self.assertEqual("unsupported", queries[0]["status"])

    async def test_empty_target_does_not_execute(self):
        fake = SimpleNamespace(query=AsyncMock())
        with self.assertRaises(collector.EvidenceError):
            await collector.paged_who_uses(fake, Path("root"), "")
        fake.query.assert_not_awaited()


if __name__ == "__main__":
    unittest.main()
