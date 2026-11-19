"""Advisory service selection and same-run PR quality evidence."""

import argparse
from collections import defaultdict
import json
import os
from pathlib import Path
import re
import subprocess

from pipeline import (
    INVENTORY, ROOT, SHA, changed_paths, evidence_identity, execution_scope, owner,
    parse_execution_scope,
)


def git(root, *args):
    return subprocess.run(
        ["git", "--literal-pathspecs", *args], cwd=root, check=True, capture_output=True
    ).stdout


def pr_paths(base, head, tested, root=ROOT):
    """Bind the selection and line ranges to the actual GitHub PR merge tree."""
    if not all(SHA.fullmatch(sha) for sha in (base, head, tested)):
        raise ValueError("PR scope requires exact base, head and tested SHAs")
    checkout = git(root, "rev-parse", "HEAD").decode("ascii").strip()
    parents = git(root, "rev-list", "--parents", "-n", "1", tested).decode("ascii").split()
    if checkout != tested or parents != [tested, base, head]:
        raise ValueError("PR checkout does not match its declared merge parents")
    return changed_paths(base, tested, root)


DOC_PATH = re.compile(r"(?:README\.md|docs/(?:adr|contracts|guides|missions)/[^/]+\.md)")


def docs_only(paths, base, tested, root=ROOT):
    """Allow only ordinary Markdown blobs in explicitly known documentation paths."""
    if not paths or not all(DOC_PATH.fullmatch(path) for path in paths):
        return False
    for path in paths:
        entries = [git(root, "ls-tree", "-z", revision, "--", path) for revision in (base, tested)]
        if not any(entries) or any(entry and not entry.startswith(b"100644 blob ") for entry in entries):
            return False
    return True


def dependency_closure(seeds, edges):
    reached = set(seeds)
    pending = list(seeds)
    while pending:
        for consumer in edges.get(pending.pop(), ()):
            if consumer not in reached:
                reached.add(consumer)
                pending.append(consumer)
    return reached


def shadow_plan(paths, data, root=ROOT):
    """Do not use this plan to skip jobs until the runtime edges are qualified."""
    groups_by_package = {
        pkg: group for group, packages in INVENTORY["packages"].items() for pkg in packages
    }
    directories = {
        Path(pkg["manifest_path"]).resolve().parent: pkg["name"] for pkg in data["packages"]
    }
    normal, test = defaultdict(set), defaultdict(set)
    for pkg in data["packages"]:
        for dep in pkg.get("dependencies", ()):
            upstream = directories.get(Path(dep["path"]).resolve()) if dep.get("path") else None
            if upstream is not None:
                test[upstream].add(pkg["name"])
                if dep.get("kind") != "dev":
                    normal[upstream].add(pkg["name"])

    builds, suites = defaultdict(set), defaultdict(set)
    suite_groups = set(INVENTORY["coverage"]) | {"ext-authz"}

    def full(reason):
        return {
            "full": True, "reason": reason,
            "build": {lane: [reason] for lane in sorted(INVENTORY["builds"])},
            "integration": {lane: [reason] for lane in sorted(suite_groups)},
        }

    for path in paths or (None,):
        if path and (path == "README.md" or
                     (path.startswith("docs/") and path.endswith(".md"))):
            continue
        pkg = None
        if path and path.endswith(".rs") and owner(path) not in (None, "shared", "readiness"):
            # Prefer the most specific manifest (a service contains nested crates).
            file = root / path
            package_dir = next((d for d in file.parents if d in directories), None)
            if package_dir and file.parent != package_dir:
                pkg = directories[package_dir]
                kind = file.relative_to(package_dir).parts[0]
                group = groups_by_package[pkg]
                if group not in INVENTORY["builds"] or kind not in ("src", "tests"):
                    pkg = None
        if pkg is None:
            return full(f"Uncertain or global input: {path or '(no proven diff)'}")
        if kind == "tests":
            if group in suite_groups:
                suites[group].add(f"Test source in {pkg}")
            else:
                return full(f"No declared integration lane for {group}")
            continue

        production = dependency_closure({pkg}, normal)
        affected_builds = {
            lane for lane, roots in INVENTORY["builds"].items() if production.intersection(roots)
        }
        for lane in affected_builds:
            builds[lane].add(f"Build dependency on {pkg}")
        test_consumers = {groups_by_package[p] for p in dependency_closure({pkg}, test)}
        runtime = dependency_closure(
            test_consumers | affected_builds, INVENTORY["runtime_consumers"]
        )
        for lane in (test_consumers | affected_builds) & suite_groups:
            suites[lane].add(f"Build or test dependency on {pkg}")
        for lane in (runtime - test_consumers - affected_builds) & suite_groups:
            suites[lane].add(f"Conservative runtime dependency on {group}")

    return {
        "full": set(builds) == set(INVENTORY["builds"]) and set(suites) == suite_groups,
        "build": {group: sorted(reasons) for group, reasons in sorted(builds.items())},
        "integration": {group: sorted(reasons) for group, reasons in sorted(suites.items())},
    }


