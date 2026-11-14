#!/usr/bin/env python3
"""Collect explicit, read-only MCP evidence; never infer mandatory edits."""

from __future__ import annotations

import argparse
import asyncio
from contextlib import asynccontextmanager
from datetime import datetime, timezone
from importlib.metadata import version, PackageNotFoundError
import ipaddress
import hashlib
import json
import math
import os
import re
from pathlib import Path
import sys
import tempfile
from urllib.parse import urlsplit, urlunsplit

DEFAULT_URL = "http://127.0.0.1:3062/mcp?tags=rust-code-mcp"
CANONICAL_WORKSPACE = Path("C:/Users/djden/source/repos/AIForAll")
ROOT = Path(__file__).resolve().parents[2]
MAX_PAGES = 32
MAX_TOOLS = 2048
MAX_REQUESTS = 32
MAX_INPUT_BYTES = 131072
MAX_RESULT_BYTES = 8 * 1024 * 1024
MAX_SOURCE_BYTES = 128 * 1024 * 1024
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


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def strict_json(path):
    if path.stat().st_size > 64 * 1024 * 1024:
        raise EvidenceError("Producer input exceeds 64 MiB")
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=strict_object,
                      parse_constant=lambda _: (_ for _ in ()).throw(
                          EvidenceError("Non-finite JSON number")))


class SourcePathError(EvidenceError):
    def __init__(self, message, status="unknown"):
        super().__init__(message)
        self.status = status


def source_path(root, raw):
    """Normalize repeated separators, but never dot, case, or symlink aliases."""
    if not isinstance(raw, str) or not raw or "\x00" in raw:
        raise SourcePathError("Invalid source path")
    spelling = raw.replace("\\", "/")
    if spelling.startswith("//"):
        raise SourcePathError("UNC/device source paths are unresolved", "outside_root")
    spelling = re.sub("/+", "/", spelling)
    if spelling.startswith(("//", "/")) or re.match(r"^[A-Za-z]:", spelling):
        prefix = str(root).replace("\\", "/").rstrip("/") + "/"
        if not spelling.startswith(prefix):
            raise SourcePathError("Source outside workspace", "outside_root")
        spelling = spelling[len(prefix):]
    parts = re.split("/+", spelling)
    if any(part in ("", ".", "..") or part.endswith((".", " ")) or ":" in part
           for part in parts):
        raise SourcePathError("Dot/device/path aliases are unresolved", "ambiguous")
    path = root.joinpath(*parts)
    resolved = path.resolve()
    if not resolved.is_relative_to(root.resolve()):
        raise SourcePathError("Source outside root", "outside_root")
    if resolved != path.absolute():
        raise SourcePathError("Symlink source alias", "ambiguous")
    # Windows Path equality folds case; inspect actual directory spellings too.
    parent = root
    for part in parts:
        names = [p.name for p in parent.iterdir() if p.name.casefold() == part.casefold()]
        if not names:
            raise SourcePathError("Missing source")
        if names != [part]:
            raise SourcePathError("Case source alias", "ambiguous")
        parent = parent / part
    if not path.is_file():
        raise SourcePathError("Source is not a file")
    return "/".join(parts), path


def git_head(root):
    """Read Git metadata without invoking Git, a shell, or OpenCode."""
    git = root / ".git"
    if git.is_file():
        text = git.read_text(encoding="utf-8").strip()
        if not text.startswith("gitdir: "):
            raise EvidenceError("Unsupported Git metadata")
        git = (root / text[8:]).resolve()
    value = (git / "HEAD").read_text(encoding="ascii").strip()
    if value.startswith("ref: "):
        ref = value[5:]
        if not re.fullmatch(r"refs/[A-Za-z0-9_./-]+", ref) or ".." in ref:
            raise EvidenceError("Invalid Git ref")
        common = git
        if (git / "commondir").exists():
            common = (git / (git / "commondir").read_text().strip()).resolve()
        if (common / ref).exists():
            value = (common / ref).read_text().strip()
        else:
            matches = [line.split()[0] for line in (common / "packed-refs").read_text().splitlines()
                       if line.endswith(" " + ref)]
            value = matches[0] if len(matches) == 1 else ""
    if not re.fullmatch("[0-9a-f]{40}", value):
        raise EvidenceError("Cannot establish full workspace HEAD")
    return value


def decode_wire(raw, serena=False):
    """Decode only verified MCP envelopes; disagreeing dual payloads fail closed."""
    if result_failed(raw):
        raise EvidenceError("Tool error envelope")
    candidates = []
    if raw.get("structuredContent") is not None:
        value = raw["structuredContent"]
        if serena and isinstance(value, dict) and set(value) == {"result"}:
            value = value["result"]
        candidates.append(value)
    for block in raw.get("content", []):
        if block.get("type") != "text":
            raise EvidenceError("Unsupported non-text tool payload")
        candidates.append(block.get("text"))
    decoded = []
    for value in candidates:
        if isinstance(value, str):
            try:
                value = json.loads(value, object_pairs_hook=strict_object)
            except json.JSONDecodeError:
                if not serena:
                    raise EvidenceError("Unsupported non-JSON HIR payload") from None
        decoded.append(value)
    if not decoded or any(value != decoded[0] for value in decoded[1:]):
        raise EvidenceError("Missing or conflicting tool payloads")
    return decoded[0]


def coverage():
    return {key: "unknown" for key in ("private", "macros", "traits", "features", "tests", "freshness")}


