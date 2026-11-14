"""Offline transport/query evidence tests; no MCP server or runtime is started."""

import argparse
import asyncio
from contextlib import asynccontextmanager, redirect_stderr, redirect_stdout
import importlib.util
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import AsyncMock, Mock, patch


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "mcp-code-evidence.py"
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


if __name__ == "__main__":
    unittest.main()
