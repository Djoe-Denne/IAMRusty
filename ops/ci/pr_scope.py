"""Advisory service selection and same-run PR quality evidence."""

import argparse
from collections import defaultdict
import json
import os
from pathlib import Path
import re
import subprocess

from pipeline import GROUPS, INVENTORY, ROOT, SHA, changed_paths, evidence_identity, owner


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
    parser.add_argument("mode", choices=("plan", "quality"))
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--tested", required=True)
    args = parser.parse_args()
    root = ROOT.resolve()
    try:
        paths = pr_paths(args.base, args.head, args.tested, root)
    except (ValueError, UnicodeError, subprocess.CalledProcessError):
        if args.mode != "plan":
            summary("PR quality not measured: could not verify the checkout and PR merge parents.")
            return
        paths = None
    if args.mode == "plan":
        from pipeline import metadata, validate_inventory

        data = metadata(root)
        validate_inventory(data, root)
        plan = shadow_plan(paths, data, root)
        summary("### Service pruning preview (does not skip jobs)\n\n"
                "```json\n" + json.dumps(plan, indent=2) + "\n```")
        return

    ranges = changed_rust_ranges(args.base, args.tested, paths, root)
    same_tree = git(root, "rev-parse", f"{args.head}^{{tree}}") == git(
        root, "rev-parse", f"{args.tested}^{{tree}}"
    )
    with (root / "lcov.info").open(encoding="utf-8") as lcov:
        files = patch_coverage(lcov, ranges, root)
    directory = root / "target/sonar"
    with (directory / "clippy.json").open(encoding="utf-8") as original:
        diagnostics = list(filter_clippy((line for line in original if line.strip()), ranges, root))
    with (directory / "clippy-pr.json").open("x", encoding="utf-8", newline="\n") as report:
        report.writelines(diagnostics)
    value = {
        "identity": evidence_identity(), "base_sha": args.base, "head_sha": args.head,
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
    if filename := os.environ.get("GITHUB_OUTPUT"):
        with open(filename, "a", encoding="utf-8") as outputs:
            outputs.write("report_ready=true\n")
            outputs.write(f"head_compatible={str(same_tree).lower()}\n")
            outputs.write(f"has_clippy={str(bool(diagnostics)).lower()}\n")


if __name__ == "__main__":
    main()
