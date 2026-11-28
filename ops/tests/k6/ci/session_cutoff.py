"""Conservative reset timing and source attribution for hosted qualification."""

from dataclasses import dataclass
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import base64
import http.client
import json
import re
import secrets
import ssl
import time
from urllib.parse import quote, urlencode

from binaries import Refused, require, strict_json
from late_check import PORT as LATE_PORT

AUTHORITY_COUNTER = "cluster.session_authority.upstream_rq_total"
DIRECT_PORT = 18446
EDGE_PORT = 18443
GATEWAY_COUNTS = re.compile(
    r"replica_decisions=(\d+) authority_decisions=(\d+) unavailable=(\d+)"
)
UUID = r"[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}"
SECOND = 1_000_000_000


@dataclass(frozen=True)
class Observation:
    started_ns: int
    completed_ns: int
    status: int


def observe(request):
    started = time.clock_gettime_ns(time.CLOCK_BOOTTIME)
    status = request()
    return Observation(started, time.clock_gettime_ns(time.CLOCK_BOOTTIME), status)


def cutoff(lower_bound_ns, observations):
    """Require a completed denial by the deadline and a fresh request after it."""
    deadline = lower_bound_ns + 5_000_000_000
    require(bool(observations), "E2E_RESET_NO_OBSERVATIONS")
    for item in observations:
        require(lower_bound_ns <= item.started_ns <= item.completed_ns
                and item.status in (200, 403, 503), "E2E_RESET_OBSERVATION_INVALID")
        require(item.status != 200 or item.started_ns < deadline, "E2E_RESET_LATE_ADMISSION")
    denied = [item for item in observations if item.status in (403, 503)]
    by_deadline = [item.completed_ns for item in denied if item.completed_ns <= deadline]
    require(bool(by_deadline), "E2E_RESET_DENIAL_BOUND_UNPROVEN")
    require(any(item.started_ns >= deadline for item in denied), "E2E_RESET_POST_BOUND_UNPROVEN")
    return (min(by_deadline) - lower_bound_ns + 999_999) // 1_000_000


def replica_only(before, after, authority_before, authority_after, expected):
    """A downstream status is insufficient without both independent witnesses."""
    require(expected > 0 and tuple(right - left for left, right in zip(before, after, strict=True))
            == (expected, 0, 0), "E2E_REPLICA_DECISION_SOURCE")
    require(authority_after == authority_before, "E2E_REPLICA_AUTHORITY_CALL")


def wait_for(condition, error, seconds):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        value = condition()
        if value:
            return value
        time.sleep(0.2)
    raise Refused(error)


