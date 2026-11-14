#!/usr/bin/env python3
"""Collect explicit, read-only MCP evidence; never infer mandatory edits."""

from __future__ import annotations

import argparse
import asyncio
from contextlib import asynccontextmanager
from datetime import datetime, timezone
from importlib.metadata import version, PackageNotFoundError
import ipaddress
import json
import math
import os
from pathlib import Path
import sys
import tempfile
from urllib.parse import urlsplit, urlunsplit

DEFAULT_URL = "http://127.0.0.1:3062/mcp?tags=rust-code-mcp"
ROOT = Path(__file__).resolve().parents[2]
MAX_PAGES = 32
MAX_TOOLS = 2048
MAX_REQUESTS = 32
MAX_INPUT_BYTES = 131072
MAX_RESULT_BYTES = 8 * 1024 * 1024
MAX_LIMIT = 200
MAX_OFFSET = 100000
SELECTORS = {
    "who_uses": {"target"},
    "who_uses_summary": {"target"},
    "who_calls": {"target"},
    "calls_from": {"caller"},
    "function_signature": {"target"},
    "who_imports": {"target"},
    "get_declared_reexports": {"module"},
    "get_exports": {"module", "consumer"},
    "workspace_stats": set(),
}
PAGED = set(SELECTORS) - {"function_signature", "workspace_stats"}


class EvidenceError(Exception):
    """An input, protocol, or evidence failure."""


def validate_url(value):
    """Accept literal loopback HTTP(S) only, without credentials or fragments."""
    try:
        parsed = urlsplit(value)
        host = parsed.hostname
        port = parsed.port
        if (parsed.scheme not in {"http", "https"} or not host
                or parsed.username is not None or parsed.password is not None
                or "#" in value or any(c.isspace() for c in value)):
            raise ValueError()
        # Normalize localhost without relying on ambient DNS configuration.
        host = "127.0.0.1" if host == "localhost" else host
        if "%" in host or not ipaddress.ip_address(host).is_loopback:
            raise ValueError()
        authority = f"[{host}]" if ":" in host else host
        if port is not None:
            authority += f":{port}"
        return urlunsplit((parsed.scheme, authority, parsed.path, parsed.query, ""))
    except (ValueError, TypeError):
        raise EvidenceError("URL must be loopback HTTP(S), without credentials or fragment") from None


def strict_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise EvidenceError(f"Duplicate JSON key: {key}")
        result[key] = value
    return result


def read_manifest(path, root=ROOT):
    if path.stat().st_size > MAX_INPUT_BYTES:
        raise EvidenceError("Manifest exceeds 128 KiB")
    try:
        data = json.loads(path.read_text(encoding="utf-8"),
                          object_pairs_hook=strict_object,
                          parse_constant=lambda _: (_ for _ in ()).throw(
                              EvidenceError("Non-finite JSON number")))
    except (UnicodeError, json.JSONDecodeError):
        raise EvidenceError("Manifest must be strict UTF-8 JSON") from None
    return validate_manifest(data, root)


def same_root(value, root):
    if not isinstance(value, str) or not Path(value).is_absolute():
        return False
    return Path(value).resolve() == root.resolve()


def validate_manifest(data, root=ROOT):
    if not isinstance(data, dict) or set(data) != {"workspace", "requests"}:
        raise EvidenceError("Manifest requires exactly workspace and requests")
    if not same_root(data["workspace"], root):
        raise EvidenceError("Manifest workspace must equal the collector repository root")
    requests = data["requests"]
    if not isinstance(requests, list) or not 1 <= len(requests) <= MAX_REQUESTS:
        raise EvidenceError("Manifest requires 1..32 requests")
    seen = set()
    validated = []
    for item in requests:
        if not isinstance(item, dict) or set(item) != {"id", "tool", "arguments"}:
            raise EvidenceError("Each request requires exactly id, tool, arguments")
        identity, tool, args = item["id"], item["tool"], item["arguments"]
        if (not isinstance(identity, str) or not identity or len(identity) > 128
                or identity in seen):
            raise EvidenceError("Request IDs must be unique nonempty strings (max 128)")
        seen.add(identity)
        if not isinstance(tool, str) or tool not in SELECTORS:
            raise EvidenceError("Tool is not in the curated semantic read-only allowlist")
        if not isinstance(args, dict):
            raise EvidenceError("arguments must be an object")
        required = SELECTORS[tool] | {"directory"}
        allowed = required | ({"limit", "offset", "summary"} if tool in PAGED else set())
        if not required <= set(args) or not set(args) <= allowed:
            raise EvidenceError(f"Unsupported or missing arguments for {tool}")
        if not same_root(args["directory"], root):
            raise EvidenceError("Every directory must equal the explicit workspace")
        # No path arguments are currently allowed. Selectors are qualified names,
        # never file paths, URLs, globs, or server configuration.
        for key in SELECTORS[tool]:
            value = args[key]
            if (not isinstance(value, str) or not value or len(value) > 1024
                    or any(c in value for c in "/\\\x00\r\n") or ".." in value
                    or "://" in value):
                raise EvidenceError(f"{key} must be a qualified symbol/module, not a path")
        prepared = dict(args, directory=str(root.resolve()))
        if tool in PAGED:
            prepared.setdefault("limit", 50)
            prepared.setdefault("offset", 0)
            for key, low, high in (("limit", 1, MAX_LIMIT), ("offset", 0, MAX_OFFSET)):
                if type(prepared[key]) is not int or not low <= prepared[key] <= high:
                    raise EvidenceError(f"{key} outside allowed bounds {low}..{high}")
            if "summary" in prepared and type(prepared["summary"]) is not bool:
                raise EvidenceError("summary must be boolean")
        validated.append(dict(item, arguments=prepared))
    return {"workspace": str(root.resolve()), "requests": validated}