def service_scope(paths, base, tested, data, root=ROOT):
    """Prune known Rust paths only; mixed or unproven inputs run the full baseline."""
    if not paths or any(not path.endswith(".rs") for path in paths):
        return execution_scope("full")
    for path in paths:
        entries = [git(root, "ls-tree", "-z", revision, "--", path) for revision in (base, tested)]
        if not any(entries) or any(entry and not entry.startswith(b"100644 blob ") for entry in entries):
            return execution_scope("full")
    plan = shadow_plan(paths, data, root)
    if plan["full"] or not set(plan["integration"]) & INVENTORY["coverage"].keys():
        return execution_scope("full")
    return execution_scope("services", builds=plan["build"], integration=plan["integration"])


def write_outputs(**values):
    if filename := os.environ.get("GITHUB_OUTPUT"):
        with open(filename, "a", encoding="utf-8") as outputs:
            for name, value in values.items():
                outputs.write(f"{name}={json.dumps(value, separators=(',', ':'))}\n")


HUNK = re.compile(rb"^@@ -[0-9]+(?:,[0-9]+)? \+([0-9]+)(?:,([0-9]+))? @@")


def changed_rust_ranges(base, tested, paths, root=ROOT):
    ranges = {}
    for path in (p for p in paths if p.endswith(".rs")):
        diff = git(root, "diff", "--no-ext-diff", "--no-textconv", "--no-renames", "--no-color",
                   "--unified=0", base, tested, "--", path)
        spans = []
        for line in diff.split(b"\n"):
            match = HUNK.match(line)
            if match:
                start, count = int(match[1]), int(match[2] or b"1")
                if count:
                    spans.append((start, start + count - 1))
        if spans:
            ranges[path] = spans
    return ranges


def project_path(raw, root, manifest=None):
    if not isinstance(raw, str) or not raw:
        return ()
    path = Path(raw)
    candidates = [root / path]
    if not path.is_absolute() and isinstance(manifest, str):
        candidates.append((root / manifest).parent / path)
    result = []
    for candidate in candidates:
        try:
            result.append(candidate.resolve().relative_to(root).as_posix())
        except (OSError, RuntimeError, ValueError):
            continue
    return tuple(dict.fromkeys(result))


def filter_clippy(lines, ranges, root=ROOT):
    """Keep original Cargo records only for warnings anchored in the PR diff."""
    for line in lines:
        record = json.loads(line)
        if record.get("reason") != "compiler-message":
            continue
        message = record["message"]
        code = message.get("code") or {}
        if message.get("level") != "warning" or not code.get("code", "").startswith("clippy::"):
            continue
        for span in message.get("spans", ()):
            if not span.get("is_primary"):
                continue
            paths = project_path(span.get("file_name"), root, record.get("manifest_path"))
            # Do not choose the package-relative interpretation just because it
            # hits the diff if there is an actual workspace-relative source.
            path = next((p for p in paths if (root / p).is_file()), None)
            if any(span["line_start"] <= end and span["line_end"] >= start
                   for start, end in ranges.get(path, ())):
                yield line
                break


def patch_coverage(lines, ranges, root=ROOT):
    """Count observed instrumented lines, not a percentage over unknown lines."""
    hits = defaultdict(dict)
    path = None
    for line in lines:
        if line.startswith("SF:"):
            path = next(iter(project_path(line[3:].rstrip("\r\n"), root)), None)
        elif line.rstrip("\r\n") == "end_of_record":
            path = None
        elif line.startswith("DA:") and path in ranges:
            number, count = map(int, line[3:].split(",")[:2])
            if number <= 0 or count < 0:
                raise ValueError("Invalid LCOV line counts")
            if any(start <= number <= end for start, end in ranges[path]):
                hits[path][number] = hits[path].get(number, 0) + count
    return {
        path: {
            "instrumented": len(hits[path]),
            "covered": sum(count > 0 for count in hits[path].values()),
            "unmapped": sum(end - start + 1 for start, end in spans) - len(hits[path]),
        }
        for path, spans in sorted(ranges.items())
    }


