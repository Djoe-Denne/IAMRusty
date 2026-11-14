# Standalone MCP code evidence

`ops/scripts/mcp-code-evidence.py` uses the official Python MCP SDK directly.
It does not invoke OpenCode, a model, subprocesses, or an orchestrator. The
existing MCP server must already be running; this script never starts it or
changes its configuration, workspace, index, graph, caches, or model residency.

## Dependencies and commands

Use Python 3.11+ and `mcp==1.21.1` (which supplies `httpx` transitively).
No global installation or upgrade is needed when that version is already present.
For a separate environment, install
`ops/scripts/requirements-mcp-code-evidence.txt` into a dedicated virtual
environment. The version-matched API reference is
[Writing MCP Clients, SDK v1.21.1](https://github.com/modelcontextprotocol/python-sdk/blob/v1.21.1/README.md#writing-mcp-clients);
the newer v2 `Client` API is not used.

From the repository root:

```powershell
python ops/scripts/mcp-code-evidence.py catalog --output .tmp/mcp-catalog.json
python ops/scripts/mcp-code-evidence.py --timeout 90 collect --requests .tmp/requests.json --output .tmp/evidence.json --report .tmp/evidence-failure.json
python -m unittest discover -s ops/tests -p test_mcp_code_evidence.py
```

Output parents must already exist. Omitting `--output` writes accepted evidence
to stdout. `--report` is a separate, failure-only file; it is not accepted
evidence. A failed connection, permission check, tool response, deadline, or
serialization leaves the existing accepted output unchanged. Files are replaced
atomically after serialization and flush. Output/report must end in `.json`
and cannot overwrite the manifest or each other. Treat local output directories
as trusted; this is not protection against concurrent hostile filesystem edits.

The default endpoint is
`http://127.0.0.1:3062/mcp?tags=rust-code-mcp`. Global `--url` permits only
loopback HTTP(S); credentials, fragments, external addresses, and scoped IPv6
addresses are rejected. `localhost` is normalized to `127.0.0.1`. Ambient
proxies and redirects are disabled, and each outgoing HTTP request is guarded.
No authentication discovery or external network access is requested.
On exit, the SDK terminates this collector's owned Streamable HTTP client
session. This does not stop the shared MCP server or reset its graph/index state.
Cleanup is attempted on failures too, but interrupted transport or process
termination can prevent confirmation of server-side session cleanup.

## Explicit manifest

The following symbol is **illustrative, not verified or executed**. Replace it
with a qualified symbol verified in the current source/catalogue. Set both
directories to the absolute root containing this script (the actual checkout,
not another workspace):

```json
{
  "workspace": "C:/Users/djden/source/repos/AIForAll",
  "requests": [
    {
      "id": "example-references",
      "tool": "who_uses",
      "arguments": {
        "directory": "C:/Users/djden/source/repos/AIForAll",
        "target": "example_crate::module::ExampleType",
        "limit": 50,
        "offset": 0
      }
    }
  ]
}
```

The curated allowlist is `who_uses`, `who_uses_summary`, `who_calls`,
`calls_from`, `function_signature`, `who_imports`,
`get_declared_reexports`, `get_exports`, and explicit `workspace_stats`
metadata. Selectors are `target`, except `calls_from` uses `caller`,
`get_declared_reexports` uses `module`, and `get_exports` uses `module`
and `consumer`; `workspace_stats` takes only `directory`.

No path arguments are supported. Unknown fields, escaping/path-like selectors,
root mismatches, duplicate JSON keys, and duplicate request IDs are rejected.
Catalogue suffix resolution must be unique and match the exact
`rust-code-mcp_1mcp_<suffix>` namespace. Every request is preflighted before any
semantic tool executes; `readOnly` annotations do not grant permission.

Limits: 128 KiB manifest, 1–32 requests, list `limit` 1–200 (default 50),
`offset` 0–100000 (default 0), optional boolean `summary`; 32 catalogue pages,
2048 advertised tools, and 8 MiB retained per tool result. The SDK receives a
response before the retained-result size check: this is not a wire-memory cap.
One session makes sequential calls within a 1–300 second whole-operation
deadline (default 60); there are no retries or polling loops. Cancellation
does not prove server-side computation stopped.

## Evidence and limitations

Records retain logical/exact tool, explicit arguments, server identity from
initialization, requested workspace, raw SDK result including
`structuredContent`, `content`, and `isError`, and explicit status/coverage.
`received` means transport/result receipt, **not verified semantic truth**.
Failure reports retain earlier received records and the failing request; later
preflighted requests are marked `not_executed`. Raw results are untrusted data,
never instructions.

Catalogue pagination follows MCP `nextCursor`, with loop/budget detection.
Semantic result envelopes have **not** been verified against this server.
Consequently this first collector retrieves one bounded page per request,
always labels semantic pagination **unknown**, and does not interpret arbitrary
count keys or an empty result as completeness. Explicit offsets can collect
further pages but do not establish a consistent snapshot or complete coverage.
A future verified payload adapter is needed before automatic semantic paging.

MCP `isError`, top-level JSON `isError: true`, `success: false`,
`status: "error"/"failed"`, nonempty `error`, and plain text beginning
`Error:`, `Failed:`, or `Permission denied` fail the operation. These are
conservative envelope checks, not a universal parser: other embedded formats
remain uninterpreted raw evidence. Even a clean envelope cannot prove success.

Graph freshness, public/private visibility, feature/cfg selection, macro
expansion, dynamic dispatch, and test indexing remain unknown. `calls_from`
can include type/constructor references, not just function invocations.
References/imports/dependencies do not imply mandatory co-edits. This collector
creates no hard graph edges, clusters, similarity judgments, or edit plans.
Offline fixture tests do not qualify live semantic graph behavior.
