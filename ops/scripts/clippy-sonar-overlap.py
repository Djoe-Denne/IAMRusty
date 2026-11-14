#!/usr/bin/env python3
"""Conservative, offline overlap of complete Cargo Clippy and Sonar captures."""

import argparse
from collections import Counter, defaultdict
import json
import os
from pathlib import Path
import sys
import tempfile


class InputError(ValueError):
    """Invalid or incomplete inventory input."""


def require(condition, message):
    if not condition:
        raise InputError(message)


def integer(value, minimum=0):
    return type(value) is int and value >= minimum


def text(value, label, nonempty=True):
    require(isinstance(value, str) and (bool(value) or not nonempty),
            f"{label} must be a {'nonempty ' if nonempty else ''}string")
    return value


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"))


def normalized_path(value):
    # Do not resolve dot segments: source captures may traverse symlinks or
    # compiler include paths. Only normalize the platform separator.
    return text(value, "file path").replace("\\", "/")


def normalized_rule(value):
    value = text(value, "rule")
    if value.startswith("external_clippy:"):
        return "clippy::" + value[len("external_clippy:"):]
    return value


def source_range(value, cargo=False):
    require(isinstance(value, dict), "range/span must be an object")
    names = (("line_start", "column_start", "line_end", "column_end") if cargo
             else ("startLine", "startOffset", "endLine", "endOffset"))
    coords = tuple(value.get(k) for k in names)
    for index, coord in enumerate(coords):
        require(integer(coord, 1 if cargo or index % 2 == 0 else 0),
                f"invalid range coordinate {names[index]}")
    start_line, start_col, end_line, end_col = coords
    require((end_line, end_col) >= (start_line, start_col), "range ends before start")
    if cargo:
        start_col -= 1
        end_col -= 1
    return [start_line, start_col, end_line, end_col]


def load_json(raw, label):
    try:
        return json.loads(raw, parse_constant=lambda s: (_ for _ in ()).throw(
            InputError(f"{label}: non-JSON constant {s}")))
    except json.JSONDecodeError as exc:
        raise InputError(f"{label}: malformed JSON at line {exc.lineno}, column {exc.colno}") from exc


def read_clippy(paths):
    groups = {}
    stats = Counter({key: 0 for key in (
        "rows", "successful_files", "excluded_non_compiler_rows",
        "excluded_compiler_messages", "clippy_diagnostic_rows")})
    for path in sorted(set(map(str, paths))):
        finished = False
        for row_number, raw in enumerate(Path(path).read_text(encoding="utf-8-sig").splitlines(), 1):
            if not raw.strip():
                continue
            label = f"{path}:{row_number}"
            row = load_json(raw, label)
            require(isinstance(row, dict), f"{label}: Cargo row must be an object")
            reason = text(row.get("reason"), f"{label}: reason")
            require(not finished, f"{label}: rows after build-finished")
            stats["rows"] += 1
            if reason == "build-finished":
                require(row.get("success") is True, f"{label}: build-finished must be successful")
                finished = True
                stats["successful_files"] += 1
                continue
            if reason != "compiler-message":
                stats["excluded_non_compiler_rows"] += 1
                continue
            diagnostic = row.get("message")
            require(isinstance(diagnostic, dict), f"{label}: message must be an object")
            level = text(diagnostic.get("level"), f"{label}: level")
            code = diagnostic.get("code")
            require(code is None or isinstance(code, dict), f"{label}: code must be object/null")
            rule = code.get("code") if code else None
            require(rule is None or isinstance(rule, str), f"{label}: diagnostic code must be string/null")
            if not rule or not rule.startswith("clippy::") or level not in ("warning", "error"):
                stats["excluded_compiler_messages"] += 1
                continue
            message = text(diagnostic.get("message"), f"{label}: diagnostic message", False)
            spans = diagnostic.get("spans")
            require(isinstance(spans, list), f"{label}: spans must be an array")
            primary = []
            for span in spans:
                require(isinstance(span, dict) and type(span.get("is_primary")) is bool,
                        f"{label}: span needs boolean is_primary")
                if span["is_primary"]:
                    primary.append({"file": normalized_path(span.get("file_name")),
                                    "range": source_range(span, cargo=True)})
            primary.sort(key=canonical)
            record = {"rule": rule, "message": message, "primary_spans": primary}
            key = canonical(record)
            if key not in groups:
                groups[key] = dict(record, provenance=[])
            groups[key]["provenance"].append({
                "input": path, "row": row_number, "level": level,
                "package_id": row.get("package_id"), "target": row.get("target")})
            stats["clippy_diagnostic_rows"] += 1
        require(finished, f"{path}: missing successful build-finished")
    records = [groups[k] for k in sorted(groups)]
    for index, record in enumerate(records, 1):
        record["id"] = f"C{index:06d}"
        record["provenance"].sort(key=canonical)
        record["unresolved_reason"] = (
            None if len(record["primary_spans"]) == 1 else
            "no_primary_span" if not record["primary_spans"] else "multi_primary_spans")
    return records, dict(sorted(stats.items()))


