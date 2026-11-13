"""Closed receiver for the existing three-scenario campaign (no deployment)."""
import json
import re


class EvidenceError(ValueError):
    """Public bounded failure category; never retain raw child output."""


OAUTH_CHECKS = frozenset((
    "browser-version", "registration-callback", "registration-required",
    "registration-nonce", "registration-completed", "login-callback",
    "login-identity", "login-authority", "login-nonce", "relink-start",
    "relink-callback", "relink-identity", "relink-nonce", "negative-link",
    "negative-relink-start", "negative-relink-alias", "oauth-observations",
    "oauth-iam-cardinality", "login-redirects", "oauth-cookie-attributes",
    "oauth-callback-headers", "registration-request-headers",
    "relink-start-authorization", "provider-authorizations",
    "provider-no-credentials", "negative-headers",
))
CASES = {
    "iam-health": ("HTTP", frozenset(("iam-status", "iam-body")), 1),
    "auth-envoy-san-good": ("HTTP", frozenset(("san-status", "san-no-redirect")), 1),
    "auth-envoy-san-bad": ("HTTP", frozenset(("san-status", "san-no-redirect")), 1),
    "auth-envoy-san-restored": ("HTTP", frozenset(("san-status", "san-no-redirect")), 1),
    "auth-oauth-browser": ("Chromium", OAUTH_CHECKS, 0),
}
SUMMARY_FIELDS = frozenset((
    "version", "run_id", "phase", "cases", "expected", "started", "completed",
    "iterations", "requests", "checks",
))


def strict_json(raw):
    if not isinstance(raw, bytes) or len(raw) > 65536:
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


def accept_summary(raw, run_id, case, exit_code):
    if not isinstance(run_id, str) or not re.fullmatch(r"[a-f0-9]{32}", run_id):
        raise EvidenceError("RUN_ID")
    if case not in CASES or type(exit_code) is not int or exit_code != 0:
        raise EvidenceError("CHILD_FAILED")
    value = strict_json(raw)
    phase, checks, requests = CASES[case]
    if not isinstance(value, dict) or value.keys() != SUMMARY_FIELDS:
        raise EvidenceError("SUMMARY_SCHEMA")
    if (type(value["version"]) is not int or value["version"] != 1
            or value["run_id"] != run_id or value["phase"] != phase
            or value["cases"] != [case]):
        raise EvidenceError("SUMMARY_IDENTITY")
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
    if not isinstance(results, dict) or results.keys() != CASES.keys():
        raise EvidenceError("CAMPAIGN_PHASES")
    summaries = {}
    for case, result in results.items():
        if not isinstance(result, tuple) or len(result) != 2:
            raise EvidenceError("CAMPAIGN_NATIVE_RESULT")
        raw, exit_code = result
        summaries[case] = accept_summary(raw, run_id, case, exit_code)
    return summaries