class ProducerCapture:
    """One bounded, sequential capture; raw artifacts survive partial queries."""

    def __init__(self, session, tools, directory, budget):
        self.session, self.tools, self.directory = session, tools, directory
        self.remaining = budget
        self.queries = []
        self.next_id = 0

    async def query(self, namespace, suffix, arguments, serena=False):
        exact = f"{namespace}_1mcp_{suffix}"
        matches = [tool for tool in self.tools if tool.get("name", "").endswith("_1mcp_" + suffix)]
        if len(matches) != 1 or matches[0]["name"] != exact:
            raise EvidenceError("Unknown or ambiguous curated producer tool")
        allowed = {
            ("rust-code-mcp", "who_uses"), ("serena", "initial_instructions"),
            ("serena", "get_current_config"), ("serena", "get_symbols_overview"),
            ("serena", "find_symbol"),
        }
        if (namespace, suffix) not in allowed:
            raise EvidenceError("Producer tool is not curated")
        if suffix == "find_symbol" and not arguments.get("name_path_pattern"):
            raise EvidenceError("Empty symbol patterns are forbidden")
        schema = matches[0].get("inputSchema", {})
        if (not set(arguments) <= set(schema.get("properties", {}))
                or not set(schema.get("required", [])) <= set(arguments)):
            raise EvidenceError("Producer tool schema drift")
        if self.remaining <= 0:
            raise EvidenceError("Producer query budget exhausted")
        self.remaining -= 1
        self.next_id += 1
        identity = f"q{self.next_id:06d}"
        artifact = self.directory / (identity + ".json")
        query = {"id": identity, "tool": exact, "arguments": arguments,
                 "artifact": str(artifact), "artifact_sha256": None, "status": "error",
                 "pagination": {"status": "unknown", "pages": 1, "returned": 0,
                                "total": None, "reason": "Unverified result envelope"},
                 "coverage": coverage()}
        self.queries.append(query)
        try:
            raw = as_json(await self.session.call_tool(exact, arguments=arguments))
            if len(json.dumps(raw).encode()) > MAX_RESULT_BYTES:
                raise EvidenceError("Producer result exceeds evidence budget")
            atomic_json(artifact, raw)
            query["artifact_sha256"] = sha256(artifact)
            value = decode_wire(raw, serena)
            query["status"] = "ok"
            return value, query
        except Exception as exc:
            if not artifact.exists():
                atomic_json(artifact, {"failure": type(exc).__name__})
                query["artifact_sha256"] = sha256(artifact)
            raise


async def paged_who_uses(capture, workspace, target, limit=50, max_pages=8):
    """Only who_uses has a verified semantic page contract."""
    if not isinstance(target, str) or not target.strip() or any(c in target for c in "/\\\r\n"):
        raise EvidenceError("who_uses requires a nonempty qualified target")
    offset, total, rows, seen, queries = 0, None, [], set(), []
    reason, state = "Page budget exhausted", "partial"
    for _ in range(max_pages):
        try:
            data, query = await capture.query("rust-code-mcp", "who_uses", {
                "directory": str(workspace), "target": target,
                "offset": offset, "limit": limit, "summary": False})
            queries.append(query)
            if not isinstance(data, dict) or not isinstance(data.get("usages"), list):
                query["status"] = "unsupported"
                raise EvidenceError("Unsupported who_uses envelope")
            count, current_total = data.get("returned_match_count"), data.get("total_match_count")
            if (type(count) is not int or type(current_total) is not int or count < 0
                    or current_total < 0 or type(data.get("offset")) is not int
                    or type(data.get("limit")) is not int or data.get("offset") != offset
                    or data.get("limit") != limit or data.get("summary") is not False
                    or data.get("target") != target or count != len(data["usages"])
                    or count > limit or offset + count > current_total
                    or (total is not None and total != current_total)):
                raise EvidenceError("Inconsistent/gapped who_uses pagination")
            total = current_total
            identities = []
            for row in data["usages"]:
                if not isinstance(row, dict) or not all(
                        key in row for key in ("file", "start", "end", "category",
                                               "consumer_module")):
                    query["status"] = "unsupported"
                    raise EvidenceError("Unsupported who_uses usage")
                if (not all(isinstance(row[key], str) and row[key] for key in
                            ("file", "category", "consumer_module"))
                        or type(row["start"]) is not int or type(row["end"]) is not int
                        or not 0 <= row["start"] <= row["end"]
                        or ("consumer_function" in row and (
                            not isinstance(row["consumer_function"], str) or not row["consumer_function"]))):
                    query["status"] = "unsupported"
                    raise EvidenceError("Unsupported who_uses usage")
                identity = json.dumps(dict(row, file=re.sub(
                    "/+", "/", str(row["file"]).replace("\\", "/"))), sort_keys=True)
                if identity in seen or identity in identities:
                    raise EvidenceError("Repeated/duplicate who_uses page identity")
                identities.append(identity)
            if count == 0 and offset != total:
                raise EvidenceError("Non-progress/truncated who_uses page")
            seen.update(identities)
            rows.extend(data["usages"])
            offset += count
            if offset == total:
                state, reason = "complete", "Verified indexed-scope count sequence only"
                break
        except EvidenceError as exc:
            reason = str(exc)
            state = "partial" if rows else "unknown"
            if queries and queries[-1]["status"] == "ok":
                queries[-1]["status"] = "error"
            break
    pagination = {"status": state, "pages": len(queries), "returned": len(rows),
                  "total": total, "reason": reason}
    for query in queries:
        query["pagination"] = dict(pagination)
        if state != "complete" and query["status"] == "ok":
            query["status"] = "partial"
    return rows, queries, pagination


def source_lines(text):
    # Rust/source protocol lines advance on LF, not Unicode NEL/LS/PS.
    parts = text.split("\n")
    return [part + "\n" for part in parts[:-1]] + ([parts[-1]] if parts[-1] else [])


