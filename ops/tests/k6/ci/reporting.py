"""Public E2E projection: fixed checks, no raw attachments or invented timing."""
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
                   users_delta=1, outbox_delta=0, san_delta=1, restoration="PASS")
EXPECTED = sum(len(value[1]) for value in evidence.CASES.values())


def read(path):
    try:
        path = Path(path)
        if path.is_symlink() or not path.is_file() or path.stat().st_size > 65536:
            return None
        value = evidence.strict_json(path.read_bytes())
        return value if isinstance(value, dict) else None
    except (OSError, ValueError):
        return None


def write(path, value):
    path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")


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
    accepted = {}
    if source_ok:
        for case in evidence.CASES:
            value = read(source / (case + ".json"))
            try:
                if value is not None:
                    accepted[case] = evidence.accept_summary(json.dumps(value).encode(), run_id, case, 0)
            except ValueError:
                pass
    cleanup = read(source / "cleanup.json") or {}
    processes = read(source / "processes.json") or {}
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
    parent_ok = (source_ok and len(accepted) == len(evidence.CASES) and report.get("verdict") == "PASS"
        and report.get("cleanup") == "PASS" and report.get("restoration") == "PASS"
        and report.get("completed_cases") == list(evidence.CASES)
        and cleanup.get("status") == "PASS" and type(cleanup.get("owned_running")) is int
        and cleanup["owned_running"] == 0 and cleanup.get("errors") == []
        and read(source / "correlation.json") == CORRELATION and browser_ok
        and all(isinstance(processes.get(case), dict) and type(processes[case].get("exit")) is int and processes[case]["exit"] == 0
                and processes[case].get("cleanup") is True for case in evidence.CASES))
    passed = sum(len(value["checks"]) for value in accepted.values())
    status = "passed" if parent_ok else "failed" if source_ok else "incomplete"
    run_url = f"https://github.com/{repository}/actions/runs/{job_run}"
    metric = dict(schema="aiforall.e2e-status/v1", status=status, passed=passed, expected=EXPECTED,
        unproven=EXPECTED - passed, source_sha=target_sha, harness_sha=harness_sha,
        job_run_id=int(job_run), job_attempt=int(attempt), run_url=run_url,
        generated_at=datetime.now(timezone.utc).isoformat())
    write(destination / "status.json", metric)
    write(destination / "badge.json", dict(schemaVersion=1, label="E2E", message=f"{passed}/{EXPECTED}" if parent_ok else f"{status} ({passed}/{EXPECTED})",
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
                value["statusDetails"] = dict(message="Named check not proven by a valid same-source phase summary")
            write(results / (identity + "-result.json"), value)
    identity = str(uuid.uuid5(uuid.NAMESPACE_URL, job_run + ":" + attempt + ":campaign-contract"))
    write(results / (identity + "-result.json"), dict(uuid=identity, historyId="aiforall-e2e-parent-contract",
        testCaseId=hashlib.sha256(b"aiforall-e2e-parent-contract").hexdigest(),
        fullName="E2E.parent-contract", name="Parent deltas, TLS restoration, sandbox and cleanup",
        status="passed" if parent_ok else "broken", stage="finished",
        labels=[dict(name="parentSuite", value="E2E"), dict(name="suite", value="Campaign contract")]))
    write(results / "executor.json", dict(name="GitHub Actions", type="github", buildName=f"E2E {job_run}/{attempt}",
        buildOrder=int(job_run), buildUrl=run_url))
    (results / "environment.properties").write_text(f"applications={target_sha}\nharness={harness_sha}\n", encoding="ascii")
    page = ("<!doctype html><html lang='fr'><meta charset='utf-8'><title>AIForAll E2E</title>"
        "<style>body{font:18px system-ui;max-width:850px;margin:4rem auto;padding:1rem}code{font-size:14px}</style>"
        f"<h1>AIForAll — E2E {html.escape(status)}</h1><p>{passed}/{EXPECTED} contrôles fonctionnels prouvés.</p>"
        "<p>Le contrat parent est évalué séparément : un 34/34 ne suffit pas si les deltas, le sandbox ou le nettoyage échouent.</p>"
        f"<p><a href='allure/'>Rapport Allure</a> · <a href='{run_url}'>Run GitHub</a> · <a href='status.json'>Métriques JSON</a></p>"
        f"<p>Applications : <code>{target_sha}</code><br>Harness : <code>{harness_sha}</code></p>"
        "<p>Rapport du dernier run publié, pas une mesure de couverture de code. Allure contient les34contrôles et un contrat parent distinct."
        " Les durées et l'historique de tests ne sont pas inventés.</p></html>")
    (destination / "index.html").write_text(page, encoding="utf-8")
    (destination / ".nojekyll").write_text("", encoding="ascii")
    if summary_path:
        with Path(summary_path).open("a", encoding="utf-8") as stream:
            stream.write(f"## E2E : {status} — {passed}/{EXPECTED} checks\n\nApplications `{target_sha}` ; harness `{harness_sha}`.\n\n"
                         f"[Run]({run_url}) · [Rapports publics](https://{repository.split('/')[0].lower()}.github.io/{repository.split('/')[1]}/)\n")
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
