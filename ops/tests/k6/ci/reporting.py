"""Public E2E projection: declared checks, no raw attachments or invented timing."""
import argparse
from datetime import datetime, timezone
import hashlib
import html
import json
from pathlib import Path
import re
import tarfile
import urllib.request
import uuid

from binaries import require, sha
import evidence
from campaign_browser import accept_result

HERE = Path(__file__).resolve().parent
CORRELATION = dict(iam_delta=7, ext_authz_delta=4, provider_delta=3, pkce_delta=3,
                    users_delta=1, outbox_delta=0, san_delta=1, restoration="PASS",
                    link_email=evidence.LINK_EMAIL_CORRELATION)
K6_THRESHOLDS_FAILED = 99


def read(path):
    try:
        path = Path(path)
        if (path.is_symlink() or any(parent.is_symlink() for parent in path.parents)
                or not path.is_file() or path.stat().st_size > evidence.SUMMARY_LIMIT):
            return None
        value = evidence.strict_json(path.read_bytes())
        return value if isinstance(value, dict) else None
    except (OSError, ValueError):
        return None


def write(path, value):
    path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")


def check_counts(counts):
    return f"{counts['passes']} passed, {counts['fails']} failed"


def failure_details(case, process, run_id):
    if not isinstance(process, dict):
        return None
    code = process.get("exit")
    status = process.get("summary_status")
    has_exit = type(code) is int and -255 <= code <= 255
    if not (has_exit and code != 0) and process.get("cleanup") is not False and status not in (
            "missing", "invalid", "not_collected", "captured"):
        return None
    message = f"{case}: k6 exit {code}" if has_exit else f"{case}: k6 exit unavailable"
    if has_exit and code == K6_THRESHOLDS_FAILED:
        message += " (thresholds not met)"
    observed = {}
    complete = False
    if status == "captured" and process.get("cleanup") is not True:
        status = "invalid"
    if status == "captured":
        try:
            summary = evidence.inspect_failed_summary(process.get("failed_summary"), run_id, case)
        except evidence.EvidenceError:
            status = "invalid"
        else:
            observed = summary["checks"]
            complete = (summary["started"] == summary["completed"] == summary["iterations"] == 1
                        and summary["requests"] == evidence.CATALOG[case]["requests"])
            counters = ", ".join(f"{key}={summary[key] if summary[key] is not None else 'unavailable'}"
                                 for key in ("started", "completed", "iterations", "requests"))
            failed = ", ".join(name for name in sorted(observed) if observed[name]["fails"])
            message += f"; {counters}; failed checks: {failed or 'none observed'}"
            message += f"; observed checks: {len(observed)}/{len(evidence.CASES[case][1])}"
    if status != "captured":
        state = {"missing": "missing", "invalid": "invalid", "not_collected": "not collected"}.get(
            status if isinstance(status, str) else None, "unavailable")
        message += f"; diagnostic summary {state}"
    for key, label in (("cleanup", "child_cleanup"), ("sandbox", "sandbox"), ("renderer_observed", "renderer_observed")):
        if type(process.get(key)) is bool:
            message += f"; {label}={'yes' if process[key] else 'no'}"
    return message, observed, complete