def offset_in_source(text, line, column, unit):
    """Validate and convert explicit native columns to Unicode scalar offsets."""
    lines = source_lines(text)
    if type(line) is not int or type(column) is not int or line < 1 or column < 0:
        raise EvidenceError("Invalid source coordinates")
    if line == len(lines) + 1 and column == 0 and (not text or text.endswith("\n")):
        return len(text)
    if line > len(lines):
        raise EvidenceError("Coordinate outside source")
    content = lines[line - 1].rstrip("\r\n")
    if unit == "unicode_scalar":
        prefix = content[:column]
        if column > len(content):
            raise EvidenceError("Column outside source")
    elif unit in ("utf8_byte", "utf16"):
        encoding = "utf-8" if unit == "utf8_byte" else "utf-16-le"
        width = column if unit == "utf8_byte" else column * 2
        data = content.encode(encoding)
        if width > len(data):
            raise EvidenceError("Column outside source")
        try:
            prefix = data[:width].decode(encoding)
        except UnicodeDecodeError:
            raise EvidenceError("Column splits a Unicode character") from None
    else:
        raise EvidenceError("Unknown column unit")
    return sum(map(len, lines[:line - 1])) + len(prefix)


def serena_extent(symbol, text):
    """Conservative whole-line extent for source-crosschecked simple functions."""
    location = symbol.get("body_location", {})
    start, end = location.get("start_line"), location.get("end_line")
    lines = source_lines(text)
    if (symbol.get("kind") != "Function" or type(start) is not int or type(end) is not int
            or not 0 <= start <= end < len(lines)):
        raise EvidenceError("Unsupported symbol body range/kind")
    body = "".join(lines[start:end + 1])
    name = symbol.get("name_path", "").split("/")[-1]
    # Fail closed for macro expansion, nested declarations and syntax requiring
    # a full Rust parser. Whole-line endpoints must independently match source.
    declaration = re.search(
        r"(?m)^[ \t]*(?:(?:pub(?:\([^)\n]*\))?|async|unsafe)\s+)*fn\s+"
        + re.escape(name) + r"\s*(?:<|\()", body)
    if not declaration or not lines[end].strip().startswith("}") or lines[end].strip() != "}":
        raise EvidenceError("Unverified inclusive body endpoint")
    prefix = body[:declaration.start()]
    if any(line.strip() and not line.lstrip().startswith("///") for line in prefix.split("\n")):
        raise EvidenceError("Unverified documentation/attribute boundary")
    tail = body[declaration.end():]
    if re.search(r"\b(?:fn|struct|enum|impl|mod|trait|const|static|type|union|use|extern)\b"
                 r"|\w+\s*!|['\"#|]|//|/\*|\*/", tail):
        raise EvidenceError("Macro/nested/string syntax requires richer source verification")
    if tail.count("{") != tail.count("}") or "{" not in tail:
        raise EvidenceError("Unverified function extent")
    has_newline = lines[end].endswith("\n")
    return {"start_line": start + 1, "start_column": 0,
            "end_line": end + 2 if has_newline else end + 1,
            "end_column": 0 if has_newline else len(lines[end]),
            "column_unit": "unicode_scalar"}


def diagnostic_rows(inventory):
    if (not isinstance(inventory, dict) or type(inventory.get("schema_version")) is not int
            or inventory["schema_version"] != 1):
        raise EvidenceError("Expected overlap inventory schema_version 1")
    for namespace in ("clippy", "sonar"):
        records = inventory.get(namespace)
        if not isinstance(records, list):
            raise EvidenceError("Inventory requires explicit clippy and sonar arrays")
        for record in records:
            if not isinstance(record, dict) or not isinstance(record.get("id"), str):
                raise EvidenceError("Invalid inventory diagnostic identity")
            spans = record.get("primary_spans", []) if namespace == "clippy" else [
                {"file": record.get("file"), "range": record.get("range")}]
            yield namespace + ":" + record["id"], namespace, spans


def map_diagnostics(inventory, workspace, sources, symbols, refs, bound, contexts=None):
    mappings = []
    seen = set()
    for key, namespace, spans in diagnostic_rows(inventory):
        if key in seen:
            raise EvidenceError("Duplicate inventory diagnostic ID")
        seen.add(key)
        mapping = {"diagnostic_key": key, "status": "unknown", "symbols": [],
                   "evidence_refs": [], "reasons": []}
        mappings.append(mapping)
        if len(spans) != 1 or not spans[0].get("range"):
            mapping["reasons"].append("Missing or multi-primary range")
            continue
        try:
            path, _ = source_path(workspace, spans[0]["file"])
        except EvidenceError as exc:
            mapping.update(status=getattr(exc, "status", "unknown"), reasons=[str(exc)])
            continue
        if path not in sources:
            mapping["reasons"].append("Source not captured before queries")
            continue
        text = sources[path]["text"]
        context = (contexts or {}).get(key)
        unit = "unicode_scalar" if namespace == "clippy" else None
        if context and context["data"].get("source_sha256") == sources[path]["sha256"]:
            mapping["evidence_refs"].append(context["reference"])
            if namespace == "sonar":
                unit = context["data"].get("inventory_column_unit")
        if unit not in ("unicode_scalar", "utf8_byte", "utf16"):
            mapping["reasons"].append("Sonar native column convention not independently verified")
            continue
        try:
            line, col, end_line, end_col = spans[0]["range"]
            begin = offset_in_source(text, line, col, unit)
            end = offset_in_source(text, end_line, end_col, unit)
            if end <= begin:
                raise EvidenceError("Empty or reversed diagnostic range")
        except (EvidenceError, ValueError, TypeError) as exc:
            mapping["reasons"].append("Unverified diagnostic coordinates: " + str(exc))
            continue
        candidates = []
        for symbol in symbols:
            if symbol["path"] != path:
                continue
            r = symbol["range"]
            a = offset_in_source(text, r["start_line"], r["start_column"], r["column_unit"])
            b = offset_in_source(text, r["end_line"], r["end_column"], r["column_unit"])
            if a <= begin < b and begin <= end <= b:
                candidates.append(symbol)
        mapping["symbols"] = candidates
        mapping["evidence_refs"] = sorted({ref for s in candidates for ref in refs[s["id"]]}) + mapping["evidence_refs"]
        if path not in bound:
            mapping.update(status="stale", reasons=["Historical inventory source equivalence not proven"])
        elif len(candidates) == 1:
            mapping.update(status="exact", reasons=["Unique source-crosschecked simple function extent"])
        elif candidates:
            mapping.update(status="ambiguous", reasons=["Overlapping enclosing symbols"])
        else:
            mapping["reasons"].append("No supported verified enclosing symbol")
    return mappings


