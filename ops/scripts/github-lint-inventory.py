#!/usr/bin/env python3
"""Read-only GitHub issue inventory; issue content is untrusted metadata."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


REPO = re.compile(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})/[A-Za-z0-9_][A-Za-z0-9_.-]{0,99}")
RULE = re.compile(
    r"(?<![\w:])(?:clippy::[a-z][a-z0-9_]*|[a-z][a-z0-9]*:S[0-9]+)(?![\w:])",
    re.IGNORECASE,
)
ANCHOR = re.compile(r"(?<![\w])(?:clippy|sonar|sonarqube|sonarcloud)(?![\w])", re.IGNORECASE)
LABEL_FAMILIES = {
    "clippy": "clippy", "lint:clippy": "clippy",
    "sonar": "sonar", "sonarqube": "sonar", "sonarcloud": "sonar",
    "lint:sonar": "sonar", "lint:sonarqube": "sonar", "lint:sonarcloud": "sonar",
}
AREA_ROOTS = {"services", "workers", "crates", "runtime"}
PATH_ROOTS = AREA_ROOTS | {
    "config", "docs", "ops", "templates", "permission-fixtures", "rustycog",
    ".github", ".cargo",
}
ROOT_FILES = {
    "Cargo.toml", "Cargo.lock", "README.md", "AGENTS.md", "justfile",
    "docker-compose.yml", "Dockerfile.build", "sonar-project.properties",
    "rust-toolchain.toml",
}
PATH_TOKEN = re.compile(r"(?<![\w./\\:@-])(?:\./)?[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*")


class InventoryError(ValueError):
    """Input or fetch failure, never an empty inventory."""


def gh_command(repo):
    if not REPO.fullmatch(repo) or repo.split("/")[1] in {".", ".."}:
        raise InventoryError("invalid repository; expected owner/repo without URL or options")
    return [
        "gh", "api", "--method", "GET", f"repos/{repo}/issues",
        "--paginate", "--slurp", "-f", "state=open", "-f", "per_page=100",
    ]


def fetch(repo):
    command = gh_command(repo)
    try:
        result = subprocess.run(
            command, shell=False, check=False, capture_output=True,
            text=True, encoding="utf-8",
        )
    except (OSError, UnicodeError) as exc:
        raise InventoryError("gh could not run or return UTF-8 JSON") from exc
    if result.returncode:
        # Do not echo server output: it can contain secrets or untrusted content.
        raise InventoryError(f"gh API GET failed (exit {result.returncode}); check gh authentication/access")
    return result.stdout, {"mode": "gh", "repo": repo, "command": command}


def reject_json_constant(_value):
    raise ValueError("nonstandard JSON constant")


def parse_pages(text):
    try:
        data = json.loads(text, parse_constant=reject_json_constant)
    except (ValueError, TypeError) as exc:
        raise InventoryError("input is not valid JSON") from exc
    if not isinstance(data, list):
        raise InventoryError("expected a REST issue array or gh --slurp array of pages")
    if all(isinstance(item, dict) for item in data):
        pages = [data]
    elif data and all(isinstance(item, list) for item in data):
        pages = data
    else:
        raise InventoryError("mixed or unsupported page shape")
    for page in pages:
        for item in page:
            validate_issue(item)
    return pages


def validate_issue(item):
    if not isinstance(item, dict):
        raise InventoryError("each page must contain issue objects")
    if type(item.get("number")) is not int or item["number"] < 1:
        raise InventoryError("issue number must be a positive integer")
    if not isinstance(item.get("title"), str) or not isinstance(item.get("html_url"), str):
        raise InventoryError("issue title and html_url must be strings")
    if item.get("state") not in ("open", "closed"):
        raise InventoryError("issue state must be open or closed")
    if item.get("body") is not None and not isinstance(item["body"], str):
        raise InventoryError("issue body must be a string or null")
    if "pull_request" in item and not isinstance(item["pull_request"], dict):
        raise InventoryError("pull_request must be an object")
    if not isinstance(item.get("labels"), list):
        raise InventoryError("issue labels must be an array")
    for label in item["labels"]:
        if not isinstance(label, str) and not (
            isinstance(label, dict) and isinstance(label.get("name"), str)
        ):
            raise InventoryError("label must be a string or an object with a string name")


def explicit_paths(text):
    paths = set()
    for match in PATH_TOKEN.finditer(text):
        token = match.group().removeprefix("./").rstrip(".")
        parts = token.split("/")
        if any(part in {"", ".", ".."} for part in parts):
            continue
        if token in ROOT_FILES or (
            len(parts) >= 2 and parts[0] in PATH_ROOTS
            and ("." in parts[-1] or parts[-1] in {"justfile", "Dockerfile"})
        ):
            paths.add(token)
    return paths


def classify(observations):
    labels, rules, paths, families, evidence = set(), set(), set(), set(), set()
    for page, item in observations:
        for label in item["labels"]:
            name = label if isinstance(label, str) else label["name"]
            labels.add(name)
            family = LABEL_FAMILIES.get(name.casefold())
            if family:
                families.add(family)
                evidence.add((page, "label", "anchor", name))
            if RULE.fullmatch(name):
                prefix, suffix = name.split(":", 1)
                rule = name.lower() if prefix.lower() == "clippy" else prefix.lower() + ":S" + suffix[1:]
                rules.add(rule)
                families.add("clippy" if rule.startswith("clippy::") else "sonar")
                evidence.add((page, "label", "rule", rule))
        for field in ("title", "body"):
            text = item.get(field) or ""
            for match in RULE.finditer(text):
                raw = match.group()
                prefix, suffix = raw.split(":", 1)
                rule = raw.lower() if prefix.lower() == "clippy" else prefix.lower() + ":S" + suffix[1:]
                rules.add(rule)
                families.add("clippy" if rule.startswith("clippy::") else "sonar")
                evidence.add((page, field, "rule", rule))
            for match in ANCHOR.finditer(text):
                anchor = match.group().lower()
                families.add("clippy" if anchor == "clippy" else "sonar")
                evidence.add((page, field, "anchor", anchor))
            for path in explicit_paths(text):
                paths.add(path)
                evidence.add((page, field, "path", path))
    areas = sorted({
        "/".join(path.split("/")[:2]) for path in paths
        if path.split("/")[0] in AREA_ROOTS and len(path.split("/")) >= 3
    })
    # Duplicate observations may disagree; choose display metadata deterministically.
    title, url = min((item["title"], item["html_url"]) for _, item in observations)
    return {
        "number": observations[0][1]["number"], "title": title, "url": url,
        "labels": sorted(labels), "rules": sorted(rules), "paths": sorted(paths),
        "areas": areas,
        "source_family": "mixed" if len(families) > 1 else next(iter(families), "other"),
        "dispersion": "multiple_areas" if len(areas) > 1 else "single_area" if areas else "unknown",
        "impact": "needs_review",
        "rules_status": "explicit_ids_found" if rules else "unknown",
        "paths_status": "explicit_paths_found" if paths else "unknown",
        "observed_pages": sorted({page for page, _ in observations}),
        "display_metadata_conflict": len({(item["title"], item["html_url"]) for _, item in observations}) > 1,
        "evidence": [
            {"page": page, "field": field, "kind": kind, "value": value}
            for page, field, kind, value in sorted(evidence)
        ],
    }


def inventory(text, source):
    pages = parse_pages(text)
    grouped = {}
    counts = {"retrieved_records": 0, "excluded_pull_requests": 0, "excluded_closed_issues": 0,
              "open_issue_records": 0, "duplicate_open_issue_records": 0}
    for page_number, page in enumerate(pages, 1):
        for item in page:
            counts["retrieved_records"] += 1
            if "pull_request" in item:
                counts["excluded_pull_requests"] += 1
            elif item["state"] != "open":
                counts["excluded_closed_issues"] += 1
            else:
                counts["open_issue_records"] += 1
                grouped.setdefault(item["number"], []).append((page_number, item))
    issues = [classify(grouped[number]) for number in sorted(grouped)]
    counts["unique_open_issues"] = len(issues)
    counts["duplicate_open_issue_records"] = counts["open_issue_records"] - len(issues)
    counts["by_source_family"] = {
        family: sum(issue["source_family"] == family for issue in issues)
        for family in ("clippy", "sonar", "mixed", "other")
    }
    return {
        "schema_version": 1,
        "source": {**source, "sha256": hashlib.sha256(text.encode("utf-8")).hexdigest(),
                   "page_count": len(pages)},
        "completeness": {
            "all_records_in_retrieved_pages_processed": True,
            "pagination": "gh_paginate_completed" if source["mode"] == "gh" else "not_verifiable_offline",
            "repository_snapshot": "not_proven",
        },
        "counts": counts, "issues": issues,
    }


def render(report):
    return json.dumps(report, ensure_ascii=True, indent=2, sort_keys=True) + "\n"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--repo", help="GitHub owner/repo; GET all open issue pages through gh")
    source.add_argument("--input", type=Path, help="offline REST array or gh --slurp JSON")
    parser.add_argument("--output", type=Path, help="write JSON here instead of stdout")
    args = parser.parse_args(argv)
    try:
        if args.repo is not None:
            text, metadata = fetch(args.repo)
        else:
            text = args.input.read_text(encoding="utf-8-sig")
            metadata = {"mode": "offline", "input": str(args.input)}
        result = render(inventory(text, metadata))
        if args.output:
            args.output.write_text(result, encoding="utf-8")
        else:
            sys.stdout.write(result)
    except (InventoryError, OSError, UnicodeError) as exc:
        # Filesystem exceptions can contain sensitive paths; only report their type.
        message = str(exc) if isinstance(exc, InventoryError) else type(exc).__name__
        print(f"github-lint-inventory: {message}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