def summary(text):
    print(text)
    if filename := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(filename, "a", encoding="utf-8") as report:
            report.write(text + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("docs-only", "plan", "quality"))
    parser.add_argument("--execute-services", action="store_true")
    parser.add_argument("--event", default="pull_request")
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--tested", required=True)
    args = parser.parse_args()
    root = ROOT.resolve()
    if args.mode == "docs-only":
        try:
            paths = pr_paths(args.base, args.head, args.tested, root)
            selected = docs_only(paths, args.base, args.tested, root)
        except (ValueError, UnicodeError, subprocess.CalledProcessError):
            selected = False
        if selected:
            summary("Documentation-only PR: no build, tests, Clippy, coverage or Sonar required.")
        if selected:
            write_outputs(docs_only=True, selection=execution_scope("docs"))
        else:
            write_outputs(docs_only=False)
        return

    try:
        paths = pr_paths(args.base, args.head, args.tested, root) if args.event == "pull_request" else None
    except (ValueError, UnicodeError, subprocess.CalledProcessError):
        if args.mode != "plan":
            summary("PR quality not measured: could not verify the checkout and PR merge parents.")
            return
        paths = None
    if args.mode == "plan":
        from pipeline import metadata, validate_inventory

        data = metadata(root)
        validate_inventory(data, root)
        try:
            selected = service_scope(paths, args.base, args.tested, data, root) \
                if args.execute_services and args.event == "pull_request" else execution_scope("full")
        except (UnicodeError, subprocess.CalledProcessError):
            selected = execution_scope("full")
        summary("### Service CI execution scope\n\n"
                "```json\n" + json.dumps(selected, indent=2) + "\n```")
        write_outputs(selection=selected)
        return

    ranges = changed_rust_ranges(args.base, args.tested, paths, root)
    same_tree = git(root, "rev-parse", f"{args.head}^{{tree}}") == git(
        root, "rev-parse", f"{args.tested}^{{tree}}"
    )
    selected = parse_execution_scope(os.environ["CI_EXECUTION_SCOPE"])
    with (root / "lcov.info").open(encoding="utf-8") as lcov:
        files = patch_coverage(lcov, ranges, root)
        lcov.seek(0)
        reported = {
            path for line in lcov if line.startswith("SF:")
            for path in project_path(line[3:].rstrip("\r\n"), root)
        }
    missing = sorted(path for path in ranges
                     if "/tests/" not in path and path not in reported)
    coverage_compatible = selected["full"] or not missing
    directory = root / "target/sonar"
    with (directory / "clippy.json").open(encoding="utf-8") as original:
        diagnostics = list(filter_clippy((line for line in original if line.strip()), ranges, root))
    with (directory / "clippy-pr.json").open("x", encoding="utf-8", newline="\n") as report:
        report.writelines(diagnostics)
    value = {
        "identity": evidence_identity(), "base_sha": args.base, "head_sha": args.head,
        "coverage_scope": selected["coverage"], "changed_source_without_file_records": missing,
        "clippy_warnings": len(diagnostics), "changed_rust_lines": files,
    }
    with (directory / "pr-quality.json").open("x", encoding="utf-8") as report:
        json.dump(value, report, indent=2)
    totals = {key: sum(row[key] for row in files.values())
              for key in ("covered", "instrumented", "unmapped")}
    summary("### PR quality (same-run evidence)\n\n"
            f"- Changed Rust lines observed in LCOV: {totals['covered']} covered / "
            f"{totals['instrumented']} instrumented.\n"
            f"- Changed Rust lines without LCOV records: {totals['unmapped']} "
            "(may be non-executable, excluded, or unmeasured).\n"
            f"- Clippy records on changed lines: {len(diagnostics)}.")
    if not same_tree:
        summary("Sonar PR import not evaluated: the tested merge tree differs from the PR head. "
                "The same-run evidence above describes the tested merge.")
    if not coverage_compatible:
        summary("Sonar PR import not evaluated: selected coverage has no file records for "
                f"{len(missing)} changed Rust source file(s). See the evidence artifact.")
    write_outputs(report_ready=True, head_compatible=same_tree,
                  coverage_compatible=coverage_compatible, has_clippy=bool(diagnostics))


if __name__ == "__main__":
    main()