def as_json(value):
    return value.model_dump(mode="json", by_alias=True, exclude_none=False)


async def catalog(session):
    """Follow MCP cursor pagination, detecting loops and finite budget exhaustion."""
    tools, seen, cursor = [], set(), None
    for _ in range(MAX_PAGES):
        page = await session.list_tools(cursor=cursor)
        tools.extend(as_json(tool) for tool in page.tools)
        if len(tools) > MAX_TOOLS:
            raise EvidenceError("Catalogue tool budget exceeded")
        cursor = page.nextCursor
        if cursor is None:
            return tools
        if not isinstance(cursor, str) or not cursor or cursor in seen:
            raise EvidenceError("Invalid or repeated catalogue cursor")
        seen.add(cursor)
    raise EvidenceError("Catalogue page budget exceeded")


def resolve_requests(requests, tools):
    """Preflight every request before executing any semantic tool."""
    resolved = []
    for request in requests:
        suffix = "_1mcp_" + request["tool"]
        matches = [tool for tool in tools if isinstance(tool.get("name"), str)
                   and tool["name"].endswith(suffix)]
        if len(matches) != 1:
            raise EvidenceError(f"Unknown or ambiguous tool suffix: {request['tool']}")
        tool = matches[0]
        # Restrict the namespace as well as the suffix. An unrelated advertised
        # tool with a readOnly annotation is not authority to execute it.
        if tool["name"] != "rust-code-mcp" + suffix:
            raise EvidenceError("Resolved tool is outside the rust-code-mcp namespace")
        schema = tool.get("inputSchema", {})
        properties = schema.get("properties", {})
        args = request["arguments"]
        if (not set(args) <= set(properties)
                or not set(schema.get("required", [])) <= set(args)):
            raise EvidenceError("Advertised tool arguments drifted; inspect catalogue")
        resolved.append(dict(request, exact_tool=tool["name"]))
    return resolved


def payloads(raw):
    """Yield structured and JSON-text payloads, without executing their content."""
    structured = raw.get("structuredContent")
    if structured is not None:
        yield structured
    for block in raw.get("content", []):
        if block.get("type") == "text":
            text = block.get("text", "")
            try:
                yield json.loads(text)
            except (json.JSONDecodeError, TypeError):
                if isinstance(text, str) and text.lstrip().lower().startswith(
                        ("error:", "failed:", "permission denied")):
                    yield {"error": text}


def result_failed(raw):
    if raw.get("isError") is True:
        return True
    for payload in payloads(raw):
        if isinstance(payload, dict):
            if (payload.get("isError") is True or payload.get("success") is False
                    or payload.get("status") in ("error", "failed")
                    or bool(payload.get("error"))):
                return True
    return False


async def collect(session, requests, evidence):
    records = []
    for request in requests:
        record = {
            **request, "server": evidence["server"],
            "workspace": evidence["workspace"], "status": "not_executed",
            "coverage": "unknown; server index/features/macros/dynamic dispatch/tests not verified",
            "pagination": {
                "status": "unknown",
                "reason": "Semantic result envelope not verified; one bounded page only",
            },
        }
        evidence["requests"].append(record)
        records.append(record)
    for request, record in zip(requests, records):
        record["status"] = "pending"
        try:
            result = await session.call_tool(request["exact_tool"],
                                             arguments=request["arguments"])
            raw = as_json(result)
            if len(json.dumps(raw).encode("utf-8")) > MAX_RESULT_BYTES:
                raise EvidenceError("Tool result exceeds 8 MiB evidence budget")
            record["result"] = raw
            record["status"] = "tool_error" if result_failed(raw) else "received"
            if record["status"] == "tool_error":
                raise EvidenceError(f"Tool reported failure for request {request['id']}")
        except BaseException as exc:
            if record["status"] == "pending":
                record["status"] = "timeout" if isinstance(
                    exc, (TimeoutError, asyncio.CancelledError)) else "transport_error"
            raise