def project(source, destination, repository, job_run, attempt, harness_sha, target_sha, summary_path=None):
    require(re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository) is not None, "REPORT_REPOSITORY")
    require(all(re.fullmatch(r"[a-f0-9]{40}", value) for value in (harness_sha, target_sha)), "REPORT_SOURCE_SHA")
    require(all(re.fullmatch(r"[1-9][0-9]*", value) for value in (job_run, attempt)), "REPORT_RUN")
    source, destination = Path(source), Path(destination)
    require(not destination.exists(), "REPORT_OUTPUT_COLLISION")
    destination.mkdir(parents=True)
    results = destination / "allure-results"
    results.mkdir()
    report = read(source / "report.json") or {}
    provenance = read(source / "provenance.json") or {}
    run_id = report.get("run_id")
    source_ok = (provenance.get("harness_sha") == harness_sha and provenance.get("target_sha") == target_sha
                 and isinstance(run_id, str) and re.fullmatch(r"[a-f0-9]{32}", run_id) is not None)
    source_ok = source_ok and read(source / "selection.json") == evidence.selection_receipt(run_id, harness_sha, target_sha)
    processes = read(source / "processes.json") or {}
    accepted = {}
    if source_ok:
        for case in evidence.CASES:
            value = read(source / "cases" / (case + ".json"))
            try:
                process = processes.get(case)
                if value is not None and isinstance(process, dict) and process.get("cleanup") is True:
                    accepted[case] = evidence.accept_summary(json.dumps(value).encode(), run_id, case, process.get("exit"))
            except ValueError:
                pass
    cleanup = read(source / "cleanup.json") or {}
    browser = read(source / "browser.json") or {}
    browser_ok = False
    try:
        observed = browser.get("result", {})
        # The observed ordinary UID must agree across all kernel witnesses.
        uid = observed["witnesses"][0]["uid"]
        observed = accept_result(json.dumps(observed).encode(), run_id, uid)
        browser_ok = (type(browser.get("container_exit")) is int and browser["container_exit"] == 0 and browser.get("cleanup") == "PASS"
                      and observed["exit"] == 0 and observed["cleanup"] and observed["sandbox"] and observed["renderer_observed"])
    except (ValueError, KeyError, IndexError, TypeError):
        pass
    correlation = read(source / "correlation.json")
    correlation_ok = (correlation == CORRELATION
                      and all(evidence.integer(value) for value in correlation["link_email"].values()))
    all_cases_accepted = source_ok and len(accepted) == len(evidence.CASES)
    link_state_failed = all_cases_accepted and report.get("diagnostic") == "LINK_EMAIL_DATABASE_EFFECTS"
    parent_ok = (all_cases_accepted and report.get("verdict") == "PASS"
        and report.get("cleanup") == "PASS" and report.get("restoration") == "PASS"
        and report.get("completed_cases") == list(evidence.CASES)
        and cleanup.get("status") == "PASS" and type(cleanup.get("owned_running")) is int
        and cleanup["owned_running"] == 0 and cleanup.get("errors") == []
        and correlation_ok and browser_ok
        and all(isinstance(processes.get(case), dict) and type(processes[case].get("exit")) is int and processes[case]["exit"] == 0
                and processes[case].get("cleanup") is True for case in evidence.CASES))
    passed = sum(len(value["checks"]) for value in accepted.values())
    status = "passed" if parent_ok else "failed" if source_ok else "incomplete"
    run_url = f"https://github.com/{repository}/actions/runs/{job_run}"
    failures = {}
    if source_ok:
        for case in evidence.CASES:
            if case in accepted:
                continue
            details = failure_details(case, processes.get(case), run_id)
            if details:
                failures[case] = details
            elif report.get("verdict") == "FAIL" and report.get("stage") == "scenario:" + case:
                failures[case] = (f"{case}: scenario did not complete; no process evidence available", {}, False)
    run_link = [dict(name="GitHub Actions run", type="link", url=run_url)]
    expected = sum(len(record["checks"]) for record in evidence.CATALOG.values())
    categories = set()
    if not parent_ok:
        if not source_ok:
            categories.add("incomplete")
        if cleanup.get("status") == "FAIL" or report.get("restoration") == "FAIL" or any(
                isinstance(process, dict) and process.get("cleanup") is False for process in processes.values()):
            categories.add("cleanup")
        if link_state_failed:
            categories.add("functional")
        elif (all_cases_accepted
                and (not correlation_ok or not browser_ok
                     or report.get("restoration") not in ("PASS", "FAIL")
                      or (report.get("verdict") != "PASS" and cleanup.get("status") == "PASS"
                          and report.get("restoration") == "PASS"))):
            categories.add("infrastructure")
        if cleanup.get("status") not in ("PASS", "FAIL"):
            categories.add("incomplete")
    for case in evidence.CASES:
        if case not in accepted:
            _, observed, complete = failures.get(case, ("", {}, False))
            if not complete or any(observed.get(name) != {"passes": 1, "fails": 0}
                                   and not observed.get(name, {}).get("fails") for name in evidence.CASES[case][1]):
                categories.add("incomplete")
            for name, counts in observed.items():
                if counts["fails"]:
                    categories.add(evidence.CATALOG[case]["checks"][name])
            process = processes.get(case)
            if (isinstance(process, dict) and type(process.get("exit")) is int
                    and process["exit"] not in (0, K6_THRESHOLDS_FAILED)
                    and not any(counts["fails"] for counts in observed.values())):
                categories.add("infrastructure")
    if source_ok and report.get("verdict") == "FAIL" and not categories - {"incomplete"}:
        stage = report.get("stage")
        if isinstance(stage, str) and not stage.startswith("scenario:"):
            categories.add("infrastructure")
    metric = dict(schema="aiforall.e2e-status/v1", status=status, passed=passed, expected=expected,
        unproven=expected - passed, source_sha=target_sha, harness_sha=harness_sha,
        failure_categories=sorted(categories),
        job_run_id=int(job_run), job_attempt=int(attempt), run_url=run_url,
        generated_at=datetime.now(timezone.utc).isoformat())
    write(destination / "status.json", metric)
    write(destination / "badge.json", dict(schemaVersion=1, label="E2E", message=f"{passed}/{expected}" if parent_ok else f"{status} ({passed}/{expected})",
                                          color="brightgreen" if parent_ok else "red", cacheSeconds=300))
    for case, (_, checks, _) in evidence.CASES.items():
        for check in sorted(checks):
            key = case + "." + check
            identity = str(uuid.uuid5(uuid.NAMESPACE_URL, job_run + ":" + attempt + ":" + key))
            stable_id = hashlib.sha256(key.encode()).hexdigest()
            value = dict(uuid=identity, historyId=stable_id, testCaseId=stable_id, fullName=key,
                         name=check, status="passed" if case in accepted else "broken", stage="finished",
                         labels=[dict(name="parentSuite", value="E2E"), dict(name="suite", value=case)])
            if case not in accepted:
                category = "incomplete"
                message = "Named check not proven by a valid same-source phase summary"
                if case in failures:
                    detail, observed, _ = failures[case]
                    counts = observed.get(check)
                    if counts is None:
                        message = detail + "; no observation for this check"
                    else:
                        message = detail + "; check counts: " + check_counts(counts)
                        if counts["fails"]:
                            value["status"] = "failed"
                            category = evidence.CATALOG[case]["checks"][check]
                value["labels"].append(dict(name="failureCategory", value=category))
                value["statusDetails"] = dict(message=message)
                value["links"] = run_link
            write(results / (identity + "-result.json"), value)
    identity = str(uuid.uuid5(uuid.NAMESPACE_URL, job_run + ":" + attempt + ":campaign-contract"))
    parent = dict(uuid=identity, historyId="aiforall-e2e-parent-contract",
        testCaseId=hashlib.sha256(b"aiforall-e2e-parent-contract").hexdigest(),
        fullName="E2E.parent-contract", name="Parent deltas, TLS restoration, sandbox and cleanup",
        status="passed" if parent_ok else "broken", stage="finished",
        labels=[dict(name="parentSuite", value="E2E"), dict(name="suite", value="Campaign contract")])
    if not parent_ok:
        parent_category = next((name for name in ("cleanup", "infrastructure") if name in categories),
                               "functional" if link_state_failed else "incomplete")
        fallback = ("LINK response checks completed, but persisted state did not match" if link_state_failed
                    else "Campaign contract not proven; no scenario failure details available")
        parent["labels"].append(dict(name="failureCategory", value=parent_category))
        parent["statusDetails"] = dict(message="\n".join(detail for detail, _, _ in failures.values()) or fallback)
        parent["links"] = run_link
    write(results / (identity + "-result.json"), parent)
    write(results / "executor.json", dict(name="GitHub Actions", type="github", buildName=f"E2E {job_run}/{attempt}",
        buildOrder=int(job_run), buildUrl=run_url))
    (results / "environment.properties").write_text(f"applications={target_sha}\nharness={harness_sha}\n", encoding="ascii")
    page = ("<!doctype html><html lang='en'><meta charset='utf-8'><title>AIForAll E2E</title>"
        "<style>body{font:18px system-ui;max-width:850px;margin:4rem auto;padding:1rem}code{font-size:14px}</style>"
        f"<h1>AIForAll — E2E {html.escape(status)}</h1><p>{passed}/{expected} declared checks proven.</p>"
        "<p>The parent contract is evaluated separately: all checks passing is insufficient if correlation, sandbox or cleanup fails.</p>"
        f"<p><a href='allure/'>Allure report</a> · <a href='{run_url}'>GitHub run</a> · <a href='status.json'>JSON metrics</a></p>"
        f"<p>Applications: <code>{target_sha}</code><br>Harness: <code>{harness_sha}</code></p>"
        f"<p>Latest published run, not code coverage. Allure contains {expected} declared checks and a distinct parent contract."
        " Timings and test history are not invented.</p></html>")
    (destination / "index.html").write_text(page, encoding="utf-8")
    (destination / ".nojekyll").write_text("", encoding="ascii")
    for case, (detail, observed, _) in failures.items():
        print("E2E failure: " + detail)
        for check, counts in sorted(observed.items()):
            print(f"E2E observation: {case}.{check}: {check_counts(counts)}")
    if summary_path:
        with Path(summary_path).open("a", encoding="utf-8") as stream:
            stream.write(f"## E2E : {status} — {passed}/{expected} checks\n\nApplications `{target_sha}` ; harness `{harness_sha}`.\n\n"
                         f"[Run]({run_url}) · [Public reports](https://{repository.split('/')[0].lower()}.github.io/{repository.split('/')[1]}/)\n")
            for detail, observed, _ in failures.values():
                stream.write("\n- " + detail + "\n")
                for check, counts in sorted(observed.items()):
                    stream.write(f"  - `{check}`: {check_counts(counts)}\n")
    return metric


