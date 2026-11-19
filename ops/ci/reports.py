"""Admit complete Clippy evidence and report explicit selected CI outcomes."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys

from pipeline import BUILD_JOBS, INTEGRATION_JOBS, INVENTORY, ROOT, evidence_identity, parse_execution_scope


def clippy_packages(scope):
    if scope["full"]:
        return ["--workspace"]
    if scope["mode"] != "services":
        raise ValueError("Clippy is not part of the documentation-only scope")
    groups = {group for key in ("build", "integration") for group, enabled in scope[key].items()
              if enabled}
    return [arg for package in sorted({p for group in groups for p in INVENTORY["packages"][group]})
            for arg in ("-p", package)]


def clippy_evidence(directory, environment=os.environ):
    directory = Path(directory)
    lanes = []
    for name in ("production", "tests"):
        raw = (directory / f"clippy-{name}.jsonl").read_bytes()
        messages = [json.loads(line) for line in raw.splitlines() if line.strip()]
        terminals = [message for message in messages if message.get("reason") == "build-finished"]
        if not messages or len(terminals) != 1 or terminals[0].get("success") is not True:
            raise ValueError("Missing or unsuccessful terminal Clippy compiler record")
        if messages[-1] != terminals[0]:
            raise ValueError("Clippy report continues after its terminal compiler record")
        lanes.append(raw)
    combined = (directory / "clippy.json").read_bytes()
    if combined != b"".join(lanes):
        raise ValueError("Combined Clippy report differs from its complete lanes")
    receipt = {
        "schema": "aiforall.ci-clippy/v1",
        "identity": evidence_identity(environment),
        "report_sha256": hashlib.sha256(combined).hexdigest(),
        "policy_sha256": hashlib.sha256((ROOT / ".cargo/config.toml").read_bytes()).hexdigest(),
        "toolchain_sha256": hashlib.sha256((ROOT / "rust-toolchain.toml").read_bytes()).hexdigest(),
    }
    if "CI_EXECUTION_SCOPE" in environment:
        receipt["package_args"] = clippy_packages(parse_execution_scope(environment["CI_EXECUTION_SCOPE"]))
    return receipt


def status(needs, *, event, ref):
    if needs.get("plan", {}).get("result") != "success":
        raise ValueError("CI scope was not established")
    selection = parse_execution_scope(needs["plan"]["outputs"]["selection"])
    if event != "pull_request" and not selection["full"]:
        raise ValueError("Only PRs may use a reduced CI scope")
    expected = {"fmt", "plan", "clippy"}
    job_groups = {"build": BUILD_JOBS, "integration": INTEGRATION_JOBS}
    full_only = {"infra-static", "apparatus-p0"}
    all_lanes = set(BUILD_JOBS.values()) | set(INTEGRATION_JOBS.values()) | full_only
    if (needs["plan"]["outputs"].get("docs_only") == "true") != (selection["mode"] == "docs"):
        raise ValueError("Invalid documentation-only CI selection")
    if selection["mode"] == "docs":
        expected_skips = (expected - {"plan"}) | all_lanes | {
            "merge-coverage", "sonar", "k6-e2e", "e2e-pages"
        }
        unexpected = sorted(
            name for name in (expected_skips | set(needs)) - {"plan"}
            if needs.get(name, {}).get("result") != "skipped"
        )
        if unexpected:
            raise ValueError("Documentation-only jobs not skipped: " + ", ".join(unexpected))
        return "Documentation-only PR. Build, tests, Clippy, coverage and Sonar were NOT run."
    selected_lanes = {
        job for kind, groups in job_groups.items()
        for group, job in groups.items() if selection[kind][group]
    }
    if selection["full"]:
        selected_lanes.update(full_only)
    expected.update(selected_lanes)
    unexpected = sorted(
        name for name in all_lanes - selected_lanes
        if needs.get(name, {}).get("result") != "skipped"
    )
    if unexpected:
        raise ValueError("Out-of-scope CI jobs not skipped: " + ", ".join(unexpected))
    coverage_note = ""
    if selection["has_coverage"]:
        expected.add("merge-coverage")
        aggregation = needs.get("merge-coverage", {})
        if aggregation.get("result") == "success":
            outputs = aggregation.get("outputs", {})
            mode_output = "authoritative" if selection["full"] else "pr_ready"
            if (not selection["full"] and outputs.get("authoritative") != "false"
                    or selection["full"] and outputs.get("pr_ready") not in (None, "", "false")):
                raise ValueError("Coverage qualified for the wrong CI scope")
            qualified = outputs.get(mode_output)
            if qualified not in ("true", "false"):
                raise ValueError("Successful coverage aggregation has no qualification decision")
            if qualified == "true":
                expected.add("sonar")
            else:
                if needs.get("sonar", {}).get("result") != "skipped":
                    raise ValueError("Diagnostic coverage must not qualify Sonar analysis")
                coverage_note = (" Coverage was merged for diagnosis only; test execution is "
                                 "not qualified and Sonar was not evaluated.")
    if event == "push" and ref == "refs/heads/master":
        expected.update(("k6-e2e", "e2e-pages"))
    failed = sorted(
        name for name in expected if needs.get(name, {}).get("result") != "success"
    )
    # Unexpected failures are never ignored merely because a plan omitted them.
    failed += sorted(
        name for name, job in needs.items()
        if name not in expected and job.get("result") in ("failure", "cancelled")
    )
    if failed:
        raise ValueError("CI jobs did not succeed: " + ", ".join(failed) + coverage_note)
    if coverage_note:
        raise ValueError("Coverage is not authoritative despite successful selected jobs." + coverage_note)
    if event == "pull_request":
        prefix = ("Complete declared CI checks passed." if selection["full"] else
                  "Selected service checks passed. Full monorepo coverage was NOT evaluated.")
        if needs["sonar"].get("outputs", {}).get("scan_outcome") != "success":
            return f"{prefix} Sonar PR analysis was NOT evaluated; see the PR quality job."
        return f"{prefix} PR quality evidence was submitted to Sonar."
    return "Complete declared CI checks passed. Full coverage and Sonar were evaluated."


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("clippy-packages", "clippy-create", "clippy-verify", "status"))
    parser.add_argument("--directory", type=Path)
    args = parser.parse_args()
    if args.command == "clippy-packages":
        print("\n".join(clippy_packages(parse_execution_scope(os.environ["CI_EXECUTION_SCOPE"]))))
        return 0
    if args.command == "status":
        message = status(json.loads(os.environ["CI_NEEDS"]),
                         event=os.environ["CI_EVENT"], ref=os.environ["CI_REF"])
        print(message)
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as stream:
                stream.write(message + "\n")
        return 0
    value = clippy_evidence(args.directory)
    receipt = args.directory / "clippy-receipt.json"
    if args.command == "clippy-create":
        if receipt.exists():
            raise ValueError("Refusing stale Clippy receipt")
        receipt.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")
    elif json.loads(receipt.read_text(encoding="utf-8")) != value:
        raise ValueError("Foreign, stale or corrupted Clippy report")
    return 0


if __name__ == "__main__":
    sys.exit(main())