def capture_sources(workspace, paths, budget):
    sources = {}
    total_bytes = 0
    for raw in paths:
        path, absolute = source_path(workspace, raw)
        if path in sources:
            raise EvidenceError("Duplicate/aliased source path")
        if len(sources) >= budget or absolute.stat().st_size > MAX_RESULT_BYTES:
            raise EvidenceError("Source capture budget exceeded")
        data = absolute.read_bytes()
        total_bytes += len(data)
        if total_bytes > MAX_SOURCE_BYTES:
            raise EvidenceError("Aggregate source capture exceeds 128 MiB")
        sources[path] = {"sha256": hashlib.sha256(data).hexdigest(),
                         "text": data.decode("utf-8"), "path": absolute, "bytes": len(data)}
    return sources


def verify_sources(sources):
    changed = [path for path, value in sources.items() if sha256(value["path"]) != value["sha256"]]
    if changed:
        raise EvidenceError("Sources changed during collection: " + ", ".join(changed[:8]))


def verify_output_source_separation(outputs, sources):
    for output in outputs:
        for source in sources.values():
            path = source["path"].resolve()
            if output == path or (output.exists() and output.samefile(path)):
                raise EvidenceError("Outputs cannot overwrite or alias captured source inputs")


def capture_name(workspace, path):
    """Contract references are root-relative; native tool arguments stay native."""
    return source_path(workspace, str(Path(path).resolve()))[0]


def simple_file_functions(text):
    """Replay the initial proof method's explicit, whole-file syntax subset."""
    clean = []
    for line in text.split("\n"):
        if line.lstrip().startswith("///") and not line.lstrip().startswith("////"):
            if any(marker in line.lower() for marker in
                   ("@generated", "automatically generated", "do not edit")):
                raise EvidenceError("Generated documentation/source is unsupported")
            clean.append(" " * len(line))
        else:
            clean.append(line)
    clean = "\n".join(clean)
    if re.search(r'//|/\*|\*/|["\'`#!$|\\]|[^\x00-\x7f]', clean):
        raise EvidenceError("Whole-file comment/string/macro/closure syntax is unsupported")
    header = re.compile(
        r"(?:pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+"
        r"([A-Za-z_][A-Za-z_0-9]*)\s*\(([A-Za-z_0-9\s,:&]*)\)\s*"
        r"(?:->\s*[A-Za-z_][A-Za-z_0-9:]*(?:\s*)?)?\{")
    items = re.compile(r"\b(?:fn|impl|trait|mod|struct|enum|union|type|const|static|"
                       r"use|extern|async|move|macro_rules)\b")
    cursor, functions = 0, []
    while cursor < len(clean):
        if clean[cursor].isspace():
            cursor += 1
            continue
        match = header.match(clean, cursor)
        if not match or items.search(match.group(2)):
            raise EvidenceError("Unsupported top-level syntax or competing declaration")
        opening, depth, end = match.end() - 1, 1, match.end()
        while end < len(clean) and depth:
            depth += (clean[end] == "{") - (clean[end] == "}")
            end += 1
        body = clean[opening + 1:end - 1]
        if (depth or items.search(body)
                or not re.fullmatch(r"[A-Za-z_0-9\s{}();,:.+*/%=&<>?\-]*", body)):
            raise EvidenceError("Unsupported nested/competing declaration or function body")
        functions.append(match.group(1))
        cursor = end
    if not functions:
        raise EvidenceError("No supported whole-file functions")
    return functions


