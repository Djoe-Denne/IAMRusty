#!/usr/bin/env python3
"""Validate local evidence and report conservative file constraint components.

No discovery, execution, network access, correction or scheduling is performed.
"""

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PureWindowsPath
import re
import sys
import tempfile
import tomllib


_RANGE_SPEC = importlib.util.spec_from_file_location(
    "_constraint_source_ranges", Path(__file__).with_name("clippy_source_ranges.py"))
_RANGES = importlib.util.module_from_spec(_RANGE_SPEC)
sys.modules[_RANGE_SPEC.name] = _RANGES
_RANGE_SPEC.loader.exec_module(_RANGES)


COVERAGE = ("private", "macros", "traits", "features", "tests", "freshness")
STATES = {"verified", "partial", "unknown", "not_applicable"}
POLICY = ".agents/skills/aiforall-sonar-policy/SKILL.md"
LOCAL_RULES = {
    "doc_markdown", "missing_errors_doc", "missing_panics_doc",
    "uninlined_format_args", "needless_raw_string_hashes", "unreadable_literal",
}
LOCAL_PREDICATES = {
    "source_binding_verified", "source_hash_stable_before_after",
    "diagnostic_coordinates_verified", "symbol_endpoints_source_verified",
    "diagnostic_fully_enclosed", "relevant_local_candidates_exhausted",
    "nested_or_competing_symbols_excluded",
}


class Invalid(ValueError):
    """An artifact violates the local contract."""


def require(condition, message):
    if not condition:
        raise Invalid(message)


def fields(value, names, label):
    require(isinstance(value, dict), f"{label}: expected object")
    require(set(names) <= value.keys(), f"{label}: missing fields {sorted(set(names) - value.keys())}")
    return value


def text(value, label):
    require(isinstance(value, str) and bool(value.strip()), f"{label}: expected nonempty string")
    return value


def strings(value, label):
    require(isinstance(value, list) and all(isinstance(x, str) for x in value),
            f"{label}: expected string array")
    return value


def number(value, minimum=0):
    return type(value) is int and value >= minimum


def sha(value):
    require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value),
            "expected lowercase SHA-256")
    return value


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def timestamp(value):
    text(value, "timestamp")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise Invalid("invalid timestamp") from exc
    require(parsed.tzinfo is not None and parsed.utcoffset().total_seconds() == 0,
            "timestamp must be UTC")
    return parsed


def json_document(raw):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, f"duplicate JSON key: {key}")
            result[key] = value
        return result
    try:
        return json.loads(raw, object_pairs_hook=unique,
                          parse_constant=lambda v: (_ for _ in ()).throw(Invalid(f"invalid number: {v}")))
    except json.JSONDecodeError as exc:
        raise Invalid(f"invalid JSON payload: {exc}") from exc