def read_sonar(path):
    data = load_json(Path(path).read_text(encoding="utf-8-sig"), str(path))
    pages = [data] if isinstance(data, dict) else data
    require(isinstance(pages, list) and bool(pages), "Sonar must contain one or more response objects")
    seen_pages, issues = set(), {}
    total = size = None
    raw_rows = 0
    for page in pages:
        require(isinstance(page, dict), "Sonar response must be an object")
        paging = page.get("paging")
        require(isinstance(paging, dict), "Sonar response needs paging object")
        p, ps, count = (paging.get(k) for k in ("pageIndex", "pageSize", "total"))
        require(integer(p, 1) and integer(ps, 1) and integer(count), "invalid Sonar paging integers")
        if total is None:
            total, size = count, ps
        require((total, size) == (count, ps), "inconsistent Sonar paging totals/page sizes")
        for alias, expected in (("p", p), ("ps", ps), ("total", count)):
            require(alias not in page or (integer(page[alias]) and page[alias] == expected),
                    f"Sonar {alias} contradicts paging")
        require(p not in seen_pages, "duplicate Sonar page index")
        seen_pages.add(p)
        rows = page.get("issues")
        require(isinstance(rows, list), "Sonar issues must be an array")
        require(len(rows) <= ps, "Sonar page exceeds pageSize")
        expected_rows = max(0, min(ps, count - (p - 1) * ps))
        require(len(rows) >= expected_rows, "incomplete Sonar page contents")
        for issue in rows:
            require(isinstance(issue, dict), "Sonar issue must be an object")
            key = text(issue.get("key"), "Sonar issue key")
            if key in issues:
                require(issues[key] == issue, f"conflicting Sonar key metadata: {key}")
            issues[key] = issue
            raw_rows += 1
    expected_pages = max(1, (total + size - 1) // size)
    require(seen_pages == set(range(1, expected_pages + 1)), "incomplete Sonar paging sequence")
    require(len(issues) == total,
            f"incomplete Sonar issues: {len(issues)} unique keys != paging.total {total}")
    groups = {}
    for key, issue in sorted(issues.items()):
        rule = normalized_rule(issue.get("rule"))
        component = text(issue.get("component"), "Sonar component")
        message = text(issue.get("message"), "Sonar message", False)
        # Sonar component is project:path; remove only a declared project prefix.
        project = issue.get("project")
        require(project is None or isinstance(project, str), "Sonar project must be string/null")
        file = component[len(project) + 1:] if project and component.startswith(project + ":") else component
        file = normalized_path(file)
        range_value = issue.get("textRange")
        location = source_range(range_value) if range_value is not None else None
        line = issue.get("line")
        require(line is None or integer(line, 1), "Sonar line must be positive integer/null")
        require(location is None or line is None or location[0] == line,
                "Sonar line contradicts textRange")
        record = {"rule": rule, "component": component, "file": file,
                  "message": message, "range": location,
                  "line": location[0] if location else line}
        identity = canonical(record)
        groups.setdefault(identity, dict(record, issue_keys=[]))["issue_keys"].append(key)
    records = [groups[k] for k in sorted(groups)]
    for index, record in enumerate(records, 1):
        record["id"] = f"S{index:06d}"
        record["unresolved_reason"] = None if record["range"] else "missing_text_range"
    return records, {"pages": len(pages), "paging_total": total,
                     "raw_issue_rows": raw_rows, "raw_unique_keys": len(issues)}


def exact_key(rule, message, file, location):
    return canonical([rule, message, file, location])


def inventory(clippy_paths, sonar_path):
    clippy, clippy_stats = read_clippy(clippy_paths)
    sonar, sonar_stats = read_sonar(sonar_path)
    c_index, s_index = defaultdict(list), defaultdict(list)
    for record in clippy:
        if len(record["primary_spans"]) == 1:
            span = record["primary_spans"][0]
            c_index[exact_key(record["rule"], record["message"], span["file"], span["range"])].append(record)
    for record in sonar:
        if record["range"] is not None:
            s_index[exact_key(record["rule"], record["message"], record["file"], record["range"])].append(record)
    matches, ambiguous = [], []
    matched_c, matched_s = set(), set()
    for key in sorted(c_index.keys() & s_index.keys()):
        cs, ss = c_index[key], s_index[key]
        if len(cs) == len(ss) == 1:
            c, s = cs[0], ss[0]
            matches.append({"clippy_id": c["id"], "sonar_id": s["id"],
                            "sonar_issue_keys": s["issue_keys"],
                            "clippy_provenance": c["provenance"]})
            matched_c.add(c["id"])
            matched_s.add(s["id"])
        else:
            ambiguous.append({"clippy_ids": [c["id"] for c in cs],
                              "sonar_ids": [s["id"] for s in ss]})
            for record in cs + ss:
                record["unresolved_reason"] = "ambiguous_exact_match"
    hints = []
    for c in clippy:
        if c["id"] in matched_c:
            continue
        for s in sonar:
            if s["id"] in matched_s or c["rule"] != s["rule"]:
                continue
            types = set()
            for span in c["primary_spans"]:
                if span["file"] != s["file"]:
                    continue
                if (span["range"][0] == s["line"] and c["message"] == s["message"]
                        and (span["range"] != s["range"] or len(c["primary_spans"]) != 1)):
                    types.add("same_start_line_message_different_range_or_multi_primary")
                if (span["range"] == s["range"] and c["message"] != s["message"]):
                    types.add("same_full_range_different_message")
            for kind in sorted(types):
                hints.append({"type": kind, "clippy_id": c["id"], "sonar_id": s["id"]})
    intersection = len(matches)
    union = len(clippy) + len(sonar) - intersection
    percent = lambda numerator, denominator: round(100 * numerator / denominator, 6) if denominator else None
    return {
        "schema_version": 1,
        "counts": {"clippy_unique": len(clippy), "sonar_unique_sites": len(sonar),
                   "sonar_raw_keys": sonar_stats["raw_unique_keys"],
                   "exact_intersection": intersection, "matched_clippy": len(matched_c),
                   "matched_sonar": len(matched_s), "clippy_unmatched": len(clippy) - intersection,
                   "sonar_unmatched": len(sonar) - intersection, "strict_union": union},
        "coverage_percent": {"clippy": percent(intersection, len(clippy)),
                             "sonar": percent(intersection, len(sonar)),
                             "strict_union": percent(intersection, union)},
        "source_stats": {"clippy": clippy_stats, "sonar": sonar_stats},
        "matches": matches, "ambiguous_exact_matches": ambiguous, "unmerged_hints": hints,
        "clippy_unmatched_ids": [c["id"] for c in clippy if c["id"] not in matched_c],
        "sonar_unmatched_ids": [s["id"] for s in sonar if s["id"] not in matched_s],
        "clippy": clippy, "sonar": sonar,
        "limitations": [
            "Unmatched does not mean semantically source-exclusive.",
            "Strict union retains differences in message, range, OS, coverage and source revision.",
            "Only external_clippy: is mapped to clippy::; native Sonar rules are not synonyms.",
            "Only path separators are normalized; case/dot segments are preserved and roots not guessed.",
            "No source lookup, line offsets, fuzzy matching, network or code execution."]}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--clippy", action="append", required=True, help="Cargo JSONL; repeat for every lane")
    parser.add_argument("--sonar", required=True, help="Saved Sonar response or array of pages")
    parser.add_argument("--output", help="JSON output (otherwise stdout); replaced only after validation")
    args = parser.parse_args(argv)
    temporary = None
    try:
        if args.output:
            output = Path(args.output).resolve()
            require(output not in {Path(p).resolve() for p in args.clippy + [args.sonar]},
                    "output must not overwrite an input")
        result = inventory(args.clippy, args.sonar)
        rendered = json.dumps(result, sort_keys=True, indent=2, ensure_ascii=False) + "\n"
        if args.output:
            with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=output.parent,
                                             prefix=output.name + ".", suffix=".tmp", delete=False) as stream:
                temporary = stream.name
                stream.write(rendered)
            os.replace(temporary, output)
            temporary = None
        else:
            sys.stdout.write(rendered)
        return 0
    except (InputError, OSError, UnicodeError) as exc:
        print(f"clippy-sonar-overlap: {exc}", file=sys.stderr)
        return 2
    finally:
        if temporary is not None:
            os.unlink(temporary)


if __name__ == "__main__":
    sys.exit(main())