def localization_proofs(mappings, queries, workspace, directory, sources, contexts):
    """Emit the shared proof only after replaying the bounded local checks."""
    indexed_queries = {query["id"]: query for query in queries}
    inputs = []
    for mapping in mappings:
        if mapping["status"] != "exact":
            continue
        symbol = mapping["symbols"][0]
        name = symbol["qualified_name"].split("::")[-1]
        extent = symbol["range"]
        native_end = extent["end_line"] - (2 if extent["end_column"] == 0 else 1)
        try:
            functions = simple_file_functions(sources[symbol["path"]]["text"])
            if functions.count(name) != 1:
                raise EvidenceError("Whole-file selected function name is ambiguous")
            checked = serena_extent({
                "kind": symbol["kind"], "name_path": name,
                "body_location": {"start_line": extent["start_line"] - 1, "end_line": native_end},
            }, sources[symbol["path"]]["text"])
        except EvidenceError as exc:
            mapping.update(status="unknown", reasons=mapping["reasons"] + [str(exc)])
            continue
        if checked != extent:
            raise EvidenceError("Localization proof endpoint replay mismatch")
        query_ids = [ref for ref in mapping["evidence_refs"] if isinstance(ref, str)
                     and indexed_queries[ref]["tool"] == "serena_1mcp_find_symbol"
                     and indexed_queries[ref]["status"] == "ok"]
        if not query_ids:
            mapping.update(status="unknown", reasons=mapping["reasons"] + [
                "No successful decoded source-symbol query for local proof"])
            continue
        unit = "unicode_scalar" if mapping["diagnostic_key"].startswith("clippy:") else (
            contexts[mapping["diagnostic_key"]]["data"]["inventory_column_unit"])
        proof = {
            "schema_version": 1, "kind": "clippy-localization-proof",
            "diagnostic_key": mapping["diagnostic_key"], "source_path": symbol["path"],
            "source_sha256": symbol["source_sha256"], "symbol_id": symbol["id"],
            "inventory_column_unit": unit,
            "local_discovery": {
                "method": "source_crosschecked_simple_function_v1", "scope_range": extent,
                "candidate_symbol_ids": [candidate["id"] for candidate in mapping["symbols"]],
                "query_ids": query_ids,
                "checked_predicates": {key: True for key in (
                    "source_binding_verified", "source_hash_stable_before_after",
                    "diagnostic_coordinates_verified", "symbol_endpoints_source_verified",
                    "diagnostic_fully_enclosed", "relevant_local_candidates_exhausted",
                    "nested_or_competing_symbols_excluded")},
                "limitations": [],
            },
        }
        proof_path = directory / ("localization-" + str(len(inputs) + 1) + ".json")
        atomic_json(proof_path, proof)
        ref = {"path": capture_name(workspace, proof_path), "sha256": sha256(proof_path), "locator": "/"}
        mapping["evidence_refs"].append(ref)
        inputs.append({"role": "localization_proof", "path": ref["path"], "sha256": ref["sha256"]})
    return inputs


def bound_sources(binding, inventory_hash, head, sources, workspace=None):
    """Only explicit independently captured hash metadata can bind old diagnostics."""
    if binding is None:
        return set()
    if (binding.get("inventory_sha256") != inventory_hash or binding.get("head") != head
            or binding.get("method") not in ("fresh_capture", "independent_equivalence")
            or not isinstance(binding.get("evidence_refs"), list) or not binding["evidence_refs"]):
        raise EvidenceError("Invalid independent inventory source binding")
    for ref in binding["evidence_refs"]:
        if (not isinstance(ref, dict) or set(ref) != {"path", "sha256", "locator"}
                or not isinstance(ref["locator"], str) or not ref["locator"]
                or sha256((workspace or ROOT) / ref["path"]) != ref["sha256"]):
            raise EvidenceError("Inventory binding requires hash-verified independent context references")
    result = set()
    for item in binding.get("files", []):
        path = item["path"]
        if path not in sources or item["sha256"] != sources[path]["sha256"]:
            raise EvidenceError("Stale or absent inventory source hash")
        result.add(path)
    return result


def producer_plan(plan, workspace, budget):
    if (not isinstance(plan, dict) or type(plan.get("schema_version")) is not int
            or plan["schema_version"] != 1
            or not set(plan) <= {"schema_version", "source_files", "symbols", "targets", "configuration"}
            or not isinstance(plan.get("source_files"), list)):
        raise EvidenceError("Producer plan requires schema_version 1 and explicit source_files")
    sources = capture_sources(workspace, plan["source_files"], budget)
    symbols, targets = plan.get("symbols", []), plan.get("targets", [])
    if not isinstance(symbols, list) or not isinstance(targets, list):
        raise EvidenceError("Plan symbols/targets must be arrays")
    normalized_seeds = []
    for seed in symbols:
        if not isinstance(seed, dict) or set(seed) != {"path", "name_path"}:
            raise EvidenceError("Symbol seeds require exactly path and name_path")
        path, _ = source_path(workspace, seed["path"])
        if path not in sources or not isinstance(seed["name_path"], str) or not seed["name_path"].strip():
            raise EvidenceError("Symbol seed must have a captured path and nonempty name")
        normalized_seeds.append(dict(seed, path=path))
    for target in targets:
        if (not isinstance(target, str) or not target.strip() or len(target) > 1024
                or any(c in target for c in "/\\\r\n") or ".." in target):
            raise EvidenceError("Plan HIR targets must be explicit nonempty qualified names")
    return sources, normalized_seeds, targets