def read_json(path):
    try:
        return json_document(Path(path).read_bytes().decode("utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise Invalid(f"cannot read JSON {path}: {exc}") from exc


def envelope(value, kind, names):
    fields(value, ["schema_version", "kind", *names], kind)
    require(type(value["schema_version"]) is int and value["schema_version"] == 1
            and value["kind"] == kind, f"unsupported {kind} schema")


class Sources:
    """Root-bound paths; aliases are detected, never resolved into graph identity."""

    def __init__(self, workspace):
        path = Path(text(workspace, "workspace"))
        require(path.is_absolute() and path.is_dir(), "workspace must be an existing absolute directory")
        require(path.resolve() == path, "workspace must be canonical")
        self.root = path
        self.identities = {}
        self.raw_paths = {}

    def path(self, raw):
        text(raw, "path")
        require("\x00" not in raw and not PureWindowsPath(raw).is_absolute()
                and not PureWindowsPath(raw).drive and not raw.startswith(("/", "\\")),
                f"outside-root path: {raw}")
        normalized = re.sub(r"/+", "/", raw.replace("\\", "/"))
        require(all(part not in ("", ".", "..") for part in normalized.split("/")),
                f"noncanonical/traversal path: {raw}")
        require(":" not in normalized, f"device/stream path: {raw}")
        path = self.root.joinpath(*normalized.split("/"))
        require(path.resolve().is_relative_to(self.root), f"outside-root symlink: {raw}")
        # Do not silently canonicalize symlinks or case aliases.
        require(path.resolve() == path, f"symlink/case alias: {raw}")
        folded = normalized.casefold()
        require(folded not in self.identities or self.identities[folded] == normalized,
                f"case alias: {raw}")
        if path.exists():
            identity = (path.stat().st_dev, path.stat().st_ino)
            require(identity not in self.identities or self.identities[identity] == normalized,
                    f"filesystem alias: {raw}")
            self.identities[identity] = normalized
        self.identities[folded] = normalized
        self.raw_paths[raw] = normalized
        return normalized, path


def coverage(value):
    fields(value, COVERAGE, "coverage")
    require(all(value[k] in STATES for k in COVERAGE), "invalid coverage value")


def closed(query):
    return (query["status"] == "ok"
            and query["pagination"]["status"] in ("complete", "not_applicable")
            and all(query["coverage"][k] in ("verified", "not_applicable") for k in COVERAGE))


def position(source, line, column, unit):
    # Rust line coordinates do not treat Unicode paragraph separators, vertical
    # tabs or form feeds as extra source lines.
    lines = source.split("\n")
    require(number(line, 1) and line <= len(lines) and number(column), "range outside source")
    content = lines[line - 1].rstrip("\r\n")
    if unit == "unicode_scalar":
        require(column <= len(content), "scalar column outside source")
        scalar = column
    else:
        encoding = "utf-8" if unit == "utf8_byte" else "utf-16-le"
        raw = content.encode(encoding)
        offset = column if unit == "utf8_byte" else column * 2
        require(offset <= len(raw), "column outside source")
        try:
            scalar = len(raw[:offset].decode(encoding))
        except UnicodeError as exc:
            raise Invalid("column splits a Unicode character") from exc
    return line, scalar


def source_range(value, source):
    fields(value, ["start_line", "start_column", "end_line", "end_column", "column_unit"], "range")
    unit = value["column_unit"]
    require(unit in ("utf8_byte", "unicode_scalar", "utf16"), "unknown column unit")
    start = position(source, value["start_line"], value["start_column"], unit)
    end = position(source, value["end_line"], value["end_column"], unit)
    require(start < end, "empty/reversed range")
    return start, end


def replay_simple_function(source, symbol):
    """Replay a deliberately small whole-file grammar, not seed-only discovery.

    Supported files contain only top-level simple functions, whitespace and
    leading /// documentation. Bodies may have ordinary blocks/expressions but
    no local items, closures, attributes, macros, comments or string tokens.
    Anything else fails closed, including code outside the selected seed.
    """
    clean_lines = []
    for line in source.split("\n"):
        if line.lstrip().startswith("///") and not line.lstrip().startswith("////"):
            require(not any(marker in line.lower() for marker in
                            ("@generated", "automatically generated", "do not edit")),
                    "unsupported generated source")
            clean_lines.append(" " * len(line))
        else:
            clean_lines.append(line)
    clean = "\n".join(clean_lines)
    require(not re.search(r'//|/\*|\*/|["\'`#!$|\\]|[^\x00-\x7f]', clean),
            "unsupported comment/string/macro/attribute/closure tokens in local source")
    header = re.compile(
        r"(?:pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+"
        r"(?P<name>[A-Za-z_][A-Za-z_0-9]*)\s*"
        r"\((?P<args>[A-Za-z_0-9\s,:&]*)\)\s*"
        r"(?:->\s*[A-Za-z_][A-Za-z_0-9:]*(?:\s*)?)?\{")
    item_words = re.compile(r"\b(?:fn|impl|trait|mod|struct|enum|union|type|const|static|"
                            r"use|extern|async|move|macro_rules)\b")
    functions = []
    cursor = 0

    def coordinate(offset):
        return clean.count("\n", 0, offset) + 1, offset - clean.rfind("\n", 0, offset) - 1

    while cursor < len(clean):
        if clean[cursor].isspace():
            cursor += 1
            continue
        match = header.match(clean, cursor)
        require(match is not None, "unsupported top-level syntax or omitted competing declaration")
        require(not item_words.search(match["args"]), "unsupported function argument syntax")
        opening = match.end() - 1
        depth, closing = 1, opening + 1
        while closing < len(clean) and depth:
            if clean[closing] == "{":
                depth += 1
            elif clean[closing] == "}":
                depth -= 1
            closing += 1
        require(depth == 0, "unbalanced function body")
        body = clean[opening + 1:closing - 1]
        require(not item_words.search(body), "nested/competing local declaration or unsupported body")
        require(re.fullmatch(r"[A-Za-z_0-9\s{}();,:.+*/%=&<>?\-]*", body) is not None,
                "unsupported function body tokens")
        start = coordinate(cursor)
        end = coordinate(closing)
        # Whole-line MCP extents and exact token extents are both replayable.
        starts = {start}
        original_lines = source.split("\n")
        if not original_lines[start[0] - 1][:start[1]].strip():
            starts.add((start[0], 0))
            line_index = start[0] - 2
            while line_index >= 0 and original_lines[line_index].lstrip().startswith("///"):
                starts.add((line_index + 1, 0))
                line_index -= 1
        ends = {end}
        if (not original_lines[end[0] - 1][end[1]:].strip()
                and end[0] < len(original_lines)):
            ends.add((end[0] + 1, 0))
        functions.append((match["name"], starts, ends))
        cursor = closing
    require(functions, "no supported simple function discovered")
    require(symbol["kind"] in ("function", "Function", "fn", 12),
            "local proof supports top-level functions only")
    selected = source_range(symbol["range"], source)
    matches = [name for name, starts, ends in functions if selected[0] in starts and selected[1] in ends]
    require(len(matches) == 1, "selected symbol endpoints do not match a unique complete function")
    require(matches[0] == symbol["qualified_name"].split("::")[-1],
            "selected symbol name does not match source declaration")
    return selected


def build_plan(inventory_path, evidence_path, snapshot_path, constraints_path=None, observations_path=None):
    paths = [Path(p).resolve() for p in (inventory_path, evidence_path, snapshot_path)]
    constraint_input = Path(constraints_path).resolve() if constraints_path else None
    observation_input = Path(observations_path).resolve() if observations_path else None
    cli_paths = paths + [p for p in (constraint_input, observation_input) if p is not None]
    initial_input_hashes = {path: digest(path) for path in cli_paths}
    inventory, evidence, snapshot = [read_json(p) for p in paths]
    envelope(snapshot, "clippy-source-snapshot",
             ["workspace", "head", "captured_at", "files", "inputs", "configuration",
              "source_binding", "limitations"])
    root = Sources(snapshot["workspace"])
    require(re.fullmatch(r"[0-9a-f]{40}", text(snapshot["head"], "head")), "expected full commit ID")
    timestamp(snapshot["captured_at"])
    require(isinstance(snapshot["configuration"], dict), "configuration must be an object")
    require(snapshot["source_binding"] in ("verified", "partial", "unknown"), "source_binding invalid")
    strings(snapshot["limitations"], "snapshot limitations")
    require(isinstance(snapshot["files"], list) and isinstance(snapshot["inputs"], list),
            "snapshot files/inputs must be arrays")
    hashes, current, validity = {}, {}, {}
    for item in snapshot["files"]:
        fields(item, ["path", "sha256"], "snapshot file")
        name, path = root.path(item["path"])
        require(name not in hashes, "duplicate snapshot file")
        hashes[name] = sha(item["sha256"])
        good = path.is_file() and digest(path) == hashes[name]
        validity[name] = "verified_current" if good else "stale_or_missing"
        if good:
            try:
                current[name] = path.read_bytes().decode("utf-8")
            except UnicodeError as exc:
                raise Invalid(f"source is not UTF-8: {name}") from exc
    bound_inputs = {}
    for item in snapshot["inputs"]:
        fields(item, ["role", "path", "sha256"], "snapshot input")
        text(item["role"], "input role")
        name, path = root.path(item["path"])
        require(name not in bound_inputs, "duplicate snapshot input")
        require(path.is_file() and digest(path) == sha(item["sha256"]), f"stale snapshot input: {name}")
        bound_inputs[name] = item
    require(any(root.root / name == paths[0] and item["role"] == "inventory"
                for name, item in bound_inputs.items()), "inventory not bound by snapshot")
    envelope(evidence, "clippy-code-evidence",
             ["workspace", "snapshot_sha256", "inventory_sha256", "producer",
              "mappings", "relations", "queries", "limitations"])
    require(evidence["workspace"] == snapshot["workspace"], "workspace mismatch")
    require(sha(evidence["snapshot_sha256"]) == digest(paths[2])
            and sha(evidence["inventory_sha256"]) == digest(paths[0]), "evidence binding mismatch")
    require(isinstance(evidence["producer"], dict) and evidence["producer"], "producer metadata required")
    strings(evidence["limitations"], "evidence limitations")
    for key in ("mappings", "queries", "relations"):
        require(isinstance(evidence[key], list), f"{key} must be an array")

    # Retain raw inventory and IDs without reinterpreting strict overlap identity.
    fields(inventory, ["schema_version", "counts", "clippy", "sonar", "matches",
                      "ambiguous_exact_matches", "unmerged_hints", "limitations",
                      "coverage_percent", "source_stats"], "inventory")
    require(type(inventory["schema_version"]) is int and inventory["schema_version"] == 1,
            "unsupported inventory schema")
    require(isinstance(inventory["counts"], dict)
            and isinstance(inventory["coverage_percent"], dict)
            and isinstance(inventory["source_stats"], dict), "inventory metadata must be objects")
    strings(inventory["limitations"], "inventory limitations")
    for name in ("matches", "ambiguous_exact_matches", "unmerged_hints"):
        require(isinstance(inventory[name], list), f"{name} must be an array")
    diagnostics = {}
    ids = {"clippy": set(), "sonar": set()}
    diag_files, path_errors = {}, {}
    for namespace in ids:
        require(isinstance(inventory[namespace], list), "diagnostic array required")
        for record in inventory[namespace]:
            fields(record, ["id", "rule", "message"], "diagnostic")
            identifier = text(record["id"], "diagnostic id")
            text(record["rule"], "diagnostic rule")
            require(identifier not in ids[namespace], "duplicate diagnostic id")
            ids[namespace].add(identifier)
            key = f"{namespace}:{identifier}"
            diagnostics[key] = record
            if namespace == "clippy":
                fields(record, ["primary_spans", "provenance"], "Clippy record")
                require(isinstance(record["primary_spans"], list)
                        and isinstance(record["provenance"], list), "invalid Clippy provenance/spans")
                require(record["provenance"], "Clippy source provenance cannot be empty")
                for source in record["provenance"]:
                    fields(source, ["input", "row", "level", "package_id", "target"], "Clippy provenance")
                    text(source["input"], "Clippy source input")
                    require(number(source["row"], 1) and source["level"] in ("warning", "error"),
                            "invalid Clippy source provenance")
                spans = record["primary_spans"]
            else:
                fields(record, ["file", "range", "issue_keys", "component", "line"], "Sonar record")
                strings(record["issue_keys"], "Sonar issue keys")
                require(record["issue_keys"], "Sonar source issue provenance cannot be empty")
                spans = [record]
            diag_files[key] = set()
            for span in spans:
                fields(span, ["file", "range"], "diagnostic span")
                if span["range"] is not None:
                    require(isinstance(span["range"], list) and len(span["range"]) == 4
                            and all(number(n) for n in span["range"])
                            and span["range"][0] >= 1 and span["range"][2] >= 1
                            and tuple(span["range"][:2]) <= tuple(span["range"][2:]),
                            "invalid inventory range")
                try:
                    name, _ = root.path(span["file"])
                    diag_files[key].add(name)
                except Invalid as exc:
                    path_errors.setdefault(key, []).append(str(exc))
    matched = {k: set() for k in ids}
    for match in inventory["matches"]:
        fields(match, ["clippy_id", "sonar_id", "sonar_issue_keys", "clippy_provenance"], "alias")
        for namespace in ids:
            identifier = match[f"{namespace}_id"]
            require(identifier in ids[namespace] and identifier not in matched[namespace],
                    "unknown/duplicate exact alias")
            matched[namespace].add(identifier)
        c = diagnostics["clippy:" + match["clippy_id"]]
        s = diagnostics["sonar:" + match["sonar_id"]]
        require(len(c["primary_spans"]) == 1 and s["range"] is not None
                and c["rule"] == s["rule"] and c["message"] == s["message"]
                and c["primary_spans"][0] == {"file": s["file"], "range": s["range"]}
                and match["sonar_issue_keys"] == s["issue_keys"]
                and match["clippy_provenance"] == c["provenance"],
                "exact alias does not match strict inventory identity/provenance")
    for hint in inventory["unmerged_hints"]:
        for namespace in ids:
            require(hint.get(f"{namespace}_id") in ids[namespace], "unknown uncertain alias")
    for alias in inventory["ambiguous_exact_matches"]:
        for namespace in ids:
            values = alias.get(f"{namespace}_ids")
            require(isinstance(values, list) and all(v in ids[namespace] for v in values),
                    "unknown ambiguous alias")
    nc, ns, nm = len(ids["clippy"]), len(ids["sonar"]), len(inventory["matches"])
    expected = {"clippy_unique": nc, "sonar_unique_sites": ns, "exact_intersection": nm,
                "matched_clippy": nm, "matched_sonar": nm, "clippy_unmatched": nc - nm,
                "sonar_unmatched": ns - nm, "strict_union": nc + ns - nm,
                "sonar_raw_keys": sum(len(d["issue_keys"]) for d in inventory["sonar"])}
    require(all(type(inventory["counts"].get(k)) is int and inventory["counts"][k] == v
                for k, v in expected.items()), "inventory accounting mismatch")
    issue_keys = [k for d in inventory["sonar"] for k in d["issue_keys"]]
    require(len(issue_keys) == len(set(issue_keys)), "duplicate Sonar issue provenance")
    stats = fields(inventory["source_stats"], ["clippy", "sonar"], "source stats")
    fields(stats["clippy"], ["clippy_diagnostic_rows"], "Clippy source stats")
    fields(stats["sonar"], ["raw_unique_keys", "paging_total"], "Sonar source stats")
    require(stats["clippy"]["clippy_diagnostic_rows"]
            == sum(len(d["provenance"]) for d in inventory["clippy"])
            and stats["sonar"]["raw_unique_keys"] == len(issue_keys)
            and stats["sonar"]["paging_total"] == len(issue_keys), "source provenance accounting mismatch")
    for label, denominator in (("clippy", nc), ("sonar", ns), ("strict_union", nc + ns - nm)):
        expected_percent = round(100 * nm / denominator, 6) if denominator else None
        require(label in inventory["coverage_percent"]
                and inventory["coverage_percent"][label] == expected_percent,
                "overlap percentage accounting mismatch")
    for namespace in ids:
        field = f"{namespace}_unmatched_ids"
        require(field in inventory and isinstance(inventory[field], list)
                and len(inventory[field]) == len(set(inventory[field]))
                and set(inventory[field]) == ids[namespace] - matched[namespace],
                "unmatched diagnostic accounting mismatch")

    queries = {}
    for query in evidence["queries"]:
        fields(query, ["id", "tool", "arguments", "artifact", "artifact_sha256",
                       "status", "pagination", "coverage"], "query")
        identifier = text(query["id"], "query id")
        require(identifier not in queries, "duplicate query id")
        text(query["tool"], "query tool")
        require(isinstance(query["arguments"], dict), "query arguments must be an object")
        require(query["status"] in ("ok", "partial", "error", "unsupported"), "query status invalid")
        page = fields(query["pagination"], ["status", "pages", "returned", "total", "reason"], "pagination")
        require(page["status"] in ("complete", "partial", "unknown", "not_applicable")
                and number(page["pages"]) and number(page["returned"])
                and (page["total"] is None or number(page["total"]))
                and isinstance(page["reason"], str), "invalid pagination")
        require(page["total"] is None or page["returned"] <= page["total"], "pagination count overflow")
        if page["status"] == "complete":
            require(page["total"] == page["returned"] and page["pages"] > 0,
                    "inconsistent complete pagination")
        coverage(query["coverage"])
        name, path = root.path(query["artifact"])
        require(name in bound_inputs and path.is_file()
                and digest(path) == sha(query["artifact_sha256"]), "unbound/stale query artifact")
        queries[identifier] = query

    def references(refs, nonempty=False):
        require(isinstance(refs, list) and (refs or not nonempty), "evidence_refs must be an array")
        for ref in refs:
            if isinstance(ref, str):
                require(ref in queries, f"unknown query reference: {ref}")
            else:
                fields(ref, ["path", "sha256", "locator"], "local evidence reference")
                text(ref["locator"], "evidence locator")
                name, path = root.path(ref["path"])
                require(name in bound_inputs or name in hashes, f"unbound evidence: {name}")
                require(path.is_file() and digest(path) == sha(ref["sha256"]), f"stale evidence: {name}")

    def query_payload(identifier, *, plain=False, artifact=None):
        query = queries[identifier]
        name = artifact or query["artifact"]
        require(root.path(name)[0] in bound_inputs, "raw query is not snapshot-bound")
        raw = read_json(root.root / name)
        require(isinstance(raw, dict) and raw.get("isError") is False, "raw MCP query failed/unsupported")
        values = []

        def decode(value):
            if isinstance(value, str):
                if plain:
                    return value
                return json_document(value)
            require(not plain, "unsupported structured text query")
            return value

        content = raw.get("content")
        require(isinstance(content, list), "raw MCP content must be an array")
        for item in content:
            require(isinstance(item, dict) and item.get("type") == "text"
                    and isinstance(item.get("text"), str), "unsupported raw MCP content")
            values.append(decode(item["text"]))
        structured = raw.get("structuredContent")
        if structured is not None:
            if isinstance(structured, dict) and set(structured) == {"result"}:
                structured = structured["result"]
            values.append(decode(structured))
        require(values, "raw MCP payload missing")
        identities = [json.dumps(v, sort_keys=True, ensure_ascii=False) for v in values]
        require(len(set(identities)) == 1, "conflicting raw MCP payloads")
        return values[0]

    def serena_witnesses(query_ids):
        witnesses = []
        for identifier in query_ids:
            query = queries[identifier]
            require(query["status"] == "ok" and re.search(r"(?:^|[_:.])find_symbol$", query["tool"]),
                    "local proof needs a successful decoded find_symbol query, not an overview")
            payload = query_payload(identifier)
            require(isinstance(payload, list), "unsupported Serena symbol payload")

            def walk(rows, parent=None):
                for row in rows:
                    fields(row, ["name_path", "kind", "relative_path", "body_location"], "Serena symbol")
                    name = text(row["name_path"], "Serena name_path")
                    path = root.path(row["relative_path"])[0]
                    if query["arguments"].get("relative_path") is not None:
                        require(root.path(query["arguments"]["relative_path"])[0] == path,
                                "Serena result/request file mismatch")
                    location = fields(row["body_location"], ["start_line", "end_line"], "Serena body")
                    require(number(location["start_line"]) and number(location["end_line"])
                            and location["start_line"] <= location["end_line"], "invalid native body extent")
                    if parent is not None:
                        require(name.startswith(parent + "/"), "unadvertised Serena child hierarchy")
                    parent_name = parent or (name.rsplit("/", 1)[0] if "/" in name else None)
                    witnesses.append({"query_id": identifier, "name_path": name, "kind": row["kind"],
                                      "relative_path": path, "body_location": location,
                                      "parent_name_path": parent_name})
                    children = row.get("children", [])
                    require(isinstance(children, list), "invalid Serena children")
                    walk(children, name)
            walk(payload)
        return witnesses

    def localization_proof(mapping, symbol, span):
        proofs = []
        for ref in mapping["evidence_refs"]:
            if isinstance(ref, dict) and ref["path"].endswith(".json"):
                document = read_json(root.root / ref["path"])
                if isinstance(document, dict) and document.get("kind") == "clippy-localization-proof":
                    require(ref["locator"] == "/", "localization proof locator must be document root")
                    require(root.path(ref["path"])[0] in bound_inputs,
                            "localization proof must be snapshot input")
                    proofs.append(document)
        require(len(proofs) == 1, "one shared localization proof is required")
        proof = proofs[0]
        envelope(proof, "clippy-localization-proof",
                 ["diagnostic_key", "source_path", "source_sha256", "symbol_id",
                  "inventory_column_unit", "local_discovery"])
        name = root.path(symbol["path"])[0]
        require(proof["diagnostic_key"] == mapping["diagnostic_key"]
                and proof["source_path"] == name and proof["source_sha256"] == hashes[name]
                and proof["symbol_id"] == symbol["id"] and symbol["source_sha256"] == hashes[name]
                and name in current, "localization proof source/symbol binding mismatch")
        discovery = fields(proof["local_discovery"], ["method", "scope_range", "candidate_symbol_ids",
                                                    "query_ids", "checked_predicates", "limitations"],
                           "local discovery")
        require(discovery["method"] in ("source_crosschecked_simple_function_v1",
                                        "source_crosschecked_function_range_v1"),
                "unsupported localization proof method")
        require(discovery["candidate_symbol_ids"] == [symbol["id"]],
                "local candidate identity is ambiguous or incomplete")
        require(strings(discovery["limitations"], "local discovery limitations") == [],
                "local discovery has unresolved limitations")
        predicates = fields(discovery["checked_predicates"], LOCAL_PREDICATES, "local predicates")
        require(set(predicates) == LOCAL_PREDICATES and all(v is True for v in predicates.values()),
                "local predicate summaries are incomplete")
        query_ids = strings(discovery["query_ids"], "local query ids")
        require(query_ids and len(query_ids) == len(set(query_ids)), "local query witnesses required")
        for identifier in query_ids:
            require(identifier in mapping["evidence_refs"] and identifier in queries,
                    "local query witness not referenced by mapping")
            query = queries[identifier]
            require(query["status"] == "ok" and re.search(r"(?:^|[_:.])find_symbol$", query["tool"]),
                    "local proof needs a successful decoded find_symbol query, not an overview")
        unit = proof["inventory_column_unit"]
        require(unit in ("utf8_byte", "unicode_scalar", "utf16"), "unsupported inventory column unit")
        r = span["range"]
        lower = position(current[name], r[0], r[1], unit)
        upper = position(current[name], r[2], r[3], unit)
        if discovery["method"] == "source_crosschecked_function_range_v1":
            result = _RANGES.verify_function_range(current[name], symbol, r, unit, serena_witnesses(query_ids))
            require(result["status"] == "verified", "range-local replay: " + "; ".join(result["reasons"]))
            require(all(result["checked_predicates"].get(k) is True for k in
                        LOCAL_PREDICATES - {"source_binding_verified", "source_hash_stable_before_after"}),
                    "range-local source predicates incomplete")
            require(len(result["candidate_name_paths"]) == 1, "range-local candidate ambiguity")
            require(result["normalized_range"] == symbol["range"], "range-local normalized extent mismatch")
            selected = source_range(result["normalized_range"], current[name])
        else:
            selected = replay_simple_function(current[name], symbol)
        scope = source_range(discovery["scope_range"], current[name])
        require(selected[0] <= lower < upper <= selected[1], "diagnostic not enclosed by selected function")
        require(scope[0] <= lower < upper <= scope[1], "diagnostic outside local discovery scope")

    symbols, mappings, effective, mapping_reasons = {}, {}, {}, {}
    for mapping in evidence["mappings"]:
        fields(mapping, ["diagnostic_key", "status", "symbols", "evidence_refs", "reasons"], "mapping")
        key = mapping["diagnostic_key"]
        require(key in diagnostics and key not in mappings, "duplicate/unknown diagnostic mapping")
        require(mapping["status"] in ("exact", "ambiguous", "unknown", "stale", "outside_root"),
                "invalid mapping status")
        require(isinstance(mapping["symbols"], list), "symbols must be an array")
        strings(mapping["reasons"], "mapping reasons")
        references(mapping["evidence_refs"])
        mappings[key] = mapping
        reasons = []
        mapping_symbol_ids = set()
        for symbol in mapping["symbols"]:
            fields(symbol, ["id", "qualified_name", "kind", "path", "range", "source_sha256"], "symbol")
            identifier = text(symbol["id"], "symbol id")
            require(identifier not in mapping_symbol_ids, "duplicate symbol in mapping")
            mapping_symbol_ids.add(identifier)
            text(symbol["qualified_name"], "qualified name")
            require(isinstance(symbol["kind"], (str, int)), "invalid symbol kind")
            # Shared enclosing symbols may legitimately occur in several mappings.
            require(identifier not in symbols or symbols[identifier] == symbol, "conflicting symbol id")
            symbols[identifier] = symbol
            name, _ = root.path(symbol["path"])
            require(name in hashes, f"symbol file missing from snapshot: {name}")
            sha(symbol["source_sha256"])
            location = fields(symbol["range"], ["start_line", "start_column", "end_line",
                                               "end_column", "column_unit"], "symbol range")
            require(location["column_unit"] in ("utf8_byte", "unicode_scalar", "utf16")
                    and number(location["start_line"], 1) and number(location["end_line"], 1)
                    and number(location["start_column"]) and number(location["end_column"]),
                    "invalid normalized symbol coordinates")
            if name not in current or symbol["source_sha256"] != hashes[name]:
                reasons.append("stale symbol source")
            else:
                source_range(symbol["range"], current[name])
        status = mapping["status"]
        if status == "exact":
            if len(mapping["symbols"]) != 1 or not mapping["evidence_refs"]:
                reasons.append("exact mapping lacks unique source-bound evidence")
            if mapping["symbols"]:
                symbol = mapping["symbols"][0]
                name = root.path(symbol["path"])[0]
                if diag_files[key] != {name}:
                    reasons.append("diagnostic path/primary locations do not identify one symbol file")
                spans = diagnostics[key].get("primary_spans", [diagnostics[key]])
                if len(spans) != 1 or spans[0]["range"] is None:
                    reasons.append("missing or multi-primary diagnostic range")
                elif name in current:
                    try:
                        localization_proof(mapping, symbol, spans[0])
                    except Invalid as exc:
                        reasons.append(str(exc))
            if reasons:
                status = "stale" if "stale symbol source" in reasons else "unknown"
        effective[key] = status
        mapping_reasons[key] = sorted(set(mapping["reasons"] + reasons))
    for key in diagnostics.keys() - mappings.keys():
        effective[key] = "unknown"
        mapping_reasons[key] = ["mapping absent"]

    relations = {}
    for relation in evidence["relations"]:
        fields(relation, ["id", "kind", "from_symbol", "to_symbol", "evidence_refs", "coverage"], "relation")
        identifier = text(relation["id"], "relation id")
        require(identifier not in relations, "duplicate relation id")
        require(relation["kind"] in ("reference", "call", "implementation", "import", "reexport"),
                "invalid relation kind")
        require(relation["from_symbol"] in symbols and relation["to_symbol"] in symbols,
                "unresolved relation endpoint must remain a query limitation")
        coverage(relation["coverage"])
        references(relation["evidence_refs"], True)
        relations[identifier] = relation

    nodes = set().union(*diag_files.values()) if diag_files else set()
    constraints = []
    if constraints_path:
        reviewed = read_json(constraints_path)
        envelope(reviewed, "clippy-coedit-constraints",
                 ["workspace", "snapshot_sha256", "inventory_sha256", "constraints"])
        require(reviewed["workspace"] == snapshot["workspace"]
                and reviewed["snapshot_sha256"] == digest(paths[2])
                and reviewed["inventory_sha256"] == digest(paths[0]), "constraint binding mismatch")
        require(isinstance(reviewed["constraints"], list), "constraints must be an array")
        constraints = reviewed["constraints"]
    parent = {}

    def find(name):
        parent.setdefault(name, name)
        while parent[name] != name:
            parent[name] = parent[parent[name]]
            name = parent[name]
        return name

    def union(files):
        first = min(find(name) for name in files)
        for name in files:
            parent[find(name)] = first

    def policy_ref(value):
        # File plus exact legacy section/rule; no new policy is invented.
        text(value, "policy_ref")
        require("#" in value, "policy_ref requires an exact section/decision locator")
        path, locator = value.split("#", 1)
        name, local = root.path(path)
        require(local.is_file() and bool(locator.strip()), "unsupported policy reference")
        require(name in bound_inputs or name in hashes, "policy reference is not snapshot-bound")
        content = local.read_text(encoding="utf-8")
        require(locator in content, "policy section/decision not found")
        require(name == POLICY or (name.startswith("docs/guides/clippy-decisions/")
                                  and "Accepted" in content and "Proposed" not in content),
                "policy is not an existing accepted precedent")

    constraint_ids, accepted = set(), []
    incomplete_hard = {}
    for constraint in constraints:
        fields(constraint, ["id", "diagnostic_keys", "files", "classification", "rationale",
                            "evidence_refs", "policy_ref", "approval", "scope_complete"], "constraint")
        identifier = text(constraint["id"], "constraint id")
        require(identifier not in constraint_ids, "duplicate constraint id")
        constraint_ids.add(identifier)
        keys = strings(constraint["diagnostic_keys"], "constraint diagnostics")
        require(keys and len(keys) == len(set(keys)) and set(keys) <= diagnostics.keys(),
                "unknown/duplicate constraint diagnostics")
        files = strings(constraint["files"], "constraint files")
        require(files and len(files) == len(set(files)), "duplicate/empty constraint files")
        require(all(root.path(name)[0] == name for name in files), "constraint paths must be canonical")
        require(constraint["classification"] in ("must_coedit", "candidate", "rejected"),
                "invalid constraint classification")
        require(type(constraint["scope_complete"]) is bool, "scope_complete must be boolean")
        text(constraint["rationale"], "constraint rationale")
        text(constraint["policy_ref"], "policy_ref")
        if constraint["approval"] is not None:
            fields(constraint["approval"], ["reviewer", "authority_ref", "reviewed_at"], "approval")
            text(constraint["approval"]["reviewer"], "reviewer")
            timestamp(constraint["approval"]["reviewed_at"])
        references(constraint["evidence_refs"])
        nodes.update(files)
        if constraint["classification"] != "must_coedit":
            continue
        require(len(files) >= 2 and constraint["scope_complete"], "incomplete hard constraint")
        require(snapshot["source_binding"] == "verified", "hard constraint requires verified source binding")
        require(all(name in current for name in files), "hard constraint participant stale/unbound")
        require(all(effective[key] == "exact" and diag_files[key] <= set(files) for key in keys),
                "hard constraint has unresolved diagnostic")
        policy_ref(constraint["policy_ref"])
        approval = fields(constraint["approval"], ["reviewer", "authority_ref", "reviewed_at"], "approval")
        reviewer = text(approval["reviewer"], "reviewer")
        require(timestamp(approval["reviewed_at"]) >= timestamp(snapshot["captured_at"]),
                "approval predates snapshot")
        authority = approval["authority_ref"]
        references([authority], True)
        require(isinstance(authority, dict), "approval requires a source-bound local authorization record")
        authorization = read_json(root.root / authority["path"])
        fields(authorization, ["reviewer", "proposal_author", "authorized", "constraint_ids"], "authorization")
        require(authorization["reviewer"] == reviewer
                and text(authorization["proposal_author"], "proposal_author").strip().casefold()
                != reviewer.strip().casefold()
                and authorization["authorized"] is True
                and identifier in strings(authorization["constraint_ids"], "authorized constraints"),
                "missing delegation or self-approval")
        # A closure attestation is a review artifact, not an MCP-derived hard edge.
        closure = []
        for ref in constraint["evidence_refs"]:
            if isinstance(ref, dict) and ref["path"].endswith(".json"):
                data = read_json(root.root / ref["path"])
                if isinstance(data, dict) and data.get("constraint_id") == identifier:
                    closure.append(data)
        require(any(c.get("scope_complete") is True and c.get("reviewer") == reviewer
                    and c.get("files") == sorted(files)
                    and c.get("rationale") == constraint["rationale"]
                    and c.get("policy_ref") == constraint["policy_ref"]
                    and c.get("source_sha256") == {f: hashes[f] for f in sorted(files)}
                    for c in closure), "hard constraint lacks correction-specific source-bound closure")
        query_ids = [ref for ref in constraint["evidence_refs"] if isinstance(ref, str)]
        require(query_ids, "hard constraint lacks query closure evidence")
        incomplete = sorted(identifier for identifier in query_ids if not closed(queries[identifier]))
        if incomplete:
            # A closed localization query cannot repair an incomplete consumer
            # witness. Keep the proposal and its blockers, but never union it.
            incomplete_hard[constraint["id"]] = incomplete
            continue
        accepted.append(constraint)
        union(files)

    origin_cache, usage_cache, context_hashes = {}, {}, {}
    observation_records, unresolved_observations, candidate_observations = [], [], []
    observation_limitations, observation_name = [], None
    origin_target = "apparatus_contracts::runtime::validate_runtime_uid"
    origin_path = "crates/apparatus-contracts/src/runtime.rs"

    def bound_context(ref, expected):
        references([ref], True)
        name, path = root.path(expected)
        require(path.is_file() and digest(path) == ref["sha256"], "current origin context mismatch: " + name)
        context_hashes[name] = ref["sha256"]
        return path.read_bytes().decode("utf-8")

    def definition_path(raw):
        # Only this literal local-drive wrapper is supported, not device/UNC namespaces.
        if raw.startswith("\\\\?\\"):
            require(re.match(r"^\\\\\?\\[A-Za-z]:\\", raw), "unsupported extended definition path")
            raw = raw[4:]
        require(not raw.startswith(("\\", "/")) and not re.search(r"(^|[\\/])\.\.?([\\/]|$)", raw),
                "unsafe definition path")
        if PureWindowsPath(raw).is_absolute():
            path = Path(raw)
            require(path == path.resolve() and path.is_relative_to(root.root), "definition outside root/alias")
            raw = path.relative_to(root.root).as_posix()
        return root.path(raw)[0]

    def verify_origin(ref):
        require(isinstance(ref, dict) and ref.get("locator") == "/", "origin binding proof required")
        references([ref], True)
        require(root.path(ref["path"])[0] in bound_inputs, "origin proof must be a snapshot input")
        cache_key = (ref["path"], ref["sha256"])
        if cache_key in origin_cache:
            return origin_cache[cache_key]
        proof = read_json(root.root / ref["path"])
        envelope(proof, "clippy-origin-binding",
                 ["target", "source_path", "source_sha256", "declaration_range",
                  "crate_manifest_ref", "module_root_ref", "query_ids", "method"])
        require(set(proof) == {"schema_version", "kind", "target", "source_path", "source_sha256",
                               "declaration_range", "crate_manifest_ref", "module_root_ref", "query_ids", "method"},
                "origin proof contains unsupported fields/backward hash dependencies")
        require(proof["method"] == "direct_crate_module_function_v1" and proof["target"] == origin_target
                and proof["source_path"] == origin_path, "unsupported origin binding method/target")
        require(origin_path in current and proof["source_sha256"] == hashes[origin_path],
                "origin declaration source stale/unbound")
        manifest_text = bound_context(proof["crate_manifest_ref"], "crates/apparatus-contracts/Cargo.toml")
        try:
            manifest = tomllib.loads(manifest_text)
        except tomllib.TOMLDecodeError as exc:
            raise Invalid("unsupported origin Cargo manifest") from exc
        package = manifest.get("package", {})
        lib = manifest.get("lib", {})
        require(isinstance(package, dict) and package.get("name") == "apparatus-contracts"
                and package.get("autolib", True) is True
                and isinstance(lib, dict) and "name" not in lib and "path" not in lib,
                "origin crate/default module identity mismatch")
        module_text = bound_context(proof["module_root_ref"], "crates/apparatus-contracts/src/lib.rs")
        require(not any(marker in (manifest_text + module_text).lower()
                        for marker in ("@generated", "automatically generated", "do not edit")),
                "generated origin crate/module context")
        pending, item, declarations = [], "", 0
        for line in module_text.split("\n"):
            value = line.strip()
            if not value or value.startswith("//"):
                continue
            if value.startswith("#[") and not item:
                require(re.fullmatch(r'#\[cfg\(feature\s*=\s*"[\w-]+"\)\]', value),
                        "unsupported origin module-root attribute")
                pending.append(value)
                continue
            item += " " + value
            if not item.endswith(";"):
                require(item.lstrip().startswith("pub use "), "unsupported origin module-root context")
                continue
            item = item.strip()
            declaration = re.fullmatch(r"pub mod ([A-Za-z_]\w*);", item)
            if declaration:
                if declaration[1] == "runtime":
                    require(not pending, "conditional/path origin module")
                    declarations += 1
            else:
                require(re.fullmatch(
                    r"pub use [A-Za-z_]\w*(?:::[A-Za-z_]\w*)*(?:::\{[\w,\s]+\})?;", item)
                        and not re.search(r"\bruntime\b", item), "unsupported origin module/reexport chain")
            pending, item = [], ""
        require(declarations == 1 and not pending and not item, "literal unconditional runtime module missing")
        ids_ = strings(proof["query_ids"], "origin query IDs")
        require(ids_ and len(ids_) == len(set(ids_)) and all(q in queries for q in ids_),
                "origin query witnesses missing")
        signature_ids = [q for q in ids_ if re.search(r"(?:^|[_:.])function_signature$", queries[q]["tool"])]
        definition_ids = [q for q in ids_ if re.search(r"(?:^|[_:.])find_definition$", queries[q]["tool"])]
        symbol_ids = [q for q in ids_ if re.search(r"(?:^|[_:.])find_symbol$", queries[q]["tool"])]
        require(len(signature_ids) == len(definition_ids) == 1 and symbol_ids
                and len(signature_ids + definition_ids + symbol_ids) == len(ids_), "unsupported origin witnesses")
        for identifier in signature_ids + definition_ids:
            query = queries[identifier]
            require(query["status"] == "ok" and query["arguments"].get("directory") == str(root.root),
                    "origin HIR witness root/status mismatch")
        sig_id, def_id = signature_ids[0], definition_ids[0]
        require(queries[sig_id]["arguments"].get("target") == origin_target,
                "origin signature request target mismatch")
        payload = fields(query_payload(sig_id), ["target", "signature"], "origin signature")
        signature = fields(payload["signature"],
                           ["is_async", "self_param", "params", "return_type", "generics"], "signature")
        require(payload["target"] == origin_target and signature["is_async"] is False
                and signature["self_param"] is None and signature["generics"] == []
                and signature["params"] == [{"name": "value", "ty": "&str", "by_ref": True, "mutability": False}]
                and re.sub(r"\s+", "", text(signature["return_type"], "signature return type")) ==
                "Result<(),RuntimeErrorCodeV1>",
                "origin signature witness mismatch")
        require(queries[def_id]["arguments"].get("symbol_name") == "validate_runtime_uid"
                and queries[def_id]["arguments"].get("exact") is True, "definition request is not exact")
        match = re.fullmatch(
            r"Found 1 definition\(s\) for 'validate_runtime_uid':\r?\n"
            r"([^\r\n]+):(\d+):(\d+) \(validate_runtime_uid, exact=true\)\s*", query_payload(def_id, plain=True))
        require(match is not None, "unsupported or multiple origin definitions")
        require(definition_path(match[1]) == origin_path, "definition file differs from origin")
        line_no, column = int(match[2]), int(match[3])
        lines = current[origin_path].split("\n")
        require(1 <= line_no <= len(lines) and column >= 1, "definition coordinates outside source")
        header = lines[line_no - 1].rstrip("\r")
        require(header[column - 1:].startswith("validate_runtime_uid(")
                and re.sub(r"\s+", "", header) ==
                "pubfnvalidate_runtime_uid(value:&str)->Result<(),RuntimeErrorCodeV1>{",
                "definition token/signature coordinate mismatch (native one-based column)")
        witnesses = serena_witnesses(symbol_ids)
        require(any(w["name_path"] == "validate_runtime_uid" and w["parent_name_path"] is None
                    and w["relative_path"] == origin_path for w in witnesses), "top-level origin witness missing")
        symbol = {"path": origin_path, "qualified_name": "serena::" + origin_path + "::validate_runtime_uid",
                  "kind": "Function", "range": proof["declaration_range"]}
        verified = _RANGES.verify_function_range(
            current[origin_path], symbol, [line_no, 0, line_no, len(header)], "unicode_scalar", witnesses)
        require(verified["status"] == "verified" and verified["normalized_range"] == proof["declaration_range"]
                and verified["candidate_name_paths"] == ["validate_runtime_uid"],
                "origin source range/witness replay failed: " + "; ".join(verified["reasons"]))
        origin_cache[cache_key] = proof
        return proof

    def usage_rows(identifier):
        if identifier in usage_cache:
            return usage_cache[identifier]
        query = queries[identifier]
        require(re.search(r"(?:^|[_:.])who_uses$", query["tool"])
                and query["status"] in ("ok", "partial"), "unsupported usage query")
        require(query["arguments"].get("target") == origin_target
                and query["arguments"].get("directory") == str(root.root), "usage target/root mismatch")
        artifact = read_json(root.root / query["artifact"])
        sequence = isinstance(artifact, dict) and "pages" in artifact
        if sequence:
            fields(artifact, ["target", "pagination", "pages"], "usage sequence")
            require(artifact["target"] == origin_target and artifact["pagination"] == query["pagination"]
                    and isinstance(artifact["pages"], list) and artifact["pages"], "usage sequence mismatch")
            pages = artifact["pages"]
        else:
            pages = [{"id": identifier, "tool": query["tool"], "arguments": query["arguments"],
                      "artifact": query["artifact"], "artifact_sha256": query["artifact_sha256"],
                      "status": query["status"]}]
        rows, seen, expected_offset, total, page_ids = [], set(), 0, None, set()
        for page in pages:
            fields(page, ["id", "tool", "arguments", "artifact", "artifact_sha256", "status"], "usage page")
            text(page["id"], "usage page id")
            require(page["id"] not in page_ids and page["tool"] == query["tool"]
                    and page["status"] == "ok", "usage page identity/status mismatch")
            page_ids.add(page["id"])
            references([{"path": page["artifact"], "sha256": page["artifact_sha256"], "locator": "/"}], True)
            args = fields(page["arguments"], ["target", "directory", "offset", "limit", "summary"],
                          "usage page request")
            require(args.get("target") == origin_target and args.get("directory") == str(root.root)
                    and number(args.get("offset")) and number(args.get("limit"), 1)
                    and args.get("offset") == expected_offset and args.get("summary") is False,
                    "usage page target/root/offset mismatch")
            payload = fields(query_payload(identifier, artifact=page["artifact"]),
                             ["target", "total_match_count", "offset", "limit", "summary",
                              "returned_match_count", "usages"], "HIR usages")
            require(payload["target"] == origin_target and payload["summary"] is False
                    and number(payload["total_match_count"]) and number(payload["returned_match_count"])
                    and number(payload["offset"]) and payload["offset"] == expected_offset
                    and payload["limit"] == args.get("limit")
                    and number(payload["limit"], 1) and isinstance(payload["usages"], list)
                    and len(payload["usages"]) == payload["returned_match_count"]
                    and payload["returned_match_count"] <= payload["limit"], "usage count/page mismatch")
            require(total is None or total == payload["total_match_count"], "usage total changed")
            total = payload["total_match_count"]
            for row in payload["usages"]:
                fields(row, ["file", "start", "end", "category", "consumer_module"], "HIR usage")
                path = root.path(row["file"])[0]
                require(number(row["start"]) and number(row["end"]) and row["start"] < row["end"],
                        "invalid raw usage span")
                text(row["category"], "usage category")
                text(row["consumer_module"], "consumer module")
                if "consumer_function" in row:
                    text(row["consumer_function"], "optional consumer function")
                identity = (path, row["start"], row["end"], row["category"])
                require(identity not in seen, "duplicate/repeated usage page")
                seen.add(identity)
                rows.append((path, row))
            expected_offset += payload["returned_match_count"]
            require(expected_offset <= total and (payload["returned_match_count"] > 0 or len(pages) == 1),
                    "usage pagination overflow/nonprogress")
        page = query["pagination"]
        require(page["pages"] == len(pages) and page["returned"] == expected_offset
                and page["total"] == total, "usage sequence accounting mismatch")
        if page["status"] == "complete":
            require(expected_offset == total, "usage sequence truncated")
        usage_cache[identifier] = rows
        return rows

    if observation_input is not None:
        require(observation_input.is_relative_to(root.root), "observation artifact outside workspace")
        observation_name = root.path(observation_input.relative_to(root.root).as_posix())[0]
        require(observation_name not in bound_inputs, "observations must not be snapshot inputs (hash cycle)")
        observed = read_json(observation_input)
        envelope(observed, "clippy-file-observations",
                 ["workspace", "snapshot_sha256", "evidence_sha256", "inventory_sha256",
                  "observations", "limitations"])
        require(observed["workspace"] == snapshot["workspace"]
                and sha(observed["snapshot_sha256"]) == initial_input_hashes[paths[2]]
                and sha(observed["evidence_sha256"]) == initial_input_hashes[paths[1]]
                and sha(observed["inventory_sha256"]) == initial_input_hashes[paths[0]],
                "observation input binding mismatch")
        observation_limitations = strings(observed["limitations"], "observation limitations")
        require(isinstance(observed["observations"], list), "observations must be an array")
        seen_observations = set()
        usage_assignments = Counter()
        for index, observation in enumerate(observed["observations"]):
            fields(observation, ["id", "kind", "origin", "destination", "query_ids", "origin_binding_ref",
                                 "identity_status", "endpoint_status", "relationship_scope", "reasons"],
                   "file observation")
            identifier = text(observation["id"], "observation id")
            require(identifier not in seen_observations, "duplicate observation id")
            seen_observations.add(identifier)
            require(observation["kind"] in ("reference", "call", "implementation", "import", "reexport")
                    and observation["identity_status"] in ("verified_symbols", "file_candidate", "unresolved")
                    and observation["endpoint_status"] in ("source_verified", "path_only")
                    and observation["relationship_scope"] == "symbol_to_file_candidate",
                    "unsupported observation classification")
            strings(observation["reasons"], "observation reasons")
            ids_ = strings(observation["query_ids"], "observation query ids")
            require(ids_ and len(ids_) == len(set(ids_)) and all(q in queries for q in ids_),
                    "observation query references missing")
            # The initial producer serializes one observation per returned usage,
            # in each query's captured order, including unresolved rows. Consume
            # each witness once, never substitute another matching row/file.
            positions = {q: usage_assignments[q] for q in ids_}
            usage_assignments.update(ids_)
            files = set()
            reasons = list(observation["reasons"]) if (
                observation["identity_status"] == "unresolved" or observation["endpoint_status"] == "path_only") else []
            for endpoint_name in ("origin", "destination"):
                endpoint = fields(observation[endpoint_name], ["path", "source_sha256", "symbol_id", "range"],
                                  "observation endpoint")
                path = root.path(endpoint["path"])[0]
                require(path == endpoint["path"], "observation paths must be canonical")
                files.add(path)
                sha(endpoint["source_sha256"])
                require(endpoint["symbol_id"] is None or isinstance(endpoint["symbol_id"], str),
                        "invalid observation symbol id")
                if path not in current or endpoint["source_sha256"] != hashes.get(path):
                    reasons.append("endpoint source stale/unbound: " + path)
                elif endpoint["range"] is not None:
                    source_range(endpoint["range"], current[path])
            try:
                if observation["origin_binding_ref"] is not None:
                    ref = observation["origin_binding_ref"]
                    require(isinstance(ref, dict) and ref.get("locator") == "/", "invalid origin proof reference")
                    references([ref], True)
                    require(root.path(ref["path"])[0] in bound_inputs, "origin proof is not snapshot-bound")
                require(observation["identity_status"] != "unresolved", "producer identity unresolved")
                require(observation["endpoint_status"] == "source_verified", "endpoint is path-only")
                require(not reasons, "; ".join(reasons))
                proof = verify_origin(observation["origin_binding_ref"])
                origin, destination = observation["origin"], observation["destination"]
                require(origin["path"] == proof["source_path"]
                        and origin["source_sha256"] == proof["source_sha256"], "origin endpoint/proof mismatch")
                if origin["range"] is not None:
                    require(source_range(origin["range"], current[origin["path"]]) ==
                            source_range(proof["declaration_range"], current[origin["path"]]),
                            "origin endpoint range differs from proof")
                require(observation["kind"] == "reference", "initial observation adapter supports who_uses only")
                assigned = []
                for q in ids_:
                    rows = usage_rows(q)
                    require(positions[q] < len(rows), "observation exceeds captured usage sequence")
                    path, row = rows[positions[q]]
                    require(path == destination["path"], "observation/raw usage order or destination mismatch")
                    assigned.append(row)
                raw_source = (root.root / destination["path"]).read_bytes()
                require(all(row["end"] <= len(raw_source) and row["end"] - row["start"] == 20
                            and raw_source[row["start"]:row["end"]] == b"validate_runtime_uid"
                            for row in assigned),
                        "captured usage byte span/spelling mismatch; no offset repair")
                for endpoint in (origin, destination):
                    if endpoint["symbol_id"] is not None:
                        require(endpoint["symbol_id"] in symbols
                                and symbols[endpoint["symbol_id"]]["path"] == endpoint["path"]
                                and symbols[endpoint["symbol_id"]]["source_sha256"] == endpoint["source_sha256"]
                                and (endpoint["range"] is None or source_range(
                                    endpoint["range"], current[endpoint["path"]]) == source_range(
                                    symbols[endpoint["symbol_id"]]["range"], current[endpoint["path"]])),
                                "observation symbol id lacks matching normalized source evidence")
                if observation["identity_status"] == "verified_symbols":
                    require(all(e["symbol_id"] is not None for e in (origin, destination))
                            and any(r["from_symbol"] == origin["symbol_id"]
                                    and r["to_symbol"] == destination["symbol_id"]
                                    and {ref for ref in r["evidence_refs"] if isinstance(ref, str)} & set(ids_)
                                    for r in relations.values()),
                            "verified symbol identity lacks a normalized relation witness")
                candidate_observations.append((observation, index))
                nodes.update(files)
            except Invalid as exc:
                reasons.append(str(exc))
            record = {"id": identifier, "files": sorted(files), "record": observation,
                      "reasons": sorted(set(reasons))}
            observation_records.append(record)
            if reasons:
                unresolved_observations.append(record)
        # Cardinality checks include unresolved observations: dropping a failed
        # row cannot shift later successful observations onto a different witness.
        invalid_sequences = {q for q, rows in usage_cache.items() if usage_assignments[q] != len(rows)}
        if invalid_sequences:
            for record in observation_records:
                if set(record["record"]["query_ids"]) & invalid_sequences:
                    record["reasons"].append("observation/usage sequence cardinality mismatch")
                    if record not in unresolved_observations:
                        unresolved_observations.append(record)
            candidate_observations = [
                pair for pair in candidate_observations
                if not set(pair[0]["query_ids"]) & invalid_sequences]

    blockers = []

    def block(identifier, files, reason, keys=None):
        blockers.append({"id": identifier, "files": sorted(files), "diagnostic_keys": sorted(keys or []),
                         "scope": "bounded" if files else "unbounded", "reason": reason})

    def constraint_scope(constraint):
        affected = set(constraint["files"])
        for key in constraint["diagnostic_keys"]:
            affected.update(diag_files[key])
        return affected

    for key in sorted(diagnostics):
        reasons = list(path_errors.get(key, []))
        if effective[key] != "exact":
            reasons += [f"mapping {effective[key]}", *mapping_reasons[key]]
        if any(name not in current for name in diag_files[key]):
            reasons.append("diagnostic file stale or absent from snapshot")
        if reasons:
            block(f"diagnostic:{key}", diag_files[key], "; ".join(sorted(set(reasons))), [key])
    for relation in relations.values():
        files = {root.path(symbols[relation[e]]["path"])[0] for e in ("from_symbol", "to_symbol")}
        nodes.update(files)
        incomplete = any(relation["coverage"][k] not in ("verified", "not_applicable") for k in COVERAGE)
        if incomplete or (len(files) > 1 and not any(files <= set(c["files"]) for c in accepted)):
            block(f"relation:{relation['id']}", files,
                  f"unqualified {relation['kind']} relation; correction-specific joint-edit proof missing")
    for constraint in constraints:
        if constraint["classification"] == "candidate":
            block(f"constraint:{constraint['id']}", constraint_scope(constraint),
                  constraint["rationale"] + "; reviewed joint-edit scope closure missing",
                  constraint["diagnostic_keys"])
        elif constraint["id"] in incomplete_hard:
            block(f"constraint:{constraint['id']}", constraint_scope(constraint),
                  "hard constraint not qualified before union; incomplete query closure: "
                  + ", ".join(incomplete_hard[constraint["id"]]), constraint["diagnostic_keys"])
    for observation, _ in candidate_observations:
        files = {observation["origin"]["path"], observation["destination"]["path"]}
        if len(files) > 1:
            block(f"observation:{observation['id']}", files,
                  "source-bound file candidate; review the observed origin before partitioning corrections")
    for record in unresolved_observations:
        block(f"observation:{record['id']}", record["files"], "; ".join(record["reasons"]))
    if observation_limitations:
        block("observation-limitations", set(), "; ".join(observation_limitations))
    if snapshot["source_binding"] != "verified":
        block("source-binding", set(), "snapshot source binding is not verified")
    if evidence["limitations"] or snapshot["limitations"]:
        block("artifact-limitations", set(), "; ".join(sorted(set(
            evidence["limitations"] + snapshot["limitations"]))))
    # Relevance comes from normalized references/endpoints/participants, never
    # arbitrary query arguments or prose. A local mapping proof is not a claim
    # about external reference closure.
    for query in queries.values():
        if not closed(query):
            affected = set()
            for relation in relations.values():
                if query["id"] in relation["evidence_refs"]:
                    affected.update(root.path(symbols[relation[e]]["path"])[0]
                                    for e in ("from_symbol", "to_symbol"))
            for constraint in constraints:
                if constraint["classification"] != "rejected" and query["id"] in constraint["evidence_refs"]:
                    affected.update(constraint_scope(constraint))
            for record in observation_records:
                if query["id"] in record["record"]["query_ids"]:
                    affected.update(record["files"])
            block(f"query:{query['id']}", affected,
                   "query pagination and/or language coverage incomplete (see queries)")

    local = {}
    for key, record in sorted(diagnostics.items()):
        rule = record["rule"].removeprefix("clippy::")
        result = {"classification": "needs_review", "policy_ref": None, "checked_predicates": [],
                  "reason": "no checked bounded local-policy context"}
        if rule in LOCAL_RULES:
            result["policy_ref"] = POLICY + "#Campagne 2026-10-08"
        # Only literal rustdoc is automatically recognized. Format/numeric rules
        # need a reviewed source suggestion/context and intentionally stay review.
        if (rule in {"doc_markdown", "missing_errors_doc", "missing_panics_doc"}
                and effective[key] == "exact" and len(diag_files[key]) == 1):
            name = next(iter(diag_files[key]))
            spans = record.get("primary_spans", [record])
            r = spans[0]["range"] if len(spans) == 1 else None
            lines = current.get(name, "").split("\n")
            if (r and r[0] == r[2] and r[0] <= len(lines)
                    and lines[r[0] - 1].lstrip().startswith(("///", "//!"))
                    and not any(token in lines[r[0] - 1] for token in ("include!", "#[doc", "```"))):
                # An explicit source-bound review supplies meaning/API invariants;
                # lexical comment detection alone cannot establish these.
                checks = []
                for ref in mappings[key]["evidence_refs"]:
                    if isinstance(ref, dict) and ref["path"].endswith(".json"):
                        data = read_json(root.root / ref["path"])
                        if isinstance(data, dict) and data.get("diagnostic_key") == key:
                            checks.append(data)
                predicates = ("preserves_api", "preserves_behavior", "preserves_documentation_meaning",
                              "literal_source_only", "not_generated", "no_macro_expansion",
                              "no_cross_file_edit")
                if POLICY in bound_inputs and any(c.get("source_sha256") == hashes[name]
                       and all(c.get(p) is True for p in predicates) for c in checks):
                    result = {"classification": "local_candidate",
                              "policy_ref": POLICY + "#Campagne 2026-10-08",
                              "checked_predicates": ["current_source_hash", "literal_rustdoc_line",
                                                     "unique_enclosing_symbol", *predicates],
                              "reason": "bounded literal documentation precedent; not correction approval"}
        if result["classification"] == "needs_review":
            if rule in LOCAL_RULES:
                result["reason"] = "bounded policy exists, but source suggestion/context predicates are incomplete"
            if not any(key in b["diagnostic_keys"] for b in blockers):
                if not any(key in c["diagnostic_keys"] for c in accepted):
                    block(f"policy:{key}", diag_files[key], result["reason"], [key])
        local[key] = result

    groups = {}
    for name in sorted(nodes):
        groups.setdefault(find(name), []).append(name)
    components = []
    global_blockers = [b["id"] for b in blockers if b["scope"] == "unbounded"]
    for files in sorted(groups.values()):
        keys = sorted(key for key, names in diag_files.items() if set(files) & names)
        hard = sorted(c["id"] for c in accepted if set(files) & set(c["files"]))
        blocked = sorted(b["id"] for b in blockers if set(files) & set(b["files"]))
        components.append({"id": f"component-{len(components) + 1:04d}", "files": files,
                           "classification": ("qualified_hard_component" if hard else
                                              "local_singleton_candidate" if len(files) == 1 and keys
                                              and not blocked and all(
                                                  local[k]["classification"] == "local_candidate" for k in keys)
                                              else "unresolved_component"),
                           "diagnostic_keys": keys, "constraint_ids": hard,
                           "outside_diagnostic_files": sorted(set(files) - set().union(*diag_files.values())),
                           "dispatchability": "blocked" if blocked or (not hard and not keys) else "candidate",
                           "blocker_ids": blocked,
                           "rationale": "minimal relative to qualified hard constraints only",
                           "representative_evidence": {key: mappings.get(key) for key in keys}})
    graph_edges = []
    for constraint in accepted:
        files = sorted(constraint["files"])
        # A deterministic spanning star represents the required set without
        # inventing pair-size limits or a quadratic clique. Full sets stay above.
        for index, other in enumerate(files[1:], 1):
            graph_edges.append({"id": f"hard:{constraint['id']}:{index}",
                                "origin_path": files[0], "destination_path": other,
                                "class": "hard_must_coedit", "relationship_scope": "required_file_set",
                                "evidence_refs": [constraint["id"]], "reasons": [constraint["rationale"]]})
    for relation in relations.values():
        endpoints = [symbols[relation[k]] for k in ("from_symbol", "to_symbol")]
        if all(s["path"] in current and s["source_sha256"] == hashes[s["path"]] for s in endpoints):
            graph_edges.append({"id": "relation:" + relation["id"],
                                "origin_path": endpoints[0]["path"], "destination_path": endpoints[1]["path"],
                                "class": "candidate_dependency", "relationship_scope": "verified_symbol_relation",
                                "evidence_refs": [relation["id"]],
                                "reasons": ["Observed " + relation["kind"] + "; no must-coedit authority"]})
        else:
            unresolved_observations.append({"id": "relation:" + relation["id"],
                                            "files": sorted({s["path"] for s in endpoints}),
                                            "record": relation, "reasons": ["relation source stale/unbound"]})
    for observation, index in candidate_observations:
        graph_edges.append({
            "id": "observation:" + observation["id"],
            "origin_path": observation["origin"]["path"], "destination_path": observation["destination"]["path"],
            "class": "candidate_dependency", "relationship_scope": observation["relationship_scope"],
            "evidence_refs": [{"path": observation_name,
                               "sha256": initial_input_hashes[observation_input],
                               "locator": f"/observations/{index}"}],
            "reasons": ["Source-verified origin-to-file observation; consumer symbol identity not asserted",
                        *observation["reasons"]],
        })
    graph_edges.sort(key=lambda edge: edge["id"])
    graph_nodes = []
    exact_symbols = {s["id"] for key, m in mappings.items() if effective[key] == "exact" for s in m["symbols"]}
    for path in sorted(nodes):
        keys = sorted(key for key, files in diag_files.items() if path in files)
        graph_nodes.append({
            "path": path, "diagnostic_keys": keys,
            "symbol_ids": sorted(s["id"] for s in symbols.values()
                                 if s["path"] == path and s["source_sha256"] == hashes.get(path)),
            "mapping_status_counts": dict(sorted(Counter(effective[key] for key in keys).items())),
            "source_status": validity.get(path, "unbound"),
        })
    proposed_groups = []
    for component in components:
        files = set(component["files"])
        boundary = sorted(edge["id"] for edge in graph_edges if edge["class"] == "candidate_dependency"
                          and ((edge["origin_path"] in files) != (edge["destination_path"] in files)))
        unresolved = set(component["blocker_ids"])
        unresolved.update("observation:" + record["id"] for record in unresolved_observations
                          if set(record["files"]) & files)
        # Global unknowns remain review work, not a claim that the file itself
        # failed a previously established physical/local policy predicate.
        unresolved.update(global_blockers)
        proposed_groups.append({
            "id": component["id"], "files": component["files"],
            "constraint_ids": component["constraint_ids"], "diagnostic_keys": component["diagnostic_keys"],
            "boundary_edge_ids": boundary, "unresolved_ids": sorted(unresolved),
            "readiness": "review_required" if boundary or unresolved or
            component["dispatchability"] == "blocked" else "candidate",
        })
    cross_edges = [edge for edge in graph_edges if edge["class"] == "candidate_dependency"
                   and edge["origin_path"] != edge["destination_path"]]
    graph = {
        "nodes": graph_nodes, "edges": graph_edges, "proposed_groups": proposed_groups,
        "unresolved_observations": sorted(unresolved_observations, key=lambda r: r["id"]),
        "coverage": {
            "diagnostic_files": len(set().union(*diag_files.values())) if diag_files else 0,
            "files": len(graph_nodes),
            "source_verified_files": sum(node["source_status"] == "verified_current" for node in graph_nodes),
            "observed_normalized_symbol_ids": len({s for n in graph_nodes for s in n["symbol_ids"]}),
            "exact_mapping_symbol_ids": len(exact_symbols),
            "source_verified_origin_bindings": len(origin_cache),
            "qualified_hard_constraints": len(accepted),
            "hard_edges": sum(e["class"] == "hard_must_coedit" for e in graph_edges),
            "candidate_edges": sum(e["class"] == "candidate_dependency" for e in graph_edges),
            "candidate_cross_file_observations": len(cross_edges),
            "same_file_candidate_observations": sum(
                edge["class"] == "candidate_dependency" and edge["origin_path"] == edge["destination_path"]
                for edge in graph_edges),
            "unique_cross_file_boundaries": len({tuple(sorted((e["origin_path"], e["destination_path"])))
                                                for e in cross_edges}),
            "tentative_groups": len(proposed_groups),
            "readiness_counts": dict(sorted(Counter(g["readiness"] for g in proposed_groups).items())),
            "edge_absence": "not_observed",
            "edge_orientation": "queried declaration/provider -> observed consumer (impact direction), not depends_on",
            "partition_semantics": "file-boundary coupling for ownership review, not an execution DAG or task order",
            "queries": [{"id": q["id"], "status": q["status"], "pagination": q["pagination"],
                         "coverage": q["coverage"]} for q in sorted(queries.values(), key=lambda q: q["id"])],
            "limitations": ["Observed edges cover only their captured query neighborhood, not the repository.",
                            "File-candidate origin observations do not imply every file diagnostic touches that origin.",
                            *observation_limitations],
        },
        "global_independence": "unproven" if global_blockers else "bounded observations only",
    }
    # Final recheck detects source changes during this offline read, not historic
    # before/after collection equivalence (which remains producer evidence).
    after = {name: (root.root / name).is_file() and digest(root.root / name) == expected_hash
             for name, expected_hash in hashes.items()}
    require(all(after[n] for n in current), "source changed during planning")
    require(all((root.root / name).is_file() and digest(root.root / name) == expected_hash
                for name, expected_hash in context_hashes.items()), "origin context changed during planning")
    require(all(path.is_file() and digest(path) == expected_hash
                for path, expected_hash in initial_input_hashes.items()),
            "input changed during planning")
    require(all((root.root / name).is_file()
                and digest(root.root / name) == item["sha256"]
                for name, item in bound_inputs.items()), "capture changed during planning")
    return {
        "schema_version": 1, "kind": "clippy-constraint-plan",
        "workspace": snapshot["workspace"], "inventory": inventory,
        "snapshot": snapshot, "producer": evidence["producer"],
        "input_hashes": {"inventory": initial_input_hashes[paths[0]],
                         "evidence": initial_input_hashes[paths[1]],
                         "snapshot": initial_input_hashes[paths[2]],
                         "constraints": initial_input_hashes[constraint_input] if constraint_input else None,
                         "observations": initial_input_hashes[observation_input] if observation_input else None},
        "accounting": {"input_diagnostic_records": len(diagnostics),
                       "retained_diagnostic_records": len(diagnostics), "strict_union": nc + ns - nm,
                       "exact_alias_pairs": nm, "uncertain_alias_hints": len(inventory["unmerged_hints"]),
                       "ambiguous_alias_groups": len(inventory["ambiguous_exact_matches"]),
                       "semantic_defect_count": None,
                       "identity": "clippy_unique + sonar_unique_sites - exact_alias_pairs = strict_union"},
        "diagnostics": [{"key": k, "record": diagnostics[k], "files": sorted(diag_files[k]),
                         "mapping_status": effective[k], "mapping_reasons": mapping_reasons[k],
                         "local_policy": local[k]} for k in sorted(diagnostics)],
        "mapping_status_counts": dict(sorted(Counter(effective.values()).items())),
        "input_mapping_status_counts": dict(sorted(Counter(m["status"] for m in mappings.values()).items())),
        "raw_path_to_canonical": dict(sorted(root.raw_paths.items())),
        "source_verification": {"before_plan": validity, "after_plan": after,
                                "origin_context_hashes": dict(sorted(context_hashes.items())),
                                "captured_at": snapshot["captured_at"],
                                "head_verification": "format only; no Git execution",
                                "collection_before_after": "not independently established by offline planner",
                                "age": "evaluate captured_at against review time; no expiry inferred"},
        "queries": sorted(queries.values(), key=lambda q: q["id"]),
        "mappings": sorted(mappings.values(), key=lambda m: m["diagnostic_key"]),
        "evidence_limitations": evidence["limitations"],
        "relations": sorted(relations.values(), key=lambda r: r["id"]),
        "file_observations": observation_records,
        "observation_limitations": observation_limitations,
        "work_graph": graph,
        "constraints": sorted(constraints, key=lambda c: c["id"]),
        "unqualified_hard_constraints": [
            {"id": identifier, "incomplete_query_ids": incomplete_hard[identifier]}
            for identifier in sorted(incomplete_hard)],
        "components": components, "blockers": sorted(blockers, key=lambda b: b["id"]),
        "confirmed_multi_file_components": sum(bool(c["constraint_ids"]) for c in components),
        "global_independence": "unproven" if global_blockers else "bounded observations only",
        "scheduling": {"note": "package × src/tests mutex and integration validation are separate; no dispatch"},
        "limitations": ["Candidate is not a safety certificate or correction authorization.",
                        "A complete query is not proof of complete Rust language/index coverage.",
                        "Strict union is not a semantic defect count.",
                        "Historical source equivalence and authorization truth require independent review."],
    }


def markdown(plan):
    def safe(value):
        return str(value).replace("\r", " ").replace("\n", " ").replace("`", "'")
    a = plan["accounting"]
    counts = plan["inventory"]["counts"]
    lines = ["# Offline Clippy constraint plan", "",
             f"- Clippy records: {counts['clippy_unique']}; Sonar sites: {counts['sonar_unique_sites']}; "
             f"Sonar issue keys: {counts['sonar_raw_keys']}.",
             f"- Diagnostic records retained: {a['retained_diagnostic_records']}/{a['input_diagnostic_records']}.",
             f"- Strict union: {a['strict_union']}; exact alias pairs: {a['exact_alias_pairs']}; "
             f"uncertain hints: {a['uncertain_alias_hints']}. Semantic defect count: unknown.",
             f"- Confirmed multi-file components: {plan['confirmed_multi_file_components']}.",
             f"- Input mapping statuses: {safe(json.dumps(plan['input_mapping_status_counts'], sort_keys=True))}.",
             f"- Validated mapping statuses (including absent mappings): "
             f"{safe(json.dumps(plan['mapping_status_counts'], sort_keys=True))}.",
             f"- Global independence: {plan['global_independence']}.",
             "- Unbounded caveats deny global certification; only bounded relevant blockers attach to components.",
             f"- Snapshot captured at: {safe(plan['source_verification']['captured_at'])}.",
             "- Current source hashes checked before/after planning; collection freshness is a separate claim.",
             "", "## Components", ""]
    overview = plan["work_graph"]["coverage"]
    lines[-2:] = [
        "## Work partition overview", "",
        f"- {overview['files']} source-file nodes retain all {overview['diagnostic_files']} diagnostic files.",
        f"- {overview['tentative_groups']} tentative ownership groups, minimal relative to qualified hard "
        f"constraints; readiness: {safe(json.dumps(overview['readiness_counts'], sort_keys=True))}.",
        f"- {overview['hard_edges']} hard edges; {overview['candidate_edges']} candidate site edges: "
        f"{overview['same_file_candidate_observations']} same-file loops and "
        f"{overview['candidate_cross_file_observations']} cross-file sites over "
        f"{overview['unique_cross_file_boundaries']} unique file boundaries.",
        f"- Unresolved observations: {len(plan['work_graph']['unresolved_observations'])}. "
        "Unknown mappings remain attached to their file nodes.",
        "- Arrows are impact direction: queried declaration/provider → observed consumer, not depends_on. "
        "Same-file loops are not boundaries; this is not an execution DAG or certified-independent lot list.",
    ]
    pairs = sorted({(edge["origin_path"], edge["destination_path"]) for edge in plan["work_graph"]["edges"]
                    if edge["class"] == "candidate_dependency" and edge["origin_path"] != edge["destination_path"]})
    for origin, consumer in pairs:
        lines.append(f"- Boundary review: `{safe(origin)}` → `{safe(consumer)}`; inspect the specific "
                     "origin/correction before concurrent ownership.")
    if not pairs:
        lines.append("- Cross-file candidate boundaries: not_observed, not independent.")
    lines += ["", "## Components", ""]
    for c in plan["components"]:
        lines += [f"### {c['id']} — {c['dispatchability']} ({c['classification']})",
                  "- Files: " + ", ".join(f"`{safe(f)}`" for f in c["files"]),
                  "- Diagnostics: " + (", ".join(c["diagnostic_keys"]) or "none"),
                  "- Required consumers without diagnostics: " + (", ".join(c["outside_diagnostic_files"]) or "none"),
                  "- Hard constraints: " + (", ".join(c["constraint_ids"]) or "none"),
                  "- Blockers: " + (", ".join(c["blocker_ids"]) or "none"),
                  "- Evidence (full capture references remain in JSON):"]
        for key in c["diagnostic_keys"]:
            diagnostic = next(d for d in plan["diagnostics"] if d["key"] == key)
            mapping = c["representative_evidence"][key]
            symbol_names = ", ".join(s["qualified_name"] for s in mapping["symbols"]) if mapping else "none"
            lines += [f"  - `{key}`: mapping={diagnostic['mapping_status']}; "
                      f"symbols={safe(symbol_names)}; {diagnostic['local_policy']['classification']}: "
                      + safe(diagnostic["local_policy"]["reason"])]
        lines.append("")
    lines += ["## Reviewed/proposed constraints", ""]
    unqualified = {c["id"]: c["incomplete_query_ids"] for c in plan["unqualified_hard_constraints"]}
    for constraint in plan["constraints"]:
        lines += [f"- `{safe(constraint['id'])}` ({constraint['classification']}): "
                  f"{safe(constraint['rationale'])}; policy `{safe(constraint['policy_ref'])}`."]
        if constraint["id"] in unqualified:
            lines += ["  - Not qualified; incomplete query closure: "
                      + safe(", ".join(unqualified[constraint["id"]])) + "."]
    lines += ["## Unresolved candidates and blockers", ""]
    for b in plan["blockers"]:
        lines += [f"- `{safe(b['id'])}` ({b['scope']}; {safe(', '.join(b['files'])) or 'global'}): "
                  + safe(b["reason"])]
    lines += ["", "## Query pagination versus language coverage", ""]
    for q in plan["queries"]:
        lines += [f"- `{safe(q['id'])}`: {q['status']}; pagination "
                  f"{q['pagination']['status']} ({q['pagination']['pages']} pages, "
                  f"{q['pagination']['returned']}/{q['pagination']['total']}); "
                  + ", ".join(f"{k}={q['coverage'][k]}" for k in COVERAGE)]
    lines += ["", "## Source binding and aliases", ""]
    for name, status in plan["source_verification"]["before_plan"].items():
        lines.append(f"- `{safe(name)}`: {status}; after-plan match="
                     f"{plan['source_verification']['after_plan'][name]}.")
    for hint in plan["inventory"]["unmerged_hints"]:
        lines.append(f"- Unmerged alias hint `{safe(hint['clippy_id'])}` / "
                     f"`{safe(hint['sonar_id'])}`: {safe(hint.get('type', 'unknown'))}.")
    graph = plan["work_graph"]
    c = graph["coverage"]
    lines += ["", "## Exploratory dependency graph", "",
              f"- Files: {c['files']} ({c['diagnostic_files']} diagnostic files); "
              f"current hash verified: {c['source_verified_files']}.",
              f"- Exact-mapping symbol IDs: {c['exact_mapping_symbol_ids']}; "
              f"verified origin bindings: {c['source_verified_origin_bindings']}.",
              f"- Hard edges: {c['hard_edges']}; candidate site edges: {c['candidate_edges']} "
              f"({c['same_file_candidate_observations']} same-file loops, "
              f"{c['candidate_cross_file_observations']} cross-file sites); "
              f"unique file boundaries: {c['unique_cross_file_boundaries']}.",
              f"- Unresolved observations: {len(graph['unresolved_observations'])}; "
              f"tentative ownership groups: {c['tentative_groups']}; readiness: "
              + safe(json.dumps(c["readiness_counts"], sort_keys=True)) + ".",
              "- These are ownership proposals relative to qualified hard constraints, not independent correction lots.",
              "- Candidate arrows run from queried declaration/provider to observed consumer: impact direction, "
              "not depends_on. Same-file loops are not boundaries. This coupling graph is not an execution DAG.",
              "- Absent edges mean not_observed. No dispatch is authorized.", "",
              "### Boundary review tasks", ""]
    for edge in graph["edges"]:
        if edge["class"] == "candidate_dependency":
            lines.append(f"- `{safe(edge['id'])}`: `{safe(edge['origin_path'])}` → "
                         f"`{safe(edge['destination_path'])}` ({edge['relationship_scope']}). "
                         "Review the specific origin/correction before assigning concurrent ownership; evidence "
                         + safe(json.dumps(edge["evidence_refs"], sort_keys=True)) + ".")
    lines += ["", "### Tentative ownership groups", ""]
    for group in graph["proposed_groups"]:
        lines.append(f"- `{group['id']}` ({group['readiness']}): "
                     + ", ".join(f"`{safe(path)}`" for path in group["files"])
                     + "; boundary tasks: " + (", ".join(group["boundary_edge_ids"]) or "not_observed")
                     + "; unresolved: " + (", ".join(group["unresolved_ids"]) or "none recorded") + ".")
    for record in graph["unresolved_observations"]:
        lines.append(f"- Unresolved `{safe(record['id'])}` in {safe(', '.join(record['files']))}: "
                     + safe("; ".join(record["reasons"])))
    lines += ["", "## Limits", "", *("- " + s for s in plan["limitations"]), ""]
    return "\n".join(lines)


def publish(outputs, inputs, expected_hashes=None):
    targets = [Path(path).resolve() for path, _ in outputs]
    sources = [Path(path).resolve() for path in inputs if path]
    require(len(set(targets)) == len(targets), "output/report collision")
    require(not set(targets) & set(sources), "output/input collision")
    for target in targets:
        require(not target.exists() or target.is_file(), "output must be a file")
        require(not any(target.exists() and source.exists() and os.path.samefile(target, source)
                        for source in sources), "output/input filesystem alias")
    if len(targets) == 2 and all(t.exists() for t in targets):
        require(not os.path.samefile(*targets), "output/report filesystem alias")
    staged = []
    try:
        # Prepare every file before replacing any destination. Each replacement is
        # atomic; two separate output files cannot be a filesystem transaction.
        for target, (_, content) in zip(targets, outputs):
            with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", newline="\n",
                                             dir=target.parent, prefix="." + target.name,
                                             suffix=".tmp", delete=False) as handle:
                staged.append((Path(handle.name), target))
                handle.write(content)
                handle.flush()
                os.fsync(handle.fileno())
        require(all(path.is_file() and digest(path) == expected
                    for path, expected in (expected_hashes or {}).items()),
                "input/source changed before publication")
        for temporary, target in staged:
            os.replace(temporary, target)
    finally:
        for temporary, _ in staged:
            temporary.unlink(missing_ok=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("inventory", "evidence", "snapshot", "output"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--constraints", type=Path)
    parser.add_argument("--observations", type=Path)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args(argv)
    try:
        plan = build_plan(args.inventory, args.evidence, args.snapshot, args.constraints, args.observations)
        outputs = [(args.output, json.dumps(plan, indent=2, sort_keys=True, ensure_ascii=False) + "\n")]
        if args.report:
            outputs.append((args.report, markdown(plan)))
        protected = [args.inventory, args.evidence, args.snapshot, args.constraints, args.observations]
        # Also protect every captured input and source, not only CLI artifacts.
        protected += [Path(plan["workspace"]) / p for p in plan["raw_path_to_canonical"].values()]
        expected = {getattr(args, role).resolve(): value for role, value in plan["input_hashes"].items()
                    if value is not None}
        for item in plan["snapshot"]["inputs"]:
            expected[Path(plan["workspace"]) / item["path"]] = item["sha256"]
        for item in plan["snapshot"]["files"]:
            name = plan["raw_path_to_canonical"][item["path"]]
            if plan["source_verification"]["before_plan"][name] == "verified_current":
                expected[Path(plan["workspace"]) / name] = item["sha256"]
        expected.update({Path(plan["workspace"]) / name: value
                         for name, value in plan["source_verification"]["origin_context_hashes"].items()})
        publish(outputs, protected, expected)
        return 0
    except (Invalid, OSError, UnicodeError, TypeError, KeyError) as exc:
        parser.exit(2, f"clippy-constraint-plan: {exc}\n")


if __name__ == "__main__":
    raise SystemExit(main())