class HostedRecovery:
    def __init__(self, campaign, replica):
        self.campaign = campaign
        self.replica = replica
        self.gateway_id = self.gateway_identity()
        pki = campaign.root / "private/pki"
        self.tls = ssl.create_default_context(cafile=str(pki / "ca.crt"))
        self.direct_tls = ssl.create_default_context(cafile=str(pki / "ca.crt"))
        self.direct_tls.load_cert_chain(pki / "iam-service.crt", pki / "iam-service.key")

    def request(self, method, path, *, payload=None, token=None, port=DIRECT_PORT, timeout=4):
        headers = {"Accept": "application/json"}
        if payload is not None:
            headers["Content-Type"] = "application/json"
        if token is not None:
            headers["Authorization"] = "Bearer " + token
        context = self.direct_tls if port == DIRECT_PORT else self.tls
        connection = http.client.HTTPSConnection("127.0.0.1", port, timeout=timeout, context=context)
        try:
            connection.request(method, "/iam" + path, None if payload is None else json.dumps(payload), headers)
            response = connection.getresponse()
            body = response.read(65537)
            require(len(body) <= 65536, "E2E_RESET_HTTP_SIZE")
            return response.status, body
        except (OSError, http.client.HTTPException):
            raise Refused("E2E_RESET_HTTP_TRANSPORT") from None
        finally:
            connection.close()

    def post(self, path, payload, expected):
        status, raw = self.request("POST", path, payload=payload)
        require(status == expected, "E2E_RESET_HTTP_STATUS_" + path.rsplit("/", 1)[-1].upper())
        value = strict_json(raw)
        require(isinstance(value, dict), "E2E_RESET_HTTP_JSON")
        return value

    def event_token(self, event, field):
        # Values interpolated here are constant event/field names or the canonical
        # fixture UUID validated before its first use. SQL stdout stays in memory.
        token = self.campaign.sql(
            f"SELECT payload_json->'data'->>'{field}' FROM public.rustycog_outbox_events "
            f"WHERE aggregate_id = '{self.subject}' AND event_type = '{event}' "
            "ORDER BY created_at DESC LIMIT 1;", self.replica.writer)
        require(re.fullmatch(r"[a-zA-Z0-9_-]{16,100}", token), "E2E_RESET_FIXTURE_EVENT")
        return token

    def make_account(self, phase):
        self.email = f"{phase}-{self.campaign.state['run_id']}@example.com"
        password = "A9!" + secrets.token_hex(20)
        registered = self.post("/api/auth/signup", {"email": self.email, "password": password}, 202)
        self.subject = registered["user"]["id"]
        require(isinstance(self.subject, str) and re.fullmatch(UUID, self.subject), "E2E_RESET_FIXTURE_SUBJECT")
        self.post("/api/auth/complete-registration",
                  {"registration_token": registered["registration_token"],
                   "username": phase + "_" + self.subject.split("-")[0]}, 200)
        token = self.event_token("user_signed_up", "verification_token")
        status, _ = self.request("GET", "/api/auth/verify?" + urlencode({"email": self.email, "token": token}))
        require(status == 200, "E2E_RESET_FIXTURE_VERIFY")
        access, refresh = self.login(password, "0")
        return password, access, refresh

    def login(self, password, epoch):
        value = self.post("/api/auth/login", {"email": self.email, "password": password}, 200)
        return self.session(value, epoch)

    def session(self, value, epoch):
        access, refresh = value["access_token"], value["refresh_token"]
        try:
            claims = access.split(".")[1]
            decoded = strict_json(base64.urlsafe_b64decode(claims + "=" * (-len(claims) % 4)))
            require(decoded["sub"] == self.subject and decoded["auth_epoch"] == epoch
                    and decoded["exp"] > time.time() + 60, "E2E_RESET_JWT_FIXTURE")
        except (AttributeError, IndexError, KeyError, TypeError, ValueError):
            raise Refused("E2E_RESET_JWT_FIXTURE") from None
        require(isinstance(refresh, str) and bool(refresh), "E2E_RESET_REFRESH_FIXTURE")
        return access, refresh

    def legacy(self, live):
        header, claims, _ = live.split(".")
        decoded = strict_json(base64.urlsafe_b64decode(claims + "=" * (-len(claims) % 4)))
        require(decoded.pop("auth_epoch", None) == "0", "E2E_RESET_LEGACY_FIXTURE")
        encoded = base64.urlsafe_b64encode(json.dumps(decoded).encode()).rstrip(b"=").decode()
        message = f"{header}.{encoded}"
        key = self.campaign.root / "private/pki/platform-signing.pem"
        signature = self.campaign.r.cli("openssl", ["dgst", "-sha256", "-sign", str(key)], stdin=message.encode())
        return message + "." + base64.urlsafe_b64encode(signature).rstrip(b"=").decode()

    def profile(self, token, port=EDGE_PORT):
        status, body = self.request("GET", "/api/me", token=token, port=port)
        if status == 200:
            require(strict_json(body).get("id") == self.subject, "E2E_RESET_WRONG_PRINCIPAL")
        return status

    def epoch(self, database):
        return self.campaign.sql(
            "SELECT auth_epoch FROM public.iam_session_liveness_epochs "
            f"WHERE subject = '{self.subject}';", database)

    def gateway_identity(self):
        pod = self.campaign.pod("ext-authz")
        status, = pod["status"]["containerStatuses"]
        require(status["ready"] is True, "E2E_RESET_GATEWAY_NOT_READY")
        return pod["metadata"]["uid"], status["containerID"], status["restartCount"], pod["metadata"]["name"]

    def totals(self):
        since = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")

        def sample():
            require(self.gateway_identity() == self.gateway_id, "E2E_RESET_GATEWAY_REPLACED")
            messages = self.campaign.r.k(["logs", self.gateway_id[-1], "--container=ext-authz",
                                          "--since-time=" + since, "--tail=30"]).decode()
            matches = GATEWAY_COUNTS.findall(re.sub(r"\x1b\[[0-9;]*m", "", messages))
            return tuple(map(int, matches[-1])) if matches else None

        return wait_for(sample, "E2E_RESET_GATEWAY_COUNTER_UNAVAILABLE", 20)

    def authority_calls(self):
        require(list(self.campaign.identity()) == self.campaign.state["edge_identity"], "E2E_ENVOY_REPLACED")
        status, raw = self.campaign.http("http://127.0.0.1:19901/stats?format=json&filter="
                                         + quote("^" + re.escape(AUTHORITY_COUNTER) + "$", safe=""))
        require(status == 200, "E2E_RESET_AUTHORITY_COUNTER")
        counters = strict_json(raw)["stats"]
        require(len(counters) == 1, "E2E_RESET_AUTHORITY_COUNTER")
        counter = counters[0]
        require(counter["name"] == AUTHORITY_COUNTER and type(counter["value"]) is int
                and counter["value"] >= 0, "E2E_RESET_AUTHORITY_COUNTER")
        return counter["value"]

    def pre_reset(self, access):
        path = "/api/auth/password/reset-request"
        known = self.post(path, {"email": self.email}, 200)
        unknown = self.post(path, {"email": f"unknown-{self.campaign.state['run_id']}@example.com"}, 200)
        require(known == unknown, "E2E_RESET_REQUEST_ENUMERATED_ACCOUNT")
        proof = self.event_token("password_reset_requested", "reset_token")
        require(self.epoch(self.replica.writer) == "0", "E2E_RESET_REQUEST_REVOKED")
        require(self.profile(access, DIRECT_PORT) == 200, "E2E_RESET_REQUEST_DIRECT_ADMISSION")
        wait_for(lambda: self.epoch(self.replica.subscriber) == "0", "E2E_RESET_INITIAL_COPY", 8)
        before, calls = self.totals(), self.authority_calls()
        require(self.profile(access) == 200, "E2E_RESET_REQUEST_LOCKED_ACCOUNT")
        replica_only(before, self.totals(), calls, self.authority_calls(), 1)
        return proof

    def confirm(self, proof, access, refresh):
        password = "B9!" + secrets.token_hex(20)
        lower = time.clock_gettime_ns(time.CLOCK_BOOTTIME)
        value = self.post("/api/auth/password/reset-confirm", {"token": proof, "new_password": password}, 200)
        require(value == {"message": "Password has been successfully reset"}, "E2E_RESET_ISSUED_SESSION")
        direct = observe(lambda: self.profile(access, DIRECT_PORT))
        require(direct.status == 401 and direct.completed_ns <= lower + 5 * SECOND, "E2E_RESET_DIRECT_CUTOFF")
        results = []
        while True:
            sample = observe(lambda: self.profile(access))
            results.append(sample)
            if sample.started_ns >= lower + 5 * SECOND:
                break
            time.sleep(0.2)
        denial_ms = cutoff(lower, results)
        require(self.epoch(self.replica.writer) == "1", "E2E_RESET_WRITER_NOT_COMMITTED")
        value = self.post("/api/token/refresh", {"refresh_token": refresh}, 401)
        require("access_token" not in value and "refresh_token" not in value, "E2E_RESET_RENEWED_OLD_SESSION")
        fresh, _ = self.login(password, "1")
        require(self.profile(fresh, DIRECT_PORT) == 200, "E2E_RESET_NEW_LOGIN_DIRECT")
        direct_ms = (direct.completed_ns - lower + 999_999) // 1_000_000
        return fresh, denial_ms, results, direct_ms

    def nominal(self):
        password, access, refresh = self.make_account("nominal")
        new_password = "C9!" + secrets.token_hex(20)
        status, _ = self.request("POST", "/api/auth/password/reset-authenticated", token=access,
                                 payload={"current_password": password, "new_password": new_password})
        require(status == 200 and self.epoch(self.replica.writer) == "0",
                "E2E_RESET_SIGNED_IN_CHANGE")
        require(self.profile(access, DIRECT_PORT) == 200, "E2E_RESET_SIGNED_IN_REVOKED_ACCESS")
        self.post("/api/token/refresh", {"refresh_token": refresh}, 401)
        access, refresh = self.login(new_password, "0")
        proof = self.pre_reset(access)
        rotated = self.post("/api/token/refresh", {"refresh_token": refresh}, 200)
        access, replacement = self.session(rotated, "0")
        replay = self.post("/api/token/refresh", {"refresh_token": refresh}, 401)
        require("access_token" not in replay and "refresh_token" not in replay, "E2E_RESET_REFRESH_REPLAY")
        legacy = self.legacy(access)
        before, calls = self.totals(), self.authority_calls()
        require(self.profile(access) == 200, "E2E_RESET_PRECOMMIT_ROTATION_NOT_LIVE")
        require(self.profile(legacy) == 200 and self.profile(legacy, DIRECT_PORT) == 200,
                "E2E_RESET_LEGACY_EPOCH_ZERO")
        fresh, denial_ms, results, direct_ms = self.confirm(proof, access, replacement)
        require(results[-1].status == 403 and self.epoch(self.replica.subscriber) == "1",
                "E2E_RESET_NOMINAL_REPLICA_VERDICT")
        require(self.profile(fresh) == 200, "E2E_RESET_NEW_LOGIN_DENIED")
        require(self.profile(legacy) == 403 and self.profile(legacy, DIRECT_PORT) == 401,
                "E2E_RESET_LEGACY_ADVANCED_EPOCH")
        replica_only(before, self.totals(), calls, self.authority_calls(), len(results) + 4)
        return {"first_denial_upper_bound_ms": denial_ms, "mesh_observations": len(results) + 4,
                "authority_calls": 0, "post_bound_status": results[-1].status,
                "direct_denial_upper_bound_ms": direct_ms, "prior_rotation_revoked": True,
                "legacy_revoked": True, "signed_in_change_preserved": True}, fresh

    def late_response(self, live):
        pod = self.campaign.pod("late-check")
        name = pod["metadata"]["name"]
        identity = pod["metadata"]["uid"]
        count_name = "cluster.iam.upstream_rq_total"
        before = self.campaign.snapshot()[count_name]
        sample = observe(lambda: self.profile(live, LATE_PORT))
        elapsed_ms = (sample.completed_ns - sample.started_ns + 999_999) // 1_000_000
        require(sample.status == 503 and 950 <= elapsed_ms <= 2000, "E2E_LATE_CHECK_TIMEOUT")

        def delayed_positive():
            require(self.campaign.pod("late-check")["metadata"]["uid"] == identity,
                    "E2E_LATE_CHECK_REPLACED")
            raw = self.campaign.r.k(["logs", name, "--container=late-check", "--tail=2"])
            return [strict_json(line) for line in raw.splitlines()]

        logs = wait_for(delayed_positive, "E2E_LATE_CHECK_POSITIVE_MISSING", 5)
        require(len(logs) == 1 and set(logs[0]) == {"upstream_status", "delayed_positive_attempt_ms"}
                and logs[0]["upstream_status"] == 200
                and type(logs[0]["delayed_positive_attempt_ms"]) is int
                and logs[0]["delayed_positive_attempt_ms"] >= 1250, "E2E_LATE_CHECK_POSITIVE_INVALID")
        require(self.campaign.snapshot()[count_name] == before, "E2E_LATE_CHECK_FORWARDED")
        require(self.profile(live) == 200, "E2E_LATE_CHECK_LIVE_CONTROL")
        return {"response_ms": elapsed_ms, "late_positive_delay_ms": logs[0]["delayed_positive_attempt_ms"],
                "downstream_calls": 0, "status": sample.status}

    def lock_held(self, table, mode, database):
        return self.campaign.sql(
            "SELECT EXISTS (SELECT 1 FROM pg_catalog.pg_locks "
            f"WHERE relation = 'public.{table}'::regclass "
            "AND database = (SELECT oid FROM pg_database WHERE datname = current_database()) "
            f"AND mode = '{mode}' AND granted);", database) == "t"

    def apply_paused(self):
        return (self.lock_held("iam_session_liveness_checkpoints", "ShareLock", self.replica.subscriber)
                and self.campaign.sql("SELECT EXISTS (SELECT 1 FROM public.iam_session_liveness_ready);",
                                      self.replica.subscriber) == "t")

    def authority_unavailable(self, live, worker):
        authority_before = self.authority_calls()
        count_name = "cluster.iam.upstream_rq_total"
        downstream = self.campaign.snapshot()[count_name]
        hold = worker.submit(self.campaign.sql, "BEGIN; SET LOCAL lock_timeout='3s'; "
                             "LOCK TABLE public.users IN ACCESS EXCLUSIVE MODE; "
                             "SELECT pg_sleep(5); ROLLBACK;", self.replica.writer, record_stage=False)
        wait_for(lambda: self.lock_held("users", "AccessExclusiveLock", self.replica.writer),
                 "E2E_RESET_AUTHORITY_OUTAGE_UNPROVEN", 4)
        require(self.apply_paused() and self.profile(live) == 503, "E2E_RESET_AUTHORITY_OUTAGE_STATUS")
        require(self.campaign.snapshot()[count_name] == downstream, "E2E_RESET_AUTHORITY_OUTAGE_FORWARDED")
        require(self.authority_calls() - authority_before == 1, "E2E_RESET_AUTHORITY_OUTAGE_CALL")
        hold.result()

    def degraded(self):
        _, access, refresh = self.make_account("degraded")
        proof = self.pre_reset(access)
        before, calls = self.totals(), self.authority_calls()
        # SHARE excludes the logical apply worker's write lock while preserving
        # SELECT and readiness. The fixture transaction ends even on test failure.
        with ThreadPoolExecutor(max_workers=2) as worker:
            hold = worker.submit(self.campaign.sql, "BEGIN; SET LOCAL lock_timeout='3s'; "
                                 "LOCK TABLE public.iam_session_liveness_checkpoints IN SHARE MODE; "
                                 "SELECT pg_sleep(24); ROLLBACK;", self.replica.subscriber, record_stage=False)
            wait_for(self.apply_paused, "E2E_RESET_APPLY_PAUSE_MISSING", 5)
            fresh, denial_ms, results, direct_ms = self.confirm(proof, access, refresh)
            require(self.epoch(self.replica.subscriber) == "0" and self.apply_paused(),
                    "E2E_RESET_STALE_REPLICA_UNPROVEN")
            downstream = self.campaign.snapshot().get("cluster.iam.upstream_rq_total", 0)
            status = self.profile(access)
            require(status in (403, 503)
                    and self.campaign.snapshot().get("cluster.iam.upstream_rq_total", 0) == downstream,
                    "E2E_RESET_DEGRADED_FORWARDED")
            self.authority_unavailable(fresh, worker)
            hold.result()
        after, final_calls = self.totals(), self.authority_calls()
        delta = tuple(right - left for left, right in zip(before, after, strict=True))
        require(min(delta) >= 0 and sum(delta) == len(results) + 2 and delta[1] > 0 and delta[2] > 0,
                "E2E_RESET_DEGRADED_DECISION_SOURCE")
        fallback_calls = final_calls - calls
        require(2 <= fallback_calls <= len(results) + 2, "E2E_RESET_DEGRADED_AUTHORITY_COUNT")
        wait_for(lambda: self.epoch(self.replica.subscriber) == "1", "E2E_RESET_CATCHUP_MISSING", 12)
        before, calls = self.totals(), self.authority_calls()
        require(self.profile(access) == 403 and self.profile(fresh) == 200, "E2E_RESET_CATCHUP_ADMISSION")
        replica_only(before, self.totals(), calls, self.authority_calls(), 2)
        return {"first_denial_upper_bound_ms": denial_ms, "post_bound_status": results[-1].status,
                "authority_calls": fallback_calls, "catchup": True,
                "direct_denial_upper_bound_ms": direct_ms, "authority_outage_status": 503}

    def ordered_race(self, rotation_first):
        phase = "renew_first" if rotation_first else "reset_first"
        _, old_access, old_refresh = self.make_account(phase)
        self.post("/api/auth/password/reset-request", {"email": self.email}, 200)
        proof = self.event_token("password_reset_requested", "reset_token")
        password = "D9!" + secrets.token_hex(20)
        # The real HTTP writers wait on the same user row. Observe the first
        # waiter before submitting the second, and both before the lock ends.
        label = "k6_" + self.campaign.state["run_id"] + "_" + phase
        query = f"""
            WITH RECURSIVE blocked(pid) AS (
                SELECT pid FROM pg_stat_activity
                WHERE application_name = '{label}' AND wait_event = 'PgSleep'
                UNION
                SELECT a.pid FROM pg_stat_activity a
                JOIN blocked b ON b.pid = ANY(pg_blocking_pids(a.pid))
                WHERE a.datname = current_database() AND a.query LIKE '%FOR UPDATE%'
            )
            SELECT count(*) FROM blocked;
        """

        def wait_count(count):
            return wait_for(lambda: self.campaign.sql(query, self.replica.writer) == str(count),
                            "E2E_RESET_HTTP_WRITER_ORDER_UNPROVEN", 3)

        requests = {
            "reset": ("/api/auth/password/reset-confirm", {"token": proof, "new_password": password}),
            "renew": ("/api/token/refresh", {"refresh_token": old_refresh}),
        }
        order = ("renew", "reset") if rotation_first else ("reset", "renew")
        with ThreadPoolExecutor(max_workers=3) as worker:
            hold = worker.submit(self.campaign.sql, f"SET application_name = '{label}'; "
                                 "BEGIN; SET LOCAL lock_timeout='3s'; "
                                 f"SELECT id FROM public.users WHERE id = '{self.subject}' FOR UPDATE; "
                                 "SELECT pg_sleep(9); ROLLBACK;", self.replica.writer, record_stage=False)
            wait_count(1)
            pending = {}
            for count, name in enumerate(order, start=2):
                route, payload = requests[name]
                pending[name] = worker.submit(self.request, "POST", route, payload=payload, timeout=13)
                wait_count(count)
            hold.result()
            reset_status, reset_body = pending["reset"].result()
            renew_status, renew_body = pending["renew"].result()
        require(reset_status == 200 and strict_json(reset_body) == {"message": "Password has been successfully reset"}
                and self.epoch(self.replica.writer) == "1", "E2E_RESET_HTTP_WRITER_CONFIRM")
        value = strict_json(renew_body)
        if rotation_first:
            require(renew_status == 200, "E2E_RESET_HTTP_WRITER_ROTATION_FIRST")
            old_access, old_refresh = self.session(value, "0")
            self.post("/api/token/refresh", {"refresh_token": old_refresh}, 401)
        else:
            require(renew_status == 401 and "access_token" not in value and "refresh_token" not in value,
                    "E2E_RESET_HTTP_WRITER_RESET_FIRST")
        require(self.profile(old_access, DIRECT_PORT) == 401, "E2E_RESET_HTTP_WRITER_STALE_ACCESS")
        fresh, _ = self.login(password, "1")
        require(self.profile(fresh, DIRECT_PORT) == 200, "E2E_RESET_HTTP_WRITER_FRESH_ACCESS")
        return {"commit_status": reset_status, "renewal_status": renew_status, "writer_order_observed": True}