async def discover_serena(capture, workspace, sources, seeds, limitations):
    initial, iq = await capture.query("serena", "initial_instructions", {}, True)
    config, cq = await capture.query("serena", "get_current_config", {}, True)
    # These are the verified 2.1.0 text formats, not instructions to execute.
    root_marker = " at " + str(workspace) + " is activated."
    if (not isinstance(initial, str) or root_marker not in initial
            or not isinstance(config, str) or "Language server status: ready" not in config
            or "Active project: AIForAll\n" not in config):
        raise EvidenceError("Serena root/readiness not verified; never activate a project")
    for query in (iq, cq):
        query["pagination"] = {"status": "not_applicable", "pages": 1, "returned": 1,
                               "total": None, "reason": "Connection root/readiness text"}
    symbols, refs = [], {}
    for path in sorted({seed["path"].replace("\\", "/") for seed in seeds}):
        overview, oq = await capture.query("serena", "get_symbols_overview", {
            "relative_path": path, "depth": 0}, True)
        if not isinstance(overview, dict) or not all(
                isinstance(names, list) and all(isinstance(n, str) for n in names)
                for names in overview.values()):
            oq["status"] = "unsupported"
            limitations.append(f"{path}: unsupported symbol overview; no exact mapping")
            continue
        names = set(n for group in overview.values() for n in group)
        text = sources[path]["text"]
        # The lexical check only supplies candidates for MCP verification and
        # detects omitted/nested functions. It never establishes symbol ranges.
        lexical = set(re.findall(r"\bfn\s+([A-Za-z_][A-Za-z_0-9]*)", text))
        selected = {seed["name_path"] for seed in seeds
                    if seed["path"].replace("\\", "/") == path}
        all_functions = set(overview.get("Function", [])) | lexical | selected
        file_symbols = []
        incomplete = False
        for name in sorted(all_functions):
            if name not in names and name not in lexical:
                limitations.append(f"{path}: seed {name} not discovered; no nearest-name mapping")
                incomplete = True
                continue
            try:
                found, query = await capture.query("serena", "find_symbol", {
                    "name_path_pattern": name, "relative_path": path,
                    "include_body": False, "depth": 0, "max_matches": 64}, True)
            except EvidenceError as exc:
                limitations.append(f"{path}: symbol discovery incomplete: {exc}")
                incomplete = True
                break
            if not isinstance(found, list):
                query["status"] = "unsupported"
                incomplete = True
                continue
            query["pagination"] = {"status": "unknown", "pages": 1, "returned": len(found),
                                   "total": None, "reason": "No completeness metadata; source crosscheck required"}
            for native in found:
                try:
                    if not isinstance(native, dict):
                        raise EvidenceError("Unsupported Serena symbol")
                    normalized, _ = source_path(workspace, native.get("relative_path"))
                    if normalized != path:
                        raise EvidenceError("Symbol returned outside requested file")
                    extent = serena_extent(native, text)
                except EvidenceError as exc:
                    limitations.append(f"{path}: {name}: {exc}")
                    continue
                symbol = {"id": f"s{len(symbols) + len(file_symbols) + 1:06d}",
                          # Explicit source-scoped Serena identity, NOT a HIR name.
                          "qualified_name": f"serena::{path}::{native['name_path']}",
                          "kind": native["kind"], "path": path, "range": extent,
                          "source_sha256": sources[path]["sha256"]}
                file_symbols.append(symbol)
                refs[symbol["id"]] = [oq["id"], query["id"]]
        if incomplete:
            limitations.append(f"{path}: incomplete function discovery; symbols withheld")
        else:
            symbols.extend(file_symbols)
    return symbols, refs


def check_usage_sources(rows, target, workspace, sources, budget, limitations):
    """Byte-span witness checks do not infer coordinate conventions or endpoints."""
    valid = True
    expected = target.rsplit("::", 1)[-1]
    for row in rows:
        try:
            path, absolute = source_path(workspace, row["file"])
            if path not in sources:
                if len(sources) >= budget:
                    raise EvidenceError("Consumer source budget exhausted")
                new = capture_sources(workspace, [path], budget)
                if sum(s["bytes"] for s in sources.values()) + new[path]["bytes"] > MAX_SOURCE_BYTES:
                    raise EvidenceError("Aggregate consumer/source capture exceeds 128 MiB")
                sources.update(new)
                limitations.append(f"{path}: consumer captured after discovery, no pre-query hash")
                valid = False
            data = absolute.read_bytes()
            start, end = row["start"], row["end"]
            if (type(start) is not int or type(end) is not int or not 0 <= start <= end <= len(data)
                    or data[start:end].decode("utf-8") != expected):
                raise EvidenceError("Native byte span does not match target spelling; no offset repair")
        except (EvidenceError, UnicodeError) as exc:
            limitations.append(f"Unresolved who_uses endpoint {row.get('file')}: {exc}")
            valid = False
    return valid


async def produce(args, inventory, plan, workspace, directory, sources, seeds, targets,
                  evidence, limitations):
    async with asyncio.timeout(args.timeout):
        async with connect(args.url, args.timeout) as session:
            server = as_json((await session.initialize()).serverInfo)
            tools = await catalog(session)
            catalogue_path = directory / "hir-catalog.json"
            atomic_json(catalogue_path, {"server": server, "tools": tools})
            capture = ProducerCapture(session, tools, directory, args.max_queries)
            evidence["producer"]["captures"].append(
                {"path": str(catalogue_path), "sha256": sha256(catalogue_path), "locator": "tools"})
            for target in targets:
                first_page = len(capture.queries)
                rows, queries, pagination = await paged_who_uses(
                    capture, workspace, target, args.page_size, args.max_pages)
                witnessed = check_usage_sources(rows, target, workspace, sources,
                                                args.max_sources, limitations)
                for query in queries:
                    query["coverage"]["freshness"] = "unknown" if witnessed else "partial"
                pages = capture.queries[first_page:]
                if not pages:
                    raise EvidenceError("Producer query budget exhausted before target capture")
                if pages:
                    pagination = dict(pagination, pages=len(pages))
                    # One normalized logical query points to a hash-bound page
                    # manifest. Individual raw MCP responses remain untouched.
                    group = dict(pages[0])
                    group_path = directory / (group["id"] + "-sequence.json")
                    atomic_json(group_path, {
                        "target": target, "pagination": pagination,
                        "pages": [{"id": q["id"], "tool": q["tool"], "arguments": q["arguments"],
                                   "artifact": capture_name(workspace, q["artifact"]),
                                   "artifact_sha256": q["artifact_sha256"],
                                   "status": q["status"]} for q in pages]})
                    group.update(artifact=str(group_path), artifact_sha256=sha256(group_path),
                                 pagination=pagination, coverage=coverage())
                    group["coverage"]["freshness"] = "unknown" if witnessed else "partial"
                    group["status"] = ("error" if any(q["status"] == "error" for q in pages)
                                       else "unsupported" if any(q["status"] == "unsupported" for q in pages)
                                       else "ok" if pagination["status"] == "complete" else "partial")
                    evidence["queries"].append(group)
                    evidence["producer"]["captures"].extend(
                        {"path": q["artifact"], "sha256": q["artifact_sha256"], "locator": q["id"]}
                        for q in pages)
                limitations.append(
                    f"{target}: {pagination['status']} indexed-scope pagination; "
                    "HIR endpoint coordinate units/qualified symbol linkage unverified; no normalized relation")
            remaining, last_id = capture.remaining, capture.next_id
        if seeds:
            if args.serena_url is None:
                limitations.append("Symbol seeds supplied without --serena-url; mapping unsupported")
                return [], {}
            async with connect(args.serena_url, args.timeout) as session:
                server = as_json((await session.initialize()).serverInfo)
                tools = await catalog(session)
                catpath = directory / "serena-catalog.json"
                atomic_json(catpath, {"server": server, "tools": tools})
                evidence["producer"]["captures"].append(
                    {"path": str(catpath), "sha256": sha256(catpath), "locator": "tools"})
                # Use distinct artifact names and query IDs across connections.
                capture = ProducerCapture(session, tools, directory, remaining)
                capture.queries = list(evidence["queries"])
                capture.next_id = last_id
                try:
                    return await discover_serena(capture, workspace, sources, seeds, limitations)
                finally:
                    evidence["queries"] = capture.queries
    return [], {}


