"""Bounded scenario registration and closed campaign evidence receiver."""
import hashlib
import json
from pathlib import Path
import re


class EvidenceError(ValueError):
    """Public bounded failure category; never retain raw child output."""


SUMMARY_FIELDS = frozenset((
    "version", "run_id", "phase", "cases", "expected", "started", "completed",
    "iterations", "requests", "checks",
))
SUMMARY_LIMIT = 65536


def strict_json(raw):
    if not isinstance(raw, bytes) or len(raw) > SUMMARY_LIMIT:
        raise EvidenceError("SUMMARY_SIZE")

    def pairs(items):
        result = {}
        for name, value in items:
            if name in result:
                raise EvidenceError("SUMMARY_DUPLICATE")
            result[name] = value
        return result

    def constant(_):
        raise EvidenceError("SUMMARY_NONFINITE")

    try:
        return json.loads(raw.decode("utf-8"), object_pairs_hook=pairs,
                          parse_constant=constant)
    except (ValueError, UnicodeError, RecursionError):
        raise EvidenceError("SUMMARY_JSON") from None


def integer(value):
    return type(value) is int and value >= 0


def load_catalog(path):
    """Return ordered records; never execute discovery code or follow symlinks."""
    path = Path(path)
    try:
        if path.is_symlink() or any(parent.is_symlink() for parent in path.parents):
            raise EvidenceError("CATALOG_PATH")
        with path.open("rb") as stream:
            document = strict_json(stream.read(SUMMARY_LIMIT + 1))
        if (not isinstance(document, dict) or document.keys() != {"version", "cases"}
                or type(document["version"]) is not int or document["version"] != 1
                or not isinstance(document["cases"], list) or not 1 <= len(document["cases"]) <= 128):
            raise EvidenceError("CATALOG_SCHEMA")
        result = {}
        special = {"auth-envoy-san-" + phase: "san" for phase in ("good", "bad", "restored")}
        special["auth-oauth-browser"] = "oauth"
        for record in document["cases"]:
            if (not isinstance(record, dict) or record.keys() != {
                    "id", "script", "phase", "lifecycle", "timeout_seconds", "requests", "checks"}):
                raise EvidenceError("CATALOG_CASE")
            case = record["id"]
            if (not isinstance(case, str) or not re.fullmatch(r"[a-z0-9][a-z0-9-]{0,63}", case)
                    or case in result):
                raise EvidenceError("CATALOG_ID")
            lifecycle = special.get(case, "standard")
            if (record["lifecycle"] != lifecycle
                    or record["phase"] != ("Chromium" if lifecycle == "oauth" else "HTTP")
                    or not integer(record["timeout_seconds"]) or not 1 <= record["timeout_seconds"] <= 600
                    or not integer(record["requests"]) or record["requests"] > 65535):
                raise EvidenceError("CATALOG_EXECUTION")
            checks = record["checks"]
            if (not isinstance(checks, dict) or not 1 <= len(checks) <= 128
                    or any(not isinstance(name, str) or not re.fullmatch(r"[a-z0-9][a-z0-9-]{0,79}", name)
                           or kind not in ("functional", "cleanup") for name, kind in checks.items())):
                raise EvidenceError("CATALOG_CHECKS")
            script = record["script"]
            if (not isinstance(script, str) or len(script) > 200
                    or not re.fullmatch(r"(?:[a-z0-9][a-z0-9-]*/)*[a-z0-9][a-z0-9-]*\.js", script)):
                raise EvidenceError("CATALOG_SCRIPT")
            if ((lifecycle == "san" and script != "auth/envoy-san.js")
                    or (lifecycle == "oauth" and script != "auth/oauth-browser.js")):
                raise EvidenceError("CATALOG_SPECIAL_SCRIPT")
            target = path.parent / script
            if (not target.is_file() or target.is_symlink()
                    or any(parent.is_symlink() for parent in target.parents)
                    or not target.resolve().is_relative_to(path.parent.resolve())):
                raise EvidenceError("CATALOG_SCRIPT")
            result[case] = record
        ids = list(result)
        san = list(special)[:3]
        if (not all(case in result for case in special)
                or ids[ids.index(san[0]):ids.index(san[0]) + 3] != san):
            raise EvidenceError("CATALOG_SPECIAL_LIFECYCLE")
        return result
    except (OSError, ValueError, TypeError, RecursionError):
        raise EvidenceError("CATALOG_INVALID") from None


CATALOG_PATH = Path(__file__).resolve().parents[1] / "scenarios/catalog.json"
CATALOG = load_catalog(CATALOG_PATH)
CATALOG_SHA256 = hashlib.sha256(CATALOG_PATH.read_bytes()).hexdigest()
CASES = {case: (record["phase"], frozenset(record["checks"]), record["requests"])
         for case, record in CATALOG.items()}
OAUTH_CHECKS = CASES["auth-oauth-browser"][1]


def selection_receipt(run_id, harness_sha, target_sha):
    if (not isinstance(run_id, str) or not re.fullmatch(r"[a-f0-9]{32}", run_id)
            or any(not isinstance(value, str) or not re.fullmatch(r"[a-f0-9]{40}", value)
                   for value in (harness_sha, target_sha))):
        raise EvidenceError("SELECTION_IDENTITY")
    return dict(run_id=run_id, catalog_sha256=CATALOG_SHA256, cases=list(CATALOG),
                harness_sha=harness_sha, target_sha=target_sha)