def install_allure(directory):
    pin = json.loads((HERE / "allure.lock.json").read_text())
    version = pin["version"]
    require(re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version) is not None
            and pin["url"] == f"https://github.com/allure-framework/allure2/releases/download/{version}/allure-{version}.tgz"
            and re.fullmatch(r"[a-f0-9]{64}", pin["sha256"]) is not None, "ALLURE_PIN")
    directory = Path(directory)
    require(directory.is_absolute() and not directory.exists(), "ALLURE_OUTPUT_COLLISION")
    directory.mkdir(mode=0o700)
    archive = directory / "allure.tgz"
    with urllib.request.urlopen(pin["url"], timeout=60) as response, archive.open("xb") as target:
        total = 0
        while block := response.read(1024 * 1024):
            total += len(block)
            require(total <= pin["bytes"], "ALLURE_SIZE")
            target.write(block)
    require(total == pin["bytes"] and sha(archive) == pin["sha256"], "ALLURE_HASH")
    with tarfile.open(archive, "r:gz") as bundle:
        members = bundle.getmembers()
        require(len(members) <= 4096 and sum(member.size for member in members) <= 160 * 1024**2
                and all((member.isfile() or member.isdir()) and not member.name.startswith("/")
                        and ".." not in Path(member.name).parts
                        and Path(member.name).parts[0] == "allure-" + version for member in members), "ALLURE_ARCHIVE")
        bundle.extractall(directory, members=members, filter="data")
    return directory / ("allure-" + version) / "bin/allure"


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("project", "install-allure"))
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--source", type=Path)
    parser.add_argument("--repository")
    parser.add_argument("--job-run")
    parser.add_argument("--attempt")
    parser.add_argument("--harness-sha")
    parser.add_argument("--target-sha")
    parser.add_argument("--summary", type=Path)
    args = parser.parse_args()
    try:
        if args.action == "install-allure":
            print(install_allure(args.directory))
        else:
            project(args.source, args.directory, args.repository, args.job_run, args.attempt,
                    args.harness_sha, args.target_sha, args.summary)
    except Exception:
        raise SystemExit("E2E public reporting refused; no raw input or exception exported") from None