def producer_main(args):
    """Produce contract artifacts; inputs never become arbitrary tool calls."""
    directory = None
    try:
        workspace = Path(args.workspace).resolve()
        if workspace not in {ROOT.resolve(), CANONICAL_WORKSPACE.resolve()}:
            raise EvidenceError("Producer workspace must be script checkout or explicit canonical root")
        if workspace != CANONICAL_WORKSPACE.resolve():
            raise EvidenceError("Shared services index canonical root only; integrate/copy producer before live replay")
        args.url = validate_url(args.url)
        if args.serena_url:
            args.serena_url = validate_url(args.serena_url)
        if not math.isfinite(args.timeout) or not 1 <= args.timeout <= 300:
            raise EvidenceError("Producer deadline must be within 1..300 seconds")
        for value, low, high in ((args.max_queries, 1, 4096), (args.max_sources, 1, 20000),
                                 (args.max_pages, 1, 128), (args.page_size, 1, 200)):
            if not low <= value <= high:
                raise EvidenceError("Producer budget outside supported bounds")
        inputs = [args.inventory.resolve(), args.plan.resolve()]
        inputs += [p.resolve() for p in (args.binding, args.source_equivalence) if p]
        inputs += [p.resolve() for p in args.context]
        binding = strict_json(args.binding) if args.binding else None
        if binding is not None:
            if not isinstance(binding, dict) or not isinstance(binding.get("evidence_refs"), list):
                raise EvidenceError("Malformed source-binding references")
            for ref in binding["evidence_refs"]:
                if not isinstance(ref, dict) or not isinstance(ref.get("path"), str):
                    raise EvidenceError("Malformed source-binding reference")
                inputs.append((workspace / ref["path"]).resolve())
        inputs = list(dict.fromkeys(inputs))
        for path in inputs:
            capture_name(workspace, path)
        protected = set(inputs) | {Path(__file__).resolve()}
        output = output_path(args.output, protected)
        snapshot_path = output_path(args.snapshot, protected | {output})
        directory = args.artifacts.resolve()
        if directory.exists() or directory in protected or directory in {output, snapshot_path}:
            raise EvidenceError("Artifacts directory must be new and distinct from inputs/outputs")
        if any(path.is_relative_to(directory) for path in (output, snapshot_path)):
            raise EvidenceError("Outputs cannot be descendants of the raw artifacts directory")
        if not all(path.is_relative_to(workspace) for path in (directory, output, snapshot_path)):
            raise EvidenceError("Producer artifacts and outputs must stay within the explicit workspace")
        input_hashes = {path: sha256(path) for path in inputs}
        inventory_hash = input_hashes[args.inventory.resolve()]
        inventory, plan = strict_json(args.inventory), strict_json(args.plan)
        # Validate the whole inventory, preserving strict namespaces/counts.
        list(diagnostic_rows(inventory))
        sources, seeds, targets = producer_plan(plan, workspace, args.max_sources)
        verify_output_source_separation((output, snapshot_path), sources)
        head = git_head(workspace)
        if workspace != ROOT.resolve():
            if head != git_head(ROOT):
                raise EvidenceError("Canonical and script-worktree HEAD differ; integrate producer before live use")
            for path, value in sources.items():
                _, local = source_path(ROOT.resolve(), path)
                if sha256(local) != value["sha256"]:
                    raise EvidenceError("Canonical/script-worktree source mismatch")
        bound = bound_sources(binding, inventory_hash, head, sources, workspace)
        contexts = {}
        diagnostic_keys = {key for key, _, _ in diagnostic_rows(inventory)}
        for context_path in args.context:
            if context_path.suffix.lower() != ".json":
                continue  # Snapshot-bound policy text, never executable predicates.
            data = strict_json(context_path)
            if not isinstance(data, dict) or "diagnostic_key" not in data:
                continue
            key = data["diagnostic_key"]
            if key not in diagnostic_keys or key in contexts:
                raise EvidenceError("Unknown or duplicate diagnostic mapping context")
            contexts[key] = {"data": data, "reference": {
                "path": capture_name(workspace, context_path), "sha256": sha256(context_path),
                "locator": "diagnostic_key=" + key}}
        limitations = [
            "Current source hashes are fresh captures, not manufactured historical hashes.",
            "Source/index/feature/private/macro/trait/test completeness is not established.",
            "Serena source-scoped identities are not HIR crate-qualified names.",
            "Relations remain raw until coordinate conventions and both symbol endpoints are verified.",
        ]
        if args.source_equivalence:
            if sha256(args.source_equivalence) != args.source_equivalence_sha256:
                raise EvidenceError("Source equivalence proof hash mismatch")
            proof = strict_json(args.source_equivalence)
            if not isinstance(proof, dict) or not isinstance(proof.get("proofs"), list):
                raise EvidenceError("Unsupported source equivalence proof")
            limitations.append("Git tree equivalence proof is context only; not index freshness or historical file hashes.")
        configuration = {"toolchain": "unknown", "features": "unknown", "target": "unknown",
                         "lanes": "unknown", "submodules": "unknown", "index": "unknown",
                         "dirty_sources": "unknown; current participant hashes checked before/after",
                         "declared_context": plan.get("configuration", {})}
        if args.source_equivalence:
            configuration["source_equivalence"] = {
                "path": capture_name(workspace, args.source_equivalence),
                "sha256": sha256(args.source_equivalence), "locator": "proofs"}
        evidence = {"schema_version": 1, "kind": "clippy-code-evidence", "workspace": str(workspace),
                    "snapshot_sha256": None, "inventory_sha256": inventory_hash,
                    "producer": {"adapter_version": "2.0", "captures": []},
                    "mappings": [], "relations": [], "queries": [], "limitations": limitations}
        directory.mkdir(parents=False)
        symbols, refs = asyncio.run(produce(args, inventory, plan, workspace, directory,
                                           sources, seeds, targets, evidence, limitations))
        verify_output_source_separation((output, snapshot_path), sources)
        if not args.allow_partial and any(q["status"] != "ok" for q in evidence["queries"]):
            raise EvidenceError("Partial/error/unsupported queries retained as raw artifacts; "
                                "use --allow-partial to export explicitly partial normalized evidence")
        verify_sources(sources)
        if head != git_head(workspace):
            raise EvidenceError("Workspace HEAD changed during collection")
        if any(sha256(path) != digest for path, digest in input_hashes.items()):
            raise EvidenceError("Producer input changed during collection")
        for query in evidence["queries"]:
            query["artifact"] = capture_name(workspace, query["artifact"])
        for ref in evidence["producer"]["captures"]:
            ref["path"] = capture_name(workspace, ref["path"])
        evidence["mappings"] = map_diagnostics(inventory, workspace, sources, symbols, refs, bound, contexts)
        proof_inputs = localization_proofs(evidence["mappings"], evidence["queries"], workspace,
                                          directory, sources, contexts)
        input_records = [{"role": "inventory" if path == args.inventory.resolve() else "source_context",
                          "path": capture_name(workspace, path), "sha256": input_hashes[path]} for path in inputs]
        input_records.extend({"role": "mcp_capture", "path": q["artifact"],
                              "sha256": q["artifact_sha256"]} for q in evidence["queries"])
        input_records.extend({"role": "producer_capture", "path": ref["path"], "sha256": ref["sha256"]}
                             for ref in evidence["producer"]["captures"])
        input_records.extend(proof_inputs)
        snapshot = {"schema_version": 1, "kind": "clippy-source-snapshot",
                    "workspace": str(workspace), "head": head,
                    "captured_at": datetime.now(timezone.utc).isoformat(),
                    "files": [{"path": p, "sha256": v["sha256"]} for p, v in sorted(sources.items())],
                    "inputs": input_records, "configuration": configuration,
                    "source_binding": "partial" if bound else "unknown", "limitations": limitations}
        # Each file is atomic. The pair is hash-bound, not a multi-file transaction:
        # a failed second replace leaves a detectable mismatch, never valid proof.
        verify_sources(sources)
        atomic_json(snapshot_path, snapshot)
        evidence["snapshot_sha256"] = sha256(snapshot_path)
        atomic_json(output, evidence)
        return 0
    except Exception as exc:
        print("mcp-code-evidence producer: " + (str(exc) if isinstance(exc, EvidenceError)
              else type(exc).__name__ + "; operation failed"), file=sys.stderr)
        # Fresh raw artifacts may remain for diagnosis; never replace accepted
        # snapshot/evidence on collection/validation failure.
        return 1


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
    producer = sub.add_parser("produce", help="Capture source-bound planner evidence")
    producer.add_argument("--inventory", required=True, type=Path)
    producer.add_argument("--plan", required=True, type=Path)
    producer.add_argument("--workspace", default=str(ROOT))
    producer.add_argument("--snapshot", required=True)
    producer.add_argument("--output", required=True)
    producer.add_argument("--artifacts", required=True, type=Path)
    producer.add_argument("--binding", type=Path)
    producer.add_argument("--context", type=Path, action="append", default=[],
                          help="Snapshot-bind explicit local review/mapping context or policy text (repeatable)")
    producer.add_argument("--source-equivalence", type=Path)
    producer.add_argument("--source-equivalence-sha256")
    producer.add_argument("--serena-url")
    producer.add_argument("--max-queries", type=int, default=128)
    producer.add_argument("--max-sources", type=int, default=512)
    producer.add_argument("--max-pages", type=int, default=8)
    producer.add_argument("--page-size", type=int, default=50)
    producer.add_argument("--allow-partial", action="store_true",
                          help="Explicitly export partial/error query observations, never exhaustive closure")
    args = parser.parse_args(argv)
    if args.command == "produce":
        return producer_main(args)
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