def summary_document(value, run_id, case):
    if not isinstance(value, dict) or value.keys() != SUMMARY_FIELDS:
        raise EvidenceError("SUMMARY_SCHEMA")
    if (type(value["version"]) is not int or value["version"] != 1
            or value["run_id"] != run_id or value["phase"] != CASES[case][0]
            or value["cases"] != [case]):
        raise EvidenceError("SUMMARY_IDENTITY")
    return value


def inspect_failed_summary(value, run_id, case):
    """Validate partial counts for diagnostics, never for campaign admission."""
    if not isinstance(run_id, str) or not re.fullmatch(r"[a-f0-9]{32}", run_id) or case not in CASES:
        raise EvidenceError("SUMMARY_IDENTITY")
    value = summary_document(value, run_id, case)

    def count(number):
        return integer(number) and number <= 65535

    if type(value["expected"]) is not int or value["expected"] != 1 or any(
            value[key] is not None and not count(value[key])
            for key in ("started", "completed", "iterations", "requests")):
        raise EvidenceError("SUMMARY_CARDINALITY")
    if not isinstance(value["checks"], dict) or not value["checks"].keys() <= CASES[case][1]:
        raise EvidenceError("SUMMARY_CHECKS")
    for result in value["checks"].values():
        if (not isinstance(result, dict) or result.keys() != {"passes", "fails"}
                or not all(count(number) for number in result.values())):
            raise EvidenceError("SUMMARY_CHECK_FAILED")
    return value


def accept_summary(raw, run_id, case, exit_code):
    if not isinstance(run_id, str) or not re.fullmatch(r"[a-f0-9]{32}", run_id):
        raise EvidenceError("RUN_ID")
    if case not in CASES or type(exit_code) is not int or exit_code != 0:
        raise EvidenceError("CHILD_FAILED")
    value = summary_document(strict_json(raw), run_id, case)
    _, checks, requests = CASES[case]
    for field in ("expected", "started", "completed", "iterations", "requests"):
        if not integer(value[field]) or value[field] != (requests if field == "requests" else 1):
            raise EvidenceError("SUMMARY_CARDINALITY")
    if not isinstance(value["checks"], dict) or value["checks"].keys() != checks:
        raise EvidenceError("SUMMARY_CHECKS")
    for result in value["checks"].values():
        if (not isinstance(result, dict) or result.keys() != {"passes", "fails"}
                or not integer(result["passes"]) or not integer(result["fails"])
                or result != {"passes": 1, "fails": 0}):
            raise EvidenceError("SUMMARY_CHECK_FAILED")
    return value


def delta(before, after, name):
    if name not in before or name not in after:
        raise EvidenceError("COUNTER_MISSING")
    if not integer(before[name]) or not integer(after[name]) or after[name] < before[name]:
        raise EvidenceError("COUNTER_RESET")
    return after[name] - before[name]


def accept_san(good, bad, restored, incarnations):
    if (len(incarnations) != 3 or not incarnations[0]
            or len(set(incarnations)) != 1):
        raise EvidenceError("ENVOY_INCARNATION_CHANGED")
    for end in (bad, restored):
        if (delta(good, end, "cluster.iam.ssl.fail_verify_san") != 1
                or delta(good, end, "cluster.iam.ssl.fail_verify_error") != 0):
            raise EvidenceError("SAN_CAUSALITY")
    if delta(bad, restored, "cluster.iam.ssl.handshake") < 1:
        raise EvidenceError("SAN_RESTORATION")


def accept_oauth(before, after, provider_before, provider_after, db_before, db_after, run_id):
    if (delta(before, after, "cluster.iam.upstream_rq_total") != 7
            or delta(before, after, "cluster.ext_authz.upstream_rq_total") != 4
            or delta(before, after, "cluster.iam.upstream_rq_5xx") != 0):
        raise EvidenceError("OAUTH_EDGE_CARDINALITY")
    for cluster in ("iam", "ext_authz"):
        for counter in ("upstream_cx_connect_fail", "upstream_cx_connect_timeout",
                        "ssl.fail_verify_san", "ssl.fail_verify_error"):
            if delta(before, after, f"cluster.{cluster}.{counter}") != 0:
                raise EvidenceError("OAUTH_TRANSPORT")
    if provider_before.get("run_id") != run_id or provider_after.get("run_id") != run_id:
        raise EvidenceError("PROVIDER_RUN_ID")
    for name in ("authorize", "token", "profile", "pkce_validated"):
        if delta(provider_before, provider_after, name) != 3:
            raise EvidenceError("PROVIDER_CARDINALITY")
    if delta(db_before, db_after, "users") != 1 or delta(db_before, db_after, "outbox") != 0:
        raise EvidenceError("DATABASE_EFFECTS")


def accept_campaign(results, run_id):
    """Each phase supplies its own raw summary and actual native exit status."""
    if not isinstance(results, dict) or list(results) != list(CASES):
        raise EvidenceError("CAMPAIGN_PHASES")
    summaries = {}
    for case, result in results.items():
        if not isinstance(result, tuple) or len(result) != 2:
            raise EvidenceError("CAMPAIGN_NATIVE_RESULT")
        raw, exit_code = result
        summaries[case] = accept_summary(raw, run_id, case, exit_code)
    return summaries
