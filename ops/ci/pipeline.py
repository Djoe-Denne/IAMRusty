"""Conservative CI selection and complete, disjoint Cargo test-target partitions."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]
# These are workflow identities, not executable input supplied by a PR.
GROUPS = {
    "iam": ("services/IAMRusty", 4),
    "telegraph": ("services/Telegraph", 2),
    "hive": ("services/Hive", 2),
    "manifesto": ("services/Manifesto", 3),
    "lazaret": ("services/Lazaret", 2),
    "oauth": ("services/OAuthConnect", 1),
    "sentinel-sync": ("workers/sentinel-sync", 1),
    "ext-authz": ("workers/ext-authz", 1),
    "operator": ("workers/apparatus-operator", 1),
    "monolith": ("runtime/monolith", 1),
    "readiness": ("crates/readiness", 1),
    "shared": ("crates", 1),
}
FEATURES = {"shared": ["apparatus-contracts/test-harness"]}
SHA = re.compile(r"[a-f0-9]{40}")
INVENTORY_PATH = ROOT / ".github/ci/inventory.json"
INVENTORY = json.loads(INVENTORY_PATH.read_text(encoding="utf-8"))


def metadata(root=ROOT):
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=root, capture_output=True, text=True, check=True,
    )
    return json.loads(result.stdout)


def relative_manifest(package, root):
    return Path(package["manifest_path"]).resolve().relative_to(Path(root).resolve())


def owner(path):
    normalized = path.replace("\\", "/")
    for group, (prefix, _) in GROUPS.items():
        if normalized == prefix or normalized.startswith(prefix + "/"):
            return group
    return None


def packages_for(data, group, root=ROOT):
    if group not in GROUPS:
        raise ValueError("Unknown CI service")
    return sorted(
        [p for p in data["packages"] if owner(relative_manifest(p, root).as_posix()) == group],
        key=lambda p: p["name"],
    )


def coverage_packages(data, group, root=ROOT):
    names = INVENTORY["coverage"].get(group, [])
    packages = {p["name"]: p for p in packages_for(data, group, root)}
    if not set(names).issubset(packages):
        raise ValueError("Coverage inventory names a missing or foreign package")
    return [packages[name] for name in names]


def validate_inventory(data, root=ROOT):
    """An unregistered workspace package must fail CI, never disappear silently."""
    registered = {
        name: group for group, names in INVENTORY["packages"].items() for name in names
    }
    actual = set()
    for package in data["packages"]:
        name = package["name"]
        group = owner(relative_manifest(package, root).as_posix())
        if name not in registered or registered[name] != group or name in actual:
            raise ValueError(f"Unregistered CI package: {package['name']}")
        actual.add(name)
    if actual != set(registered):
        raise ValueError("Registered CI package missing from Cargo metadata")
    for group in GROUPS:
        if not packages_for(data, group, root):
            raise ValueError(f"Empty CI service inventory: {group}")
    for group in INVENTORY["coverage"]:
        coverage_packages(data, group, root)


def selected_groups(paths, data, *, event, root=ROOT):
    """Master/push is complete; PR selection follows conservative reverse path edges."""
    if event != "pull_request" or not paths:
        return set(GROUPS)
    packages = data["packages"]
    directories = {
        Path(p["manifest_path"]).resolve().parent: p["name"] for p in packages
    }
    by_name = {p["name"]: p for p in packages}
    changed = set()
    for path in paths:
        if path.endswith(("Cargo.toml", "Cargo.lock")):
            return set(GROUPS)
        # Only ordinary documentation is allowed to skip Rust qualification.
        if (path == "README.md" or path.startswith("docs/")) and path.endswith(".md"):
            continue
        group = owner(path)
        if group is None:
            return set(GROUPS)
        if group == "shared":
            return set(GROUPS)
        changed.update(p["name"] for p in packages_for(data, group, root))
    affected = set(changed)
    while True:
        previous = len(affected)
        for package in packages:
            for dependency in package.get("dependencies", []):
                dep_path = dependency.get("path")
                dependency_name = directories.get(Path(dep_path).resolve()) if dep_path else None
                if dependency_name in affected:
                    affected.add(package["name"])
        if len(affected) == previous:
            break
    groups = {
        owner(relative_manifest(by_name[name], root).as_posix()) for name in affected
    }
    while True:
        expanded = groups | {
            consumer for group in groups
            for consumer in INVENTORY["runtime_consumers"].get(group, [])
        }
        if expanded == groups:
            return groups
        groups = expanded


def changed_paths(base, head, root=ROOT):
    if not SHA.fullmatch(base or "") or not SHA.fullmatch(head or ""):
        raise ValueError("Change selection requires two exact commit SHAs")
    # No rename detection: both sides of a rename must contribute to selection.
    result = subprocess.run(
        ["git", "diff", "--no-renames", "--name-only", "-z", base, head, "--"],
        cwd=root, capture_output=True, check=True,
    )
    return [p.decode("utf-8", errors="strict") for p in result.stdout.split(b"\0") if p]


def enabled_features(package, group):
    result = set(package.get("features", {}).get("default", []))
    for feature in FEATURES.get(group, []):
        package_name, name = feature.split("/", 1)
        if package_name == package["name"]:
            result.add(name)
    while True:
        expanded = result | {
            child for feature in result
            for child in package.get("features", {}).get(feature, [])
            if ":" not in child and "/" not in child
        }
        if expanded == result:
            return result
        result = expanded


def test_targets(data, group, root=ROOT):
    targets = []
    for package in coverage_packages(data, group, root):
        enabled = enabled_features(package, group)
        for target in package["targets"]:
            if "test" not in target["kind"] or not target.get("test", True):
                continue
            if not set(target.get("required-features", [])).issubset(enabled):
                continue
            targets.append((package["name"], target["name"]))
    return sorted(targets)


def partition(targets, count, index):
    if count < 1 or not 0 <= index < count:
        raise ValueError("Invalid test partition")
    return [
        item for item in targets
        if int.from_bytes(hashlib.sha256("\0".join(item).encode()).digest()[:8], "big") % count == index
    ]


def coverage_commands(data, group, lane, shard, root=ROOT):
    packages = coverage_packages(data, group, root)
    extra = []
    if FEATURES.get(group):
        extra = ["--features", ",".join(FEATURES[group])]
    common = ["cargo", "llvm-cov", "--locked", "--no-report", "--no-fail-fast"]
    if lane == "unit":
        if shard != 0:
            raise ValueError("Unit coverage has exactly one partition")
        eligible = [
            p for p in packages
            if any(t["kind"][0] in ("lib", "bin", "proc-macro") and t.get("test", True)
                   for t in p["targets"])
        ]
        if not eligible:
            return []
        args = [arg for p in eligible for arg in ("--package", p["name"])]
        return [[*common, *args, *extra, "--lib", "--bins"]]
    if lane != "tests":
        raise ValueError("Unknown coverage lane")
    selected = partition(test_targets(data, group, root), GROUPS[group][1], shard)
    commands = []
    for package in packages:
        names = [name for pkg, name in selected if pkg == package["name"]]
        if names:
            commands.append([
                *common, "--package", package["name"], *extra,
                *[arg for name in names for arg in ("--test", name)],
            ])
    return commands


def target_assignment(data, group, lane, shard, root=ROOT):
    if lane == "tests":
        return [
            f"{package}|test|{target}"
            for package, target in partition(test_targets(data, group, root), GROUPS[group][1], shard)
        ]
    if lane != "unit" or shard != 0:
        raise ValueError("Invalid coverage assignment")
    return sorted(
        f"{p['name']}|{t['kind'][0]}|{t['name']}"
        for p in coverage_packages(data, group, root)
        for t in p["targets"]
        if t["kind"][0] in ("lib", "bin", "proc-macro") and t.get("test", True)
        and set(t.get("required-features", [])).issubset(enabled_features(p, group))
    )


def evidence_identity(environment=os.environ):
    values = {name: environment.get(name, "") for name in
              ("GITHUB_SHA", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT")}
    if not SHA.fullmatch(values["GITHUB_SHA"]) or any(
        not re.fullmatch(r"[1-9][0-9]*", values[name])
        for name in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT")
    ):
        raise ValueError("Coverage requires exact source/run/attempt identity")
    return values


def run_coverage(data, group, lane, shard, output, root=ROOT, run=subprocess.run,
                 environment=os.environ):
    """Collect all selected results and preserve any test/report failure."""
    commands = coverage_commands(data, group, lane, shard, root)
    if not commands:
        raise ValueError("Empty coverage partition must not produce a passing report")
    identity = evidence_identity(environment)
    scope = environment.get("CI_COVERAGE_SCOPE", "")
    if scope not in ("full", "partial"):
        raise ValueError("Coverage scope must be explicit")
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists() or output.with_suffix(".json").exists():
        raise ValueError("Refusing stale coverage output")
    status = 0
    for command in commands:
        result = run(command, cwd=root, check=False)
        status = status or result.returncode
    report = run(
        ["cargo", "llvm-cov", "report", "--lcov", "--output-path", str(output)],
        cwd=root, check=False,
    )
    receipt = {
        "schema": "aiforall.ci-coverage/v1", "identity": identity, "scope": scope,
        "inventory_sha256": hashlib.sha256(INVENTORY_PATH.read_bytes()).hexdigest(),
        "toolchain_sha256": hashlib.sha256((Path(root) / "rust-toolchain.toml").read_bytes()).hexdigest(),
        "service": group, "lane": lane, "shard": shard,
        "features": FEATURES.get(group, []),
        "targets": target_assignment(data, group, lane, shard, root),
        "test_status": status, "report_status": report.returncode,
        "report": output.name,
        "report_sha256": hashlib.sha256(output.read_bytes()).hexdigest() if output.is_file() else None,
    }
    output.with_suffix(".json").write_text(json.dumps(receipt, sort_keys=True), encoding="utf-8")
    return status or report.returncode


def matrix_for(data, group, root=ROOT):
    rows = []
    if coverage_commands(data, group, "unit", 0, root):
        rows.append({"lane": "unit", "shard": 0})
    targets = test_targets(data, group, root)
    for index in range(GROUPS[group][1]):
        if partition(targets, GROUPS[group][1], index):
            rows.append({"lane": "tests", "shard": index})
    return {"include": rows}


def validate_reports(data, directory, *, identity, root=ROOT):
    directory = Path(directory)
    expected = {
        (group, row["lane"], row["shard"]): target_assignment(
            data, group, row["lane"], row["shard"], root)
        for group in INVENTORY["coverage"]
        for row in matrix_for(data, group, root)["include"]
    }
    found, reports, targets = set(), [], set()
    inventory_sha = hashlib.sha256(INVENTORY_PATH.read_bytes()).hexdigest()
    toolchain_sha = hashlib.sha256((Path(root) / "rust-toolchain.toml").read_bytes()).hexdigest()
    for path in sorted(directory.glob("*.json")):
        receipt = json.loads(path.read_text(encoding="utf-8"))
        key = (receipt.get("service"), receipt.get("lane"), receipt.get("shard"))
        assigned = receipt.get("targets", [])
        if (key not in expected or key in found
                or receipt.get("schema") != "aiforall.ci-coverage/v1"
                or receipt.get("scope") != "full"
                or receipt.get("identity") != identity
                or receipt.get("inventory_sha256") != inventory_sha
                or receipt.get("toolchain_sha256") != toolchain_sha
                or receipt.get("features") != FEATURES.get(key[0], [])
                or assigned != expected[key]
                or receipt.get("test_status") != 0 or receipt.get("report_status") != 0):
            raise ValueError("Missing, foreign, duplicate or failed coverage evidence")
        name = receipt.get("report")
        if not isinstance(name, str) or Path(name).name != name or not name.endswith(".info"):
            raise ValueError("Invalid coverage report path")
        report = directory / name
        if (not report.is_file() or report.is_symlink()
                or report.stat().st_size == 0
                or hashlib.sha256(report.read_bytes()).hexdigest() != receipt.get("report_sha256")):
            raise ValueError("Missing or corrupted coverage report")
        if targets.intersection(assigned):
            raise ValueError("Coverage targets executed in multiple partitions")
        targets.update(assigned)
        found.add(key)
        reports.append(report)
    if found != set(expected) or set(directory.glob("*.info")) != set(reports):
        raise ValueError("Incomplete or unexpected coverage report set")
    return reports


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    select = commands.add_parser("select")
    select.add_argument("--base", default="")
    select.add_argument("--head", default="")
    select.add_argument("--event", required=True)
    select.add_argument("--selective", action="store_true")
    matrix = commands.add_parser("matrix")
    matrix.add_argument("--service", choices=GROUPS, required=True)
    coverage = commands.add_parser("coverage")
    coverage.add_argument("--service", choices=GROUPS, required=True)
    coverage.add_argument("--lane", choices=("unit", "tests"), required=True)
    coverage.add_argument("--shard", type=int, required=True)
    coverage.add_argument("--output", type=Path, required=True)
    aggregate = commands.add_parser("aggregate")
    aggregate.add_argument("--directory", type=Path, required=True)
    aggregate.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    data = metadata()
    validate_inventory(data)
    if args.command == "select":
        try:
            paths = changed_paths(args.base, args.head) if args.selective and args.event == "pull_request" else []
            selected = selected_groups(paths, data, event=args.event) if args.selective else set(GROUPS)
        except (ValueError, subprocess.CalledProcessError, UnicodeError):
            # Missing/shallow history or ambiguous input can only increase work.
            selected = set(GROUPS)
        result = {name: name in selected for name in GROUPS}
        result["full"] = selected == set(GROUPS)
        result["any"] = bool(selected)
        text = json.dumps(result, sort_keys=True, separators=(",", ":"))
        print(text)
        if os.environ.get("GITHUB_OUTPUT"):
            with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as stream:
                stream.write("selection=" + text + "\n")
    elif args.command == "matrix":
        text = json.dumps(matrix_for(data, args.service), separators=(",", ":"))
        print(text)
        if os.environ.get("GITHUB_OUTPUT"):
            with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as stream:
                stream.write("matrix=" + text + "\n")
    elif args.command == "aggregate":
        reports = validate_reports(data, args.directory, identity=evidence_identity())
        command = ["lcov", *[arg for report in reports for arg in ("--add-tracefile", str(report))],
                   "--output-file", str(args.output)]
        return subprocess.run(command, cwd=ROOT, check=False).returncode
    else:
        return run_coverage(data, args.service, args.lane, args.shard, args.output)
    return 0


if __name__ == "__main__":
    sys.exit(main())