def load_sdk():
    try:
        if version("mcp") != "1.21.1":
            raise EvidenceError("Requires mcp==1.21.1; use the dedicated requirements file")
        from mcp import ClientSession
        from mcp.client.streamable_http import streamablehttp_client
        import httpx
    except (ImportError, PackageNotFoundError):
        raise EvidenceError(
            "Missing dependency: install requirements-mcp-code-evidence.txt in a dedicated venv"
        ) from None
    return ClientSession, streamablehttp_client, httpx


@asynccontextmanager
async def connect(url, timeout):
    ClientSession, streamablehttp_client, httpx = load_sdk()

    async def guard(request):
        validate_url(str(request.url))

    def factory(headers=None, timeout=None, auth=None):
        return httpx.AsyncClient(
            headers=headers, timeout=timeout, auth=auth, trust_env=False,
            follow_redirects=False, event_hooks={"request": [guard]},
        )

    async with streamablehttp_client(
        url, timeout=timeout, sse_read_timeout=timeout,
        terminate_on_close=True, httpx_client_factory=factory,
    ) as (read, write, _):
        async with ClientSession(read, write) as session:
            yield session


async def run(args, manifest, evidence):
    # One finite deadline includes connection, initialization, catalogue,
    # sequential requests and ordinary session cleanup. No retries.
    async with asyncio.timeout(args.timeout):
        async with connect(args.url, args.timeout) as session:
            initialized = await session.initialize()
            evidence["server"] = as_json(initialized.serverInfo)
            tools = await catalog(session)
            if args.command == "catalog":
                evidence.update(tools=tools, pagination={"status": "complete",
                                "scope": "advertised catalogue only"})
            else:
                resolved = resolve_requests(manifest["requests"], tools)
                await collect(session, resolved, evidence)
    evidence["status"] = "received"


def output_path(value, protected):
    if value is None:
        return None
    path = Path(value).resolve()
    if path in protected or path.suffix.lower() != ".json":
        raise EvidenceError("Output/report must be a .json file distinct from every input")
    return path


def atomic_json(path, data):
    """Replace only after complete serialization and successful flush."""
    text = json.dumps(data, indent=2, ensure_ascii=False, allow_nan=False) + "\n"
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", newline="\n",
                                         dir=path.parent, prefix=".mcp-evidence-",
                                         suffix=".tmp", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(text)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default=DEFAULT_URL)
    parser.add_argument("--timeout", type=float, default=60,
                        help="Whole-operation deadline in seconds (1..300)")
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("catalog", "collect"):
        command = sub.add_parser(name)
        command.add_argument("--output", help="Atomic accepted evidence JSON")
        command.add_argument("--report", help="Separate failure-only diagnostic JSON")
        if name == "collect":
            command.add_argument("--requests", required=True, type=Path)
    args = parser.parse_args(argv)
    evidence = {"format_version": 1, "status": "pending",
                "collected_at": datetime.now(timezone.utc).isoformat(),
                "server": None, "workspace": str(ROOT), "requests": []}
    report = None
    try:
        args.url = validate_url(args.url)
        if not math.isfinite(args.timeout) or not 1 <= args.timeout <= 300:
            raise EvidenceError("timeout must be finite and within 1..300 seconds")
        protected = {Path(__file__).resolve()}
        if args.command == "collect":
            protected.add(args.requests.resolve())
        output = output_path(args.output, protected)
        report = output_path(args.report, protected | ({output} if output else set()))
        manifest = read_manifest(args.requests) if args.command == "collect" else None
        evidence.update(url=args.url, workspace=manifest["workspace"] if manifest else str(ROOT))
        asyncio.run(run(args, manifest, evidence))
        if output:
            atomic_json(output, evidence)
        else:
            print(json.dumps(evidence, indent=2, ensure_ascii=False, allow_nan=False))
        return 0
    except Exception as exc:
        # Exception strings from untrusted servers can contain secrets or arbitrary
        # text. Retain raw tool errors in the explicit report, not stderr.
        evidence.update(status="failed", failure={
            "type": type(exc).__name__,
            "message": str(exc) if isinstance(exc, EvidenceError) else
            "Operation failed; inspect server availability, permissions, and deadline",
        })
        print(f"mcp-code-evidence: {evidence['failure']['message']}", file=sys.stderr)
        if report:
            try:
                atomic_json(report, evidence)
            except Exception:
                print("mcp-code-evidence: could not write failure report", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
