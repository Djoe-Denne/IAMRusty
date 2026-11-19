"""Admit complete Clippy evidence and report explicit selected CI outcomes."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys

from pipeline import GROUPS, ROOT, evidence_identity


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
    return {
        "schema": "aiforall.ci-clippy/v1",
        "identity": evidence_identity(environment),
        "report_sha256": hashlib.sha256(combined).hexdigest(),
        "policy_sha256": hashlib.sha256((ROOT / ".cargo/config.toml").read_bytes()).hexdigest(),
        "toolchain_sha256": hashlib.sha256((ROOT / "rust-toolchain.toml").read_bytes()).hexdigest(),
    }


def status(needs, *, event, ref):
    if needs.get("plan", {}).get("result") != "success":
        raise ValueError("CI scope was not established")
    selection = json.loads(needs["plan"]["outputs"]["selection"])
    if event == "push" and ref == "refs/heads/master" and not selection["full"]:
        raise ValueError("Master requires complete declared CI scope")
    expected = {"fmt", "plan", "infra-static", "clippy", "apparatus-p0"}
    mappings = {
        "iam": ("build-iam", "test-iam"),
        "telegraph": ("build-telegraph", "test-telegraph"),
        "hive": ("build-hive", "test-hive"),
        "manifesto": ("build-manifesto", "test-manifesto"),
        "monolith": ("build-monolith",),
        "sentinel-sync": ("build-sentinel", "test-sentinel"),
        "ext-authz": ("build-ext-authz", "ext-authz-unit"),
        "oauth": ("build-oauth",),
        "readiness": ("test-readiness",),
        "shared": ("test-shared",),
    }
    if needs["plan"]["outputs"].get("docs_only") == "true":
        empty_scope = dict.fromkeys((*GROUPS, "full", "any"), False)
        if event != "pull_request" or selection != empty_scope:
            raise ValueError("Invalid documentation-only CI selection")
        expected_skips = (expected - {"plan"}) | {
            job for jobs in mappings.values() for job in jobs
        } | {"merge-coverage", "sonar", "k6-e2e", "e2e-pages"}
        unexpected = sorted(
            name for name in (expected_skips | set(needs)) - {"plan"}
            if needs.get(name, {}).get("result") != "skipped"
        )
        if unexpected:
            raise ValueError("Documentation-only jobs not skipped: " + ", ".join(unexpected))
        return "Documentation-only PR. Build, tests, Clippy, coverage and Sonar were NOT run."
    for group, jobs in mappings.items():
        if selection[group]:
            expected.update(jobs)
    coverage_note = ""
    if selection["full"]:
        expected.add("merge-coverage")
        aggregation = needs.get("merge-coverage", {})
        if aggregation.get("result") == "success":
            authoritative = aggregation.get("outputs", {}).get("authoritative")
            if authoritative not in ("true", "false"):
                raise ValueError("Successful coverage aggregation has no qualification decision")
            if authoritative == "true":
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
    if not selection["full"]:
        if any(needs.get(name, {}).get("result") != "skipped" for name in ("merge-coverage", "sonar")):
            raise ValueError("Partial coverage must not qualify authoritative Sonar analysis")
        return "Selected checks passed. Full monorepo coverage and Sonar were NOT evaluated."
    if event == "pull_request":
        if needs["sonar"].get("outputs", {}).get("scan_outcome") != "success":
            return ("Complete declared CI checks passed. Sonar PR analysis was NOT evaluated; "
                    "see the PR quality job for available evidence.")
        return "Complete declared CI checks passed. PR quality evidence was submitted to Sonar."
    return "Complete declared CI checks passed. Full coverage and Sonar were evaluated."


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("clippy-create", "clippy-verify", "status"))
    parser.add_argument("--directory", type=Path)
    args = parser.parse_args()
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
