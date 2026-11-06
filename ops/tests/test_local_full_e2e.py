"""Pure protocol/PG/provenance doubles: no sockets, subprocess, runtime or Cargo."""
import base64
import argparse
import copy
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
from types import SimpleNamespace
import tempfile
import sys
import traceback
import unittest
from unittest.mock import Mock, patch
from urllib.parse import parse_qs, urlsplit
import uuid

SPEC = importlib.util.spec_from_file_location("local_full_e2e_unit", Path(__file__).with_name("local_full_e2e.py"))
e2e = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(e2e)
BAO_SPEC = importlib.util.spec_from_file_location(
    "local_full_openbao_e2e_unit", Path(__file__).resolve().parents[1] / "deploy/local_full_openbao.py")
bao = importlib.util.module_from_spec(BAO_SPEC)
BAO_SPEC.loader.exec_module(bao)
RUN = "20661d5f-522a-4dac-bb93-1fa79449f063"
USER = "ed6ae7de-7535-4b0f-a007-a94ab4647273"
ORG = "d2d8b1f7-0b7d-46c4-8e35-0e6e2d415f77"


class MemoryReceipt:
    def __init__(self):
        self.events = []

    def append(self, kind, data):
        self.events.append({"kind": kind, "data": copy.deepcopy(data)})

    def last(self, kind):
        return next((copy.deepcopy(e["data"]) for e in reversed(self.events) if e["kind"] == kind), None)


class FakePg:
    def __init__(self):
        self.calls = []
        self.account = False
        self.provider = False
        self.refresh = {}
        self.safe = True
        self.changed_provider = False
        self.insert_ack_lost = False
        self.insert_ack_missing_sha = False
        self.provider_row = {"id": 42, "user_id": USER, "provider": "github", "provider_user_id": RUN,
                             "access_token": "simulated-provider-" + RUN, "refresh_token": None, "expires_in": 3600,
                             "created_at": "2026-11-02T12:00:00", "updated_at": "2026-11-02T12:00:00"}
        self.before_provider_delete = None

    def provider_sha(self):
        return hashlib.sha256(json.dumps(self.provider_row, sort_keys=True).encode()).hexdigest()

    def qualify(self, values):
        self.calls.append("qualify")
        if not self.safe:
            raise e2e.Blocked("unqualified graph")

    qualify_account_cleanup = qualify

    def qualify_notifications_cleanup(self, values):
        # Synthetic prerequisite ONLY for isolated protocol branch tests below.
        # The production Database rejects this qualification; dedicated tests
        # exercise that source gate and require zero signup/SQL mutation.
        return None

    def snapshot(self):
        result = {table: [] for table in e2e.ACCOUNT_TABLES}
        if self.account:
            result["users"] = [{"id": USER, "sha": "user-sha"}]
            result["user_emails"] = [{"id": "email-pk", "sha": "email-sha"}]
            result["identities"] = [{"id": "identity-pk", "sha": "identity-sha"}]
            result["refresh_tokens"] = [{"id": pk, "sha": "row-sha"} for pk in sorted(self.refresh.values())]
            if self.provider:
                result["provider_tokens"] = [{"id": 42, "sha": self.provider_sha()}]
        return result

    def execute(self, operation, values):
        self.calls.append(operation)
        if operation == "absence":
            return {"absent": not self.account}
        if operation == "snapshot":
            return self.snapshot()
        if operation == "account-owned":
            return {"owned": self.account}
        if operation == "refresh-ids":
            return sorted(pk for token, pk in self.refresh.items() if token in values["hashes"])
        if operation == "refresh":
            return {"count": int(values["hash"] in self.refresh)}
        if operation == "provider-insert":
            if self.provider:
                return None
            self.provider = True
            if self.insert_ack_lost:
                raise e2e.Blocked("lost insert ACK")
            if self.insert_ack_missing_sha:
                return {"id": 42}
            return {"id": 42, "sha": self.provider_sha()}
        if operation == "provider-read":
            return {"count": int(self.provider), "matches": int(self.provider and not self.changed_provider and values["sha"] == self.provider_sha())}
        if operation == "provider-state":
            return {"exists": self.provider, "matches": self.provider and not self.changed_provider and values["sha"] == self.provider_sha()}
        if operation == "provider-delete":
            if self.before_provider_delete:
                self.before_provider_delete(self.provider_row)
            if not self.provider or self.changed_provider or values["sha"] != self.provider_sha():
                return None  # SQL predicate rejects the raced/changed complete row.
            self.provider = False
            return {"id": 42, "sha": self.provider_sha()}
        raise AssertionError(operation)

    def cleanup_account(self, values, expected):
        if expected != self.snapshot():
            raise e2e.Blocked("snapshot changed")
        self.account = False
        self.calls.append("account-deleted")


class FakeHttp:
    """Endpoint behaviours, not a network server or authentication substitute."""
    def __init__(self, pg):
        self.pg, self.calls = pg, []
        self.driver = SimpleNamespace(runtime=SimpleNamespace(target=lambda: SimpleNamespace(e2e_forward=lambda *a, **k: None)))
        self.wrong_sub = False
        self.forward_missing = False
        self.message_deleted = False
        self.providers = ["github", "gitlab"]

    def iam_providers(self):
        return {"providers": list(self.providers), "config_sha256": "a" * 64, "pod_uid": "iam-pod"}

    def pki(self, identity):
        if identity not in e2e.IDENTITIES:
            raise AssertionError(identity)

    def request(self, method, path, body=None, headers=None, *, service=None, identity="mesh-client", case=None):
        headers = headers or {}
        self.calls.append((method, path, service, identity))
        if self.forward_missing and service:
            raise e2e.Blocked("missing guarded forward")
        if case is not None:
            case.update(attempted=True, observed=True)
        status, result = 200, {}
        if path.startswith("/api/v2/search"):
            email = parse_qs(urlsplit(path).query)["query"][0]
            message = {"ID": "owned-message", "To": [{"Mailbox": email.split("@")[0], "Domain": "example.com"}],
                       "Content": {"Body": "https://foreign.invalid/verify?email=" + email + "&token=owned-token"}}
            result = {"total": int(self.pg.account), "items": [message] if self.pg.account else []}
        elif path == "/api/v1/messages/owned-message":
            if method == "DELETE":
                self.message_deleted = True
            else:
                result = {"ID": "owned-message", "To": [{"Mailbox": "local-full-" + RUN, "Domain": "example.com"}]}
        elif path == "/iam/api/auth/signup":
            self.pg.account = True
            status, result = 202, {"user": {"id": USER, "email": body["email"]}, "registration_token": "registration"}
        elif path == "/iam/api/auth/complete-registration":
            result = {"user": {"id": USER}, "access_token": "", "refresh_token": ""}
        elif path.startswith("/iam/api/auth/verify?"):
            pass
        elif path == "/iam/api/auth/login":
            encode = lambda value: base64.urlsafe_b64encode(json.dumps(value).encode()).decode().rstrip("=")
            token = encode({"kid": "issued-kid"}) + "." + encode({"sub": USER, "iss": e2e.ISSUER}) + ".signature-fixture"
            self.pg.refresh[hashlib.sha256(b"login-refresh").hexdigest()] = "refresh-pk-1"
            result = {"user": {"id": USER}, "access_token": token, "refresh_token": "login-refresh"}
        elif path == "/iam/.well-known/jwks.json":
            result = {"keys": [{"kid": "issued-kid", "n": "public-modulus"}]}
        elif path == "/iam/api/me":
            if "authorization" in headers:
                result = {"id": "spoofed" if self.wrong_sub else USER}
            else:
                status = 401
        elif path == "/iam/api/token/refresh":
            self.pg.refresh[hashlib.sha256(b"replacement-refresh").hexdigest()] = "refresh-pk-2"
            result = {"refresh_token": "replacement-refresh"}
        elif path == "/telegraph/api/notifications":
            if service:
                status = 401
            else:
                result = {"user_id": USER}
        elif path == "/hive/api/organizations/search?query=mesh":
            status = 401
        elif path.startswith("/iam/internal/github/"):
            if identity != "envoy-mesh" or "x-principal-sub" not in headers:
                status = 401
            elif headers.get("x-iam-internal-token") != "internal-token-fixture":
                status = 403
            elif path.endswith("/revoke"):
                self.pg.provider = False
            elif self.pg.provider:
                result = {"access_token": "simulated-provider-" + RUN}
            else:
                status = 404
        elif path == "/hive/api/organizations":
            result = {"id": ORG}
        elif path.endswith("/members/" + USER):
            result = {"issuer": e2e.ISSUER, "user_id": USER}
        elif path == "/hive/api/organizations/" + ORG and method == "DELETE":
            status = 204
        elif path == "/hive/api/organizations/" + ORG and method == "GET":
            result = {"id": ORG, "slug": "lf-" + RUN}
        else:
            raise AssertionError((method, path))
        return status, json.dumps(result).encode()


def mesh_fixture():
    pg, receipt = FakePg(), MemoryReceipt()
    http, results = FakeHttp(pg), e2e.Results("mesh-authn")
    inputs = {"run_id": RUN, "suite": "mesh-authn"}
    mesh = e2e.Mesh(http, pg, receipt, {"password": "private-password-fixture", "internal_token": "internal-token-fixture"}, inputs, results)
    return mesh, results, pg, http, receipt


class ProtocolTests(unittest.TestCase):
    def test_full_inventories_and_polarities(self):
        mesh = e2e.mesh_cases()
        self.assertEqual((len(mesh), sum(p for _, p in mesh), len(set(n for n, _ in mesh))), (39, 12, 39))
        self.assertEqual((len(e2e.APPARATUS_CASES), sum(p for _, p in e2e.APPARATUS_CASES)), (6, 4))

    def test_mesh39_and_exact_cleanup_with_pure_doubles(self):
        mesh, results, pg, http, receipt = mesh_fixture()
        mesh.run()
        report = results.report()
        self.assertEqual((report["passed"], report["failed"], report["blocked"]), (39, 0, 0), report)
        self.assertEqual(e2e.cleanup(mesh), {"status": "PASS", "residuals": [], "removed": 3})
        self.assertFalse(pg.account)
        self.assertFalse(pg.provider)
        self.assertTrue(http.message_deleted)
        # Email link is data, never a destination. No forged principal on public requests.
        self.assertTrue(any(path.startswith("/iam/api/auth/verify?") for _, path, _, _ in http.calls))
        self.assertFalse(any("foreign.invalid" in path for _, path, _, _ in http.calls))
        encoded = json.dumps(receipt.events)
        for secret in ("private-password-fixture", "internal-token-fixture", "replacement-refresh", "owned-token", "registration"):
            self.assertNotIn(secret, encoded)

    def test_cleanup_qualification_before_signup(self):
        mesh, results, pg, http, _ = mesh_fixture()
        pg.safe = False
        results.case("signup", mesh.signup)
        self.assertEqual(results.cases["signup"]["status"], "BLOCKED")
        self.assertEqual(http.calls, [])

    def test_terminal_receipt_refused_before_direct_signup(self):
        mesh, _, _, http, receipt = mesh_fixture()
        receipt.append("cleanup", {"complete": True})
        with patch.object(http, "iam_providers", wraps=http.iam_providers) as registry:
            with self.assertRaisesRegex(e2e.Blocked, "terminal"):
                mesh.signup({})
        registry.assert_not_called()
        self.assertEqual(http.calls, [])
        self.assertIsNone(receipt.last("signup-intent"))

    def test_old_creation_after_terminal_is_not_cleanup_pass(self):
        mesh, _, pg, http, receipt = mesh_fixture()
        receipt.append("cleanup", {"complete": True})
        receipt.append("signup-intent", {"run": RUN})
        self.assertEqual(e2e.cleanup(mesh), {"status": "BLOCKED", "residuals": ["creation-after-terminal-cleanup"], "removed": 0})
        self.assertEqual(http.calls, [])
        self.assertEqual(pg.calls, [])
        receipt.append("cleanup", {"complete": False})
        self.assertEqual(e2e.cleanup(mesh)["status"], "BLOCKED")

    def test_effective_gitlab_only_blocks_before_signup_and_provider_insert(self):
        mesh, results, pg, http, receipt = mesh_fixture()
        http.providers = ["gitlab"]
        mesh.run()
        self.assertEqual(results.cases["signup"]["status"], "BLOCKED")
        self.assertIn("github absent", results.cases["signup"]["reason"])
        self.assertEqual(len(results.report()["cases"]), 39)
        self.assertFalse(any(method == "POST" for method, _, _, _ in http.calls))
        self.assertNotIn("provider-insert", pg.calls)
        self.assertIsNone(receipt.last("signup-intent"))

    def test_real_telegraph_inverse_gap_blocks_before_signup_without_pg_exec(self):
        mesh, results, _, http, receipt = mesh_fixture()
        driver = Mock()
        mesh.db = e2e.Database(driver, {})
        results.case("signup", mesh.signup)
        self.assertEqual(results.cases["signup"]["status"], "BLOCKED")
        self.assertIn("notification/delivery inverse unqualified", results.cases["signup"]["reason"])
        driver.runtime.kubectl.assert_not_called()
        self.assertEqual(http.calls, [])
        self.assertIsNone(receipt.last("signup-intent"))

    def test_unqualified_historical_notifications_never_report_cleanup_pass(self):
        mesh, _, _, http, receipt = mesh_fixture()
        driver = Mock()
        mesh.db = e2e.Database(driver, {})
        receipt.append("account-created", {**mesh.values, "user": USER})
        self.assertEqual(e2e.cleanup(mesh), {"status": "BLOCKED", "residuals": ["telegraph-notifications-unqualified"], "removed": 0})
        self.assertFalse(receipt.last("cleanup")["complete"])
        self.assertEqual(http.calls, [])
        driver.runtime.kubectl.assert_not_called()

    def test_missing_mail_channel_before_signup_not_global_guard(self):
        mesh, results, _, http, _ = mesh_fixture()
        http.forward_missing = True
        mesh.run()
        self.assertNotIn("/iam/api/auth/signup", [p for _, p, _, _ in http.calls])
        self.assertEqual(results.cases["signup"]["status"], "BLOCKED")
        self.assertEqual(results.cases["me-no-jwt"]["status"], "PASS")
        self.assertEqual(len(results.report()["cases"]), 39)

    def test_real_sub_body_is_required_not_just_status(self):
        mesh, results, _, http, _ = mesh_fixture()
        http.wrong_sub = True
        mesh.run()
        self.assertEqual(results.cases["me-bearer-spoof"]["status"], "FAIL")

    def test_provider_ack_loss_never_retries_or_adopts(self):
        mesh, results, pg, _, receipt = mesh_fixture()
        pg.insert_ack_lost = True
        mesh.run()
        self.assertEqual(pg.calls.count("provider-insert"), 1)
        self.assertIsNone(receipt.last("provider-created"))
        self.assertEqual(results.cases["s2s-token"]["status"], "BLOCKED")
        self.assertIn("provider", e2e.cleanup(mesh)["residuals"])

    def test_provider_integer_ack_without_complete_fingerprint_is_not_adopted(self):
        mesh, results, pg, _, receipt = mesh_fixture()
        pg.insert_ack_missing_sha = True
        mesh.run()
        self.assertEqual(pg.calls.count("provider-insert"), 1)
        self.assertIsNone(receipt.last("provider-created"))
        self.assertEqual(results.cases["s2s-token"]["status"], "BLOCKED")
        self.assertTrue(pg.provider)

    def test_changed_provider_never_deleted(self):
        mesh, _, pg, _, receipt = mesh_fixture()
        mesh.values["user"] = USER
        pg.account = pg.provider = True
        pg.changed_provider = True
        receipt.append("account-created", mesh.values)
        receipt.append("provider-created", {**mesh.values, "id": 42, "sha": pg.provider_sha(), "nonce": RUN})
        receipt.append("account-snapshot", pg.snapshot())
        self.assertIn("provider", e2e.cleanup(mesh)["residuals"])
        self.assertNotIn("provider-delete", pg.calls)
        self.assertNotIn("account-deleted", pg.calls)

    def test_complete_provider_row_changes_and_read_delete_races_preserve_row(self):
        for field, value in (("refresh_token", "changed-synthetic-refresh"), ("expires_in", 10),
                             ("updated_at", "2026-11-02T12:00:01")):
            for race in (False, True):
                with self.subTest(field=field, race=race):
                    mesh, _, pg, _, receipt = mesh_fixture()
                    pg.account = pg.provider = True
                    mesh.values["user"] = USER
                    ack = {**mesh.values, "id": 42, "sha": pg.provider_sha(), "nonce": RUN}
                    receipt.append("account-created", mesh.values)
                    receipt.append("provider-created", ack)
                    receipt.append("account-snapshot", pg.snapshot())
                    initial_snapshot = receipt.last("account-snapshot")
                    if race:
                        pg.before_provider_delete = lambda row, field=field, value=value: row.__setitem__(field, value)
                    else:
                        pg.provider_row[field] = value
                    result = e2e.cleanup(mesh)
                    self.assertEqual(result["status"], "BLOCKED")
                    self.assertIn("provider", result["residuals"])
                    self.assertTrue(pg.provider)
                    self.assertEqual(pg.provider_row[field], value)
                    self.assertIsNone(receipt.last("provider-cleaned"))
                    self.assertEqual(receipt.last("account-snapshot"), initial_snapshot)
                    self.assertEqual(pg.calls.count("provider-delete"), int(race))

    def test_attempt_without_completed_http_is_not_executed(self):
        result = e2e.Results("apparatus")
        def timeout(case):
            case["attempted"] = True
            raise e2e.Blocked("HTTP transport unavailable")
        result.case("h3-invoke-hostname", timeout)
        report = result.report()
        self.assertEqual((report["attempted"], report["executed"], report["blocked"]), (1, 0, 6))

    def test_six_apparatus_obligations_never_collapse_to_one(self):
        result = e2e.Results("apparatus")
        result.block_remaining("missing deployed observer")
        self.assertEqual((result.report()["expected"], result.report()["blocked"]), (6, 6))


class ProvenanceAndSqlTests(unittest.TestCase):
    def action_fixture(self, root, pg, http):
        credentials = root / "credentials.json"
        credentials.write_text(json.dumps({"password": "private-password-fixture", "internal_token": "internal-token-fixture"}))
        inputs = {"run_id": RUN, "suite": "mesh-authn", "receipt_path": root / "receipt",
                  "credentials_path": credentials, "pki_directory": root / "certificates"}
        driver = SimpleNamespace(anchor_module=SimpleNamespace(secure=lambda *a, **k: None),
            runtime=SimpleNamespace(target=lambda: SimpleNamespace(privacy=lambda p: True, e2e_forward=lambda *a, **k: None)),
            strict_json=lambda p: json.loads(p.read_text()), NAMESPACES={"namespace": e2e.NAMESPACE}, namespace_owned=Mock())
        args = argparse.Namespace(action="prepare-probes", inputs=root / "inputs.json", suite=None, executed=0)
        return inputs, driver, args

    def test_prepare_empty_cleanup_e2e_refused_without_signup(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            pg, http = FakePg(), FakeHttp(FakePg())
            inputs, driver, args = self.action_fixture(root, pg, http)
            with patch.object(e2e, "read_inputs", return_value=inputs), patch.object(e2e, "binding", return_value={"run": RUN}), patch.object(
                    e2e, "Database", return_value=pg), patch.object(e2e, "Services", return_value=http):
                e2e.run_action(driver, args, {})
                self.assertTrue(args.preparation_report["prepared"])
                args.action = "cleanup-probes"
                self.assertEqual(e2e.run_action(driver, args, {})["cleanup"]["status"], "PASS")
                args.action, args.suite = "e2e", "mesh-authn"
                with self.assertRaisesRegex(e2e.Blocked, "terminal"):
                    e2e.run_action(driver, args, {})
            self.assertEqual(http.calls, [])
            receipt = e2e.Receipt(inputs["receipt_path"], driver, {"run": RUN})
            self.assertIsNone(receipt.last("signup-intent"))
            self.assertIsNone(receipt.last("execution-started"))

    def test_prepare_current_gitlab_and_notification_gap_never_prepared_true(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            http = FakeHttp(FakePg())
            http.providers = ["gitlab"]
            pg = e2e.Database(Mock(), {})
            inputs, driver, args = self.action_fixture(root, pg, http)
            with patch.object(e2e, "read_inputs", return_value=inputs), patch.object(e2e, "binding", return_value={"run": RUN}), patch.object(
                    e2e, "Database", return_value=pg), patch.object(e2e, "Services", return_value=http):
                e2e.run_action(driver, args, {})
            self.assertFalse(args.preparation_report["prepared"])
            self.assertEqual(args.executed, 0)
            reasons = " ".join(args.preparation_report["blocked_prerequisites"])
            self.assertIn("github absent", reasons)
            self.assertIn("notification/delivery inverse unqualified", reasons)
            pg.driver.runtime.kubectl.assert_not_called()
            self.assertEqual(http.calls, [])

    def test_input_schema_is_closed_no_url_sql_or_provider_override(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            credentials = root / "credentials.json"
            credentials.write_text('{}')
            path = root / "inputs.json"
            value = {"version": 1, "run_id": RUN, "suite": "mesh-authn", "provider_mode": "SIMULATED_PROVIDER",
                     "credentials_path": str(credentials), "pki_directory": str(root / "certificates"),
                     "receipt_path": str(root / "receipt")}
            driver = SimpleNamespace(runtime=SimpleNamespace(target=lambda: SimpleNamespace(privacy=lambda p: True)),
                                     strict_json=lambda p: json.loads(p.read_text()))
            path.write_text(json.dumps(value))
            self.assertEqual(e2e.read_inputs(path, driver)["run_id"], RUN)
            for change in ({"version": True}, {"run_id": str(uuid.uuid1())}, {"provider_mode": "VENDOR"},
                           {"url": "https://foreign.invalid"}, {"sql": "TRUNCATE users"}, {"pki_directory": "relative"}):
                with self.subTest(change=list(change)):
                    path.write_text(json.dumps({**value, **change}))
                    with self.assertRaises(e2e.Blocked):
                        e2e.read_inputs(path, driver)

    def test_receipt_freezes_mutable_creation_intent_before_ack(self):
        with tempfile.TemporaryDirectory() as directory:
            driver = SimpleNamespace(anchor_module=SimpleNamespace(secure=lambda *a, **k: None),
                                     runtime=SimpleNamespace(target=lambda: SimpleNamespace(privacy=lambda p: True)),
                                     strict_json=lambda p: json.loads(p.read_text()))
            path = Path(directory).resolve() / "receipt"
            receipt = e2e.Receipt(path, driver, {"run": RUN}, create=True)
            values = {"run": RUN}
            receipt.append("intent", values)
            values["user"] = USER
            receipt.append("ack", values)
            restored = e2e.Receipt(path, driver, {"run": RUN})
            self.assertEqual(restored.last("intent"), {"run": RUN})
            self.assertEqual(restored.last("ack"), values)
            restored.append("cleanup", {"complete": True})
            with self.assertRaisesRegex(e2e.Blocked, "terminal"):
                restored.append("signup-intent", values)

    def test_receipt_chain_and_source_binding_reject_change(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory).resolve() / "receipt"
            driver = SimpleNamespace(anchor_module=SimpleNamespace(secure=lambda *a, **k: None),
                                     runtime=SimpleNamespace(target=lambda: SimpleNamespace(privacy=lambda p: True)),
                                     strict_json=lambda p: json.loads(p.read_text()))
            receipt = e2e.Receipt(path, driver, {"run": RUN}, create=True)
            receipt.append("prepared", {"created_resources": []})
            self.assertEqual(len(e2e.Receipt(path, driver, {"run": RUN}).events), 2)
            with self.assertRaises(e2e.Blocked):
                e2e.Receipt(path, driver, {"run": str(uuid.uuid4())})
            event = path / "000001.json"
            event.write_text('{"kind":"prepared","data":{},"previous":"wrong"}')
            with self.assertRaises(e2e.Blocked):
                e2e.Receipt(path, driver, {"run": RUN})

    def test_provider_integer_pk_and_closed_operation_before_any_exec(self):
        driver = Mock()
        database = e2e.Database(driver, {})
        values = {"run": RUN, "user": USER, "nonce": RUN, "token": "simulated-provider-" + RUN, "id": USER}
        with self.assertRaises(e2e.Blocked):
            database.execute("provider-delete", values)
        with self.assertRaises(e2e.Blocked):
            database.execute("TRUNCATE users", values)
        driver.runtime.kubectl.assert_not_called()

    def test_cleanup_rejects_unknown_dependents_before_delete(self):
        driver = Mock()
        database = e2e.Database(driver, {})
        database.qualify_account_cleanup = Mock()
        expected = {table: [] for table in e2e.ACCOUNT_TABLES}
        expected["users"] = [{"id": USER}]
        expected["oauth_transactions"] = [{"id": "foreign"}]
        database.execute = Mock()
        with self.assertRaises(e2e.Blocked):
            database.cleanup_account({"run": RUN, "user": USER}, expected)
        database.execute.assert_not_called()

    def test_fixed_sql_parameters_guards_and_no_global_inverse(self):
        insert = e2e.DB_SQL["provider-insert"]
        inverse = e2e.DB_SQL["provider-delete"]
        self.assertIn("NOT EXISTS", insert)
        self.assertIn("RETURNING jsonb_build_object('id',id,'sha'", insert)
        self.assertIn("row_to_json(p)", insert)
        for field in ("id", "user", "nonce", "token", "sha"):
            self.assertIn("$1->>'" + field + "'", inverse)
        self.assertIn("observed.jsonb_build_object=$1->'snapshot'", e2e.DB_SQL["account-delete"])
        self.assertIn("row_to_json(p)", inverse)
        self.assertIn("=$1->>'sha'", inverse)
        self.assertIn("=$1->>'sha'", e2e.DB_SQL["provider-state"])
        for sql in e2e.DB_SQL.values():
            for forbidden in ("TRUNCATE", "DROP TABLE", "is_verified =", "DELETE FROM users;"):
                self.assertNotIn(forbidden, sql)

    def test_psql_fixed_parameter_frame_and_creation_lock(self):
        pod = {"metadata": {"name": "postgres-test", "uid": "pod-uid", "ownerReferences": [
            {"kind": "ReplicaSet", "name": "postgres-rs", "uid": "rs-uid"}]}, "status": {"phase": "Running"}}
        calls = []
        def kubectl(*arguments, **kwargs):
            calls.append((arguments, kwargs))
            if arguments[0] == "get":
                return json.dumps({"items": [pod]})
            return '{"id":42}'
        driver = SimpleNamespace(runtime=SimpleNamespace(kubectl=kubectl),
            owned=lambda *a: {"metadata": {"uid": "deployment-uid"}},
            obj=lambda kind, *a: {"metadata": {"uid": "rs-uid", "ownerReferences": [
                {"kind": "Deployment", "uid": "deployment-uid"}]}} if kind == "ReplicaSet" else pod)
        values = {"run": RUN, "user": USER, "nonce": RUN, "token": "simulated-provider-" + RUN}
        self.assertEqual(e2e.Database(driver, {}).execute("provider-insert", values), {"id": 42})
        arguments, options = calls[-1]
        self.assertEqual(arguments[:4], ("exec", "-i", "-n", e2e.NAMESPACE))
        self.assertIn("-X", arguments)
        self.assertNotIn(values["token"], str(arguments))
        self.assertIn("LOCK TABLE provider_tokens IN SHARE ROW EXCLUSIVE MODE", options["stdin"])
        self.assertIn("PREPARE local_e2e(jsonb)", options["stdin"])
        self.assertNotIn(values["token"], options["stdin"])


class RetentionContractTests(unittest.TestCase):
    """§8 contract oracles, not a live run or qualification of an inverse.

    C-G1: existing ProtocolTests retain the strict negative controls.
    C-G2: a valid v2 must pass before malformed-v2 rejection is meaningful.
    C-G3/4: prepare is read-only until its fixed, single-use authority is claimed.
    C-G5/7: cleanup retains even uncertain effects, never attempts an inverse.
    C-G6: a reloaded terminal/receipt cannot recreate a mutation capability.
    """

    CONSENT = "835d481e-099e-4a23-adf5-b01b1ca9af30"
    POLICY = {"mode": "RETAIN_LOCAL_V1", "consent_id": CONSENT}
    RESIDUALS = ["accounts", "sessions", "provider-fixture", "organizations", "mail",
                 "notifications", "deliveries", "outbox", "fga"]

    def fixture(self, root):
        pg = FakePg()
        http = FakeHttp(pg)
        http.providers = ["github"]  # G-1: the approved C profile, not GitLab coexistence.
        inputs, driver, args = ProvenanceAndSqlTests().action_fixture(root, pg, http)
        inputs.update(version=2, provider_mode="SIMULATED_PROVIDER", cleanup_policy=dict(self.POLICY))
        inputs["pki_directory"].mkdir()
        args.inputs.write_text(json.dumps({k: str(v) if isinstance(v, Path) else v for k, v in inputs.items()}), encoding="utf-8")
        config = root / "private-target" / "config.json"
        config.parent.mkdir()
        config.write_text("{}", encoding="utf-8")
        target = SimpleNamespace(config=config, privacy=lambda p: True, e2e_forward=lambda *a, **k: None)
        driver.runtime.target = lambda: target
        # Source/PKI binding is isolated as in the historical action fixture.
        # These fields come from §8.1, not from a future helper API.
        bound = {"version": 2, "run_id": RUN, "suite": "mesh-authn", "source": {},
                 "git": "a97e51d4185aca3b0d78f997e03df56f0288a254", "context": "kind-aiforall-local-full",
                 "namespace_uids": {e2e.NAMESPACE: "namespace-fixture-uid"},
                 "credential_sha256": hashlib.sha256(inputs["credentials_path"].read_bytes()).hexdigest(),
                 "public_certificates_sha256": {}, "input_sha256": hashlib.sha256(args.inputs.read_bytes()).hexdigest(),
                 "cleanup_policy": dict(self.POLICY),
                 "target_anchor": {"task_id": "retention-test-only", "kube_system_uid": "system-fixture-uid",
                                   "nodeContainerIds": ["node-fixture"], "openbao_epoch_sha256": "b" * 64}}
        canonical = json.dumps(bound, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()
        consent = {"version": 1, "consent_id": self.CONSENT,
                   "decision_ref": "20261103-user-retain-local-one-run", "run_id": RUN,
                   "mode": "RETAIN_LOCAL_V1", "suite": "mesh-authn",
                   "binding_sha256": hashlib.sha256(canonical).hexdigest(), "max_executions": 1,
                   "final_business_cleanup": "FORBIDDEN", "required_business_revoke": "PRESERVED",
                   "sessions": "MAY_REMAIN", "disposition_owner": "mission-lead"}
        authority = config.parent / "e2e-retention-v1"
        authority.mkdir()
        consent_path = authority / "consent.json"
        consent_path.write_text(json.dumps(consent), encoding="utf-8")
        consent_sha = hashlib.sha256(consent_path.read_bytes()).hexdigest()
        bound["consent_sha256"] = consent_sha
        claim = {"version": 1, "consent_id": self.CONSENT, "consent_sha256": consent_sha,
                 "run_id": RUN, "binding_sha256": consent["binding_sha256"],
                 "receipt_path": str(inputs["receipt_path"])}
        return SimpleNamespace(pg=pg, http=http, inputs=inputs, driver=driver, args=args,
                               bound=bound, authority=authority, claim=claim)

    def rebind_attempt(self, fixture):
        """Make a new attempt internally consistent, leaving the old claim intact.

        Otherwise rejection on an unrelated stale input hash would masquerade
        as coverage of the fixed-authority anti-replay rule.
        """
        f = fixture
        f.args.inputs.write_text(json.dumps({k: str(v) if isinstance(v, Path) else v for k, v in f.inputs.items()}), encoding="utf-8")
        f.bound.update(version=f.inputs["version"], run_id=f.inputs["run_id"],
                       input_sha256=hashlib.sha256(f.args.inputs.read_bytes()).hexdigest())
        f.bound.pop("consent_sha256", None)
        if f.inputs["version"] == 1:
            f.bound.pop("cleanup_policy", None)
            f.bound.pop("target_anchor", None)
            return
        f.bound["cleanup_policy"] = copy.deepcopy(f.inputs["cleanup_policy"])
        path = f.authority / "consent.json"
        consent = json.loads(path.read_text(encoding="utf-8"))
        consent.update(run_id=f.inputs["run_id"], consent_id=f.inputs["cleanup_policy"]["consent_id"],
                       binding_sha256=hashlib.sha256(json.dumps(
                           f.bound, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()).hexdigest())
        path.write_text(json.dumps(consent), encoding="utf-8")
        f.bound["consent_sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()

    @contextlib.contextmanager
    def action(self, fixture):
        # Only pre-existing public Ops seams are doubled. Receipt/cleanup and
        # run_action remain real, and all paths are in this test's temp directory.
        with patch.object(e2e, "read_inputs", return_value=fixture.inputs), patch.object(
                e2e, "binding", return_value=fixture.bound), patch.object(
                e2e, "Database", return_value=fixture.pg), patch.object(
                e2e, "Services", return_value=fixture.http):
            yield

    def test_c_g2_valid_explicit_v2_is_accepted_without_defaulting_to_strict(self):
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            value = {key: str(value) if isinstance(value, Path) else value for key, value in f.inputs.items()}
            f.args.inputs.write_text(json.dumps(value), encoding="utf-8")
            actual = e2e.read_inputs(f.args.inputs, f.driver)
            self.assertEqual(actual["version"], 2)
            self.assertEqual(actual["cleanup_policy"], self.POLICY)
            self.assertEqual(actual["receipt_path"], f.inputs["receipt_path"])
            self.assertEqual(list(f.authority.iterdir()), [f.authority / "consent.json"])

    def test_c_g2_v2_rejects_ambiguous_policy_and_strict_never_accepts_it(self):
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            value = {key: str(value) if isinstance(value, Path) else value for key, value in f.inputs.items()}
            changes = [{"version": True}, {"version": "2"}, {"version": 1}, {"suite": "apparatus"},
                       {"cleanup_policy": None}, {"cleanup_policy": "RETAIN_LOCAL_V1"},
                       {"cleanup_policy": {"mode": "RETAIN_LOCAL_V1"}},
                       {"cleanup_policy": {**self.POLICY, "skip_cleanup": True}},
                       {"cleanup_policy": {**self.POLICY, "mode": "retain_local_v1"}},
                       {"cleanup_policy": {**self.POLICY, "consent_id": self.CONSENT.upper()}},
                       {"cleanup_policy": {**self.POLICY, "consent_id": RUN.replace("4dac", "1dac")}},
                       {"authority_path": str(f.authority)}, {"skip_cleanup": True}]
            for change in changes:
                with self.subTest(change=change):
                    f.args.inputs.write_text(json.dumps({**value, **change}), encoding="utf-8")
                    with self.assertRaises(e2e.Blocked):
                        e2e.read_inputs(f.args.inputs, f.driver)
            self.assertFalse((f.authority / "claim.json").exists())

    def test_c_g3_ready_prepare_does_not_qualify_final_inverses_or_execute_business(self):
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            with self.action(f), patch.object(f.pg, "qualify_notifications_cleanup") as notifications, patch.object(
                    f.pg, "qualify_account_cleanup") as account:
                report = e2e.run_action(f.driver, f.args, {})
            notifications.assert_not_called()
            account.assert_not_called()
            self.assertTrue(report["prepared"])
            self.assertEqual(f.args.executed, 0)
            self.assertEqual(f.http.calls, [])
            self.assertFalse(f.pg.account)
            claim = json.loads((f.authority / "claim.json").read_text(encoding="utf-8"))
            self.assertEqual(claim, f.claim)
            receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound)
            self.assertEqual(receipt.last("retention-prepared"), {
                "mode": "RETAIN_LOCAL_V1", "prepared": True,
                "inverse_qualification": "UNQUALIFIED_NOT_REQUIRED_BY_CONSENT",
                "remaining_prerequisites": [], "created_resources": [],
                "exceptions": ["telegraph-final-inverse", "account-final-inverse"]})
            self.assertIsNone(receipt.last("execution-started"))

    def test_c_g3_non_github_only_profile_never_claims_or_creates_receipt(self):
        # G-1: GitHub alone moved to the positive fixture; mixed/ambiguous is not
        # an alternative authority to consume the one local retention chance.
        for providers in ([], ["gitlab"], ["github", "gitlab"], ["github", "github"], ["other"]):
            with self.subTest(providers=providers), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                f.http.providers = providers
                with self.action(f):
                    try:
                        report = e2e.run_action(f.driver, f.args, {})
                    except e2e.Blocked:
                        report = {"prepared": False}
                self.assertFalse(report["prepared"])
                self.assertFalse((f.authority / "claim.json").exists())
                self.assertFalse(f.inputs["receipt_path"].exists())
                self.assertEqual(f.http.calls, [])
                self.assertFalse(f.pg.account)

    def test_gh_g1_github_only_runs_mesh39_and_retains_without_final_inverses(self):
        """G-1: valid C reaches real Mesh.run; no reduced inventory or cleanup PASS."""
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            with self.action(f):
                prepared = e2e.run_action(f.driver, f.args, {})
            self.assertTrue(prepared["prepared"])
            self.assertEqual(f.http.calls, [], "read-only preparation must precede signup")
            f.args.action, f.args.suite = "e2e", "mesh-authn"
            with self.action(f), patch.object(f.pg, "qualify_notifications_cleanup") as notifications, patch.object(
                    f.pg, "qualify_account_cleanup") as account, patch.object(f.pg, "cleanup_account") as inverse:
                report = e2e.run_action(f.driver, f.args, {})
            notifications.assert_not_called()
            account.assert_not_called()
            inverse.assert_not_called()
            self.assertEqual((report["expected"], report["passed"], report["executed"]), (39, 39, 39))
            self.assertEqual(len(report["cases"]), 39)
            self.assertEqual(len(e2e.APPARATUS_CASES), 6)
            self.assertEqual(report["provider_fixture"], "SIMULATED_PROVIDER")
            self.assertEqual(report["launch_status"], "STARTED")
            self.assertEqual(report["cleanup"]["status"], "RETAINED")
            self.assertIs(report["cleanup"]["complete"], False)
            self.assertTrue(f.pg.account)
            self.assertFalse(f.pg.provider, "the required business revoke is not a final cleanup inverse")
            deletes = [(path, service) for method, path, service, _ in f.http.calls if method == "DELETE"]
            self.assertEqual(len(deletes), 12, "keep the authorized revoke and its 11 negative controls")
            self.assertEqual(set(deletes), {("/iam/internal/github/revoke", "iam")})
            self.assertNotIn("provider-delete", f.pg.calls)
            current = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound)
            self.assertEqual(current.last("iam-provider-registry")["providers"], ["github"])
            self.assertEqual(current.last("retained")["business_cleanup"], "NOT_ATTEMPTED")
            self.assertEqual(json.loads((f.authority / "terminal.json").read_text(encoding="utf-8"))["status"], "RETAINED")
            self.assertNotIn("a" * 64, json.dumps(current.last("iam-provider-registry")))

    def test_c_g4_claimed_crash_cannot_rearm_via_new_receipt_uuid_consent_or_strict(self):
        # Crash after exclusive claim, before receipt exists: the allowed chance
        # is spent. Do not mistake a new empty receipt directory for authority.
        for change in ({}, {"run_id": USER}, {"cleanup_policy": {**self.POLICY, "consent_id": USER}},
                       {"version": 1}):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                claim_path = f.authority / "claim.json"
                claim_path.write_text(json.dumps(f.claim), encoding="utf-8")
                before = claim_path.read_bytes()
                f.inputs.update(change)
                if f.inputs["version"] == 1:
                    f.inputs.pop("cleanup_policy")
                f.inputs["receipt_path"] = Path(directory).resolve() / "new-receipt"
                self.rebind_attempt(f)
                with self.action(f), self.assertRaises(e2e.Blocked):
                    e2e.run_action(f.driver, f.args, {})
                self.assertEqual(claim_path.read_bytes(), before)
                self.assertFalse(f.inputs["receipt_path"].exists())
                self.assertEqual(f.http.calls, [])

    def test_c_g4_consumed_crash_or_partial_claim_refuses_new_preparation(self):
        for contents, consumed in (("{", False), (None, True)):
            with self.subTest(partial=contents is not None), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                (f.authority / "claim.json").write_text(contents or json.dumps(f.claim), encoding="utf-8")
                if consumed:
                    (f.authority / "consumed.json").write_text(json.dumps(f.claim), encoding="utf-8")
                before = {p.name: p.read_bytes() for p in f.authority.iterdir()}
                with self.action(f), self.assertRaises(e2e.Blocked):
                    e2e.run_action(f.driver, f.args, {})
                self.assertEqual({p.name: p.read_bytes() for p in f.authority.iterdir()}, before)
                self.assertFalse(f.inputs["receipt_path"].exists())
                self.assertFalse(f.pg.account)

    def test_c_g2_changed_or_noncanonical_consent_never_claims_the_slot(self):
        for change in ({"version": True}, {"max_executions": 2}, {"decision_ref": "another-decision"},
                       {"run_id": USER}, {"binding_sha256": "C" * 64}, {"extra": "forbidden"},
                       {"final_business_cleanup": "ALLOWED"}):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                path = f.authority / "consent.json"
                value = json.loads(path.read_text(encoding="utf-8"))
                path.write_text(json.dumps({**value, **change}), encoding="utf-8")
                before = path.read_bytes()
                with self.action(f), self.assertRaises(e2e.Blocked):
                    e2e.run_action(f.driver, f.args, {})
                self.assertEqual(path.read_bytes(), before)
                self.assertFalse((f.authority / "claim.json").exists())
                self.assertFalse(f.inputs["receipt_path"].exists())
                self.assertEqual(f.http.calls, [])

    def test_c_g5_cleanup_keeps_acked_and_uncertain_effects_without_recovery_login(self):
        for lost_ack in (False, True):
            with self.subTest(lost_ack=lost_ack):
                mesh, _, pg, http, receipt = mesh_fixture()
                mesh.inputs.update(version=2, cleanup_policy=dict(self.POLICY))
                receipt.append("binding", {"version": 2, "cleanup_policy": dict(self.POLICY)})
                # Use the historical isolated protocol seed to create ACKs. Its
                # synthetic qualified inverse is NOT claimed as production proof.
                mesh.run()
                if lost_ack:
                    receipt.events[:] = [e for e in receipt.events if e["kind"] not in {
                        "account-created", "provider-created", "organization-created"}]
                mesh.token = ""  # would trigger the strict recovery login
                calls, sql, events = len(http.calls), len(pg.calls), len(receipt.events)
                outcome = e2e.cleanup(mesh)
                self.assertEqual(http.calls[calls:], [], "retention must not login, read/adopt or DELETE for cleanup")
                self.assertEqual(pg.calls[sql:], [], "retention must not prepare or execute final inverse SQL")
                self.assertTrue(pg.account)
                self.assertFalse(http.message_deleted)
                # This lower-level double has no owned authority invocation:
                # BLOCKED is permitted, destructive fallback/PASS is not.
                # Successful terminal generation/repair still needs the future
                # owned invocation seam; this is the zero-inverse boundary only.
                self.assertIn(outcome["status"], {"RETAINED", "BLOCKED"})
                self.assertEqual(outcome["removed"], 0)
                new_events = receipt.events[events:]
                self.assertFalse({e["kind"] for e in new_events} & {
                    "account-deleted", "organization-deleted", "provider-cleaned", "mail-deleted"})
                self.assertFalse(any(e["kind"] == "cleanup" and e["data"].get("complete") is True for e in new_events))
                serialized = json.dumps(new_events)
                for secret in ("private-password-fixture", "internal-token-fixture", "login-refresh", "owned-token"):
                    self.assertNotIn(secret, serialized)

    def test_c_g5_business_revoke_and_token404_remain_among_all_39_oracles(self):
        mesh, results, pg, http, _ = mesh_fixture()
        mesh.run()
        self.assertEqual((results.report()["expected"], results.report()["passed"]), (39, 39))
        self.assertIn(("DELETE", "/iam/internal/github/revoke", "iam", "envoy-mesh"), http.calls)
        self.assertFalse(pg.provider, "business revoke, not cleanup, must remove the fixture")
        self.assertEqual(len(e2e.APPARATUS_CASES), 6)

    def test_c_g6_retained_false_complete_is_terminal_after_receipt_reload(self):
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound, create=True)
            receipt.append("retained", {"status": "RETAINED", "complete": False, "mode": "RETAIN_LOCAL_V1",
                "business_cleanup": "NOT_ATTEMPTED", "technical_cleanup": "PASS", "effects_possible": True,
                "inventory_complete": False, "residual_classes": self.RESIDUALS, "uncertainties": ["signup-ack"]})
            restored = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound)
            before = {p.name: p.read_bytes() for p in receipt.path.iterdir()}
            for kind in ("execution-started", "signup-intent", "account-created", "provider-intent", "organization-intent"):
                with self.subTest(kind=kind), self.assertRaises(e2e.Blocked):
                    restored.append(kind, {"run": RUN})
            self.assertEqual({p.name: p.read_bytes() for p in receipt.path.iterdir()}, before)

    def test_c_g4_reloaded_receipt_is_not_a_live_signup_capability(self):
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound, create=True)
            for name in ("claim.json", "consumed.json"):
                (f.authority / name).write_text(json.dumps(f.claim), encoding="utf-8")
            restored = e2e.Receipt(receipt.path, f.driver, f.bound)
            mesh = e2e.Mesh(f.http, f.pg, restored,
                            {"password": "private-password-fixture", "internal_token": "internal-token-fixture"},
                            f.inputs, e2e.Results("mesh-authn"))
            with self.assertRaises(e2e.Blocked):
                mesh.signup({})
            self.assertFalse(f.pg.account)
            self.assertEqual(f.http.calls, [])
            self.assertIsNone(restored.last("signup-intent"))

    def test_c_g4_consumed_precedes_business_and_finally_records_uncertain_retention(self):
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            (f.authority / "claim.json").write_text(json.dumps(f.claim), encoding="utf-8")
            receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound, create=True)
            receipt.append("retention-prepared", {
                "mode": "RETAIN_LOCAL_V1", "prepared": True,
                "inverse_qualification": "UNQUALIFIED_NOT_REQUIRED_BY_CONSENT",
                "remaining_prerequisites": [], "created_resources": [],
                "exceptions": ["telegraph-final-inverse", "account-final-inverse"]})
            f.args.action, f.args.suite = "e2e", "mesh-authn"

            def interrupted_business():
                consumed = json.loads((f.authority / "consumed.json").read_text(encoding="utf-8"))
                self.assertEqual(consumed, f.claim)
                current = e2e.Receipt(receipt.path, f.driver, f.bound)
                self.assertEqual(current.last("execution-started"), {"suite": "mesh-authn"})
                raise e2e.Blocked("test interruption after consumption before signup ACK")

            with self.action(f), patch.object(e2e.Mesh, "run", side_effect=interrupted_business) as business:
                try:
                    e2e.run_action(f.driver, f.args, {})
                except e2e.Blocked:
                    pass  # Either exception/report policy is allowed at this boundary.
            business.assert_called_once()  # No environmental/preparation refusal masquerades as this scenario.
            self.assertEqual(f.http.calls, [])
            terminal = json.loads((f.authority / "terminal.json").read_text(encoding="utf-8"))
            retained = e2e.Receipt(receipt.path, f.driver, f.bound).last("retained")
            self.assertIsNotNone(retained)
            self.assertEqual((retained["status"], retained["complete"], retained["effects_possible"]),
                             ("RETAINED", False, True))
            self.assertEqual(retained["business_cleanup"], "NOT_ATTEMPTED")
            self.assertEqual(retained["residual_classes"], self.RESIDUALS)
            self.assertIs(retained["inventory_complete"], False)
            self.assertEqual(terminal, {**retained, "version": 1, "run_id": RUN, "consent_id": self.CONSENT,
                "binding_sha256": f.claim["binding_sha256"], "consent_sha256": f.claim["consent_sha256"]})

    def test_c_g7_closed_retained_record_rejects_free_text_secret_before_persisting(self):
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound, create=True)
            invalid = {"status": "RETAINED", "complete": False, "mode": "RETAIN_LOCAL_V1",
                       "business_cleanup": "NOT_ATTEMPTED", "technical_cleanup": "PASS", "effects_possible": True,
                       "inventory_complete": False, "residual_classes": self.RESIDUALS,
                       "uncertainties": ["oauth-token-SECRET-CONTENT"]}
            before = {p.name: p.read_bytes() for p in receipt.path.iterdir()}
            with self.assertRaises(e2e.Blocked):
                receipt.append("retained", invalid)
            self.assertEqual({p.name: p.read_bytes() for p in receipt.path.iterdir()}, before)

    def test_c_g6_observed_signup_failure_attests_start_but_transport_failure_does_not(self):
        for observed in (True, False):
            with self.subTest(observed=observed), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                (f.authority / "claim.json").write_text(json.dumps(f.claim), encoding="utf-8")
                receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound, create=True)
                receipt.append("retention-prepared", {
                    "mode": "RETAIN_LOCAL_V1", "prepared": True,
                    "inverse_qualification": "UNQUALIFIED_NOT_REQUIRED_BY_CONSENT",
                    "remaining_prerequisites": [], "created_resources": [],
                    "exceptions": ["telegraph-final-inverse", "account-final-inverse"]})
                f.args.action, f.args.suite = "e2e", "mesh-authn"
                original = f.http.request

                def signup_response(method, path, *args, **kwargs):
                    if path == "/iam/api/auth/signup":
                        kwargs["case"].update(attempted=True, observed=observed)
                        if not observed:
                            raise e2e.Blocked("test transport failed without response")
                        return 400, b'{"error":"SECRET-BODY-MUST-NOT-BE-JOURNALED"}'
                    return original(method, path, *args, **kwargs)

                def one_business_oracle(mesh):
                    # Focus the journal boundary, not a shortened E2E campaign:
                    # all other obligations stay explicitly BLOCKED in this test.
                    mesh.results.case("signup", mesh.signup)
                    mesh.results.block_remaining("not exercised by isolated journal test")

                with self.action(f), patch.object(f.http, "request", side_effect=signup_response), patch.object(
                        e2e.Mesh, "run", autospec=True, side_effect=one_business_oracle) as business:
                    report = e2e.run_action(f.driver, f.args, {})
                business.assert_called_once()
                self.assertEqual((report["expected"], len(report["cases"])), (39, 39))
                self.assertEqual(report["launch_status"], "STARTED" if observed else "NOT_PROVEN")
                current = e2e.Receipt(receipt.path, f.driver, f.bound)
                marker = current.last("case-observed")
                if observed:
                    self.assertIsNotNone(marker)
                    self.assertEqual({key: marker[key] for key in (
                        "id", "status", "attempted", "observed", "completed", "evidence", "http_status")},
                        {"id": "signup", "status": "FAIL", "attempted": True, "observed": True,
                         "completed": True, "evidence": "LIVE", "http_status": 400})
                    self.assertTrue(marker["checked_assertions"])
                    self.assertTrue(marker["failed_assertions"])
                    self.assertEqual((report["failed"], report["executed"]), (1, 1))
                else:
                    self.assertIsNone(marker)
                    self.assertEqual(report["executed"], 0)
                self.assertNotIn("SECRET-BODY-MUST-NOT-BE-JOURNALED", json.dumps(current.events))
                self.assertFalse(f.pg.account)

    def test_c_g6_empty_assertion_manifest_cannot_attest_launch(self):
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound, create=True)
            # The accepted source-specific assertion names do not exist yet.
            # Do not invent them or claim independent coverage of the other
            # schema fields using an already-invalid assertion manifest.
            event = {"id": "signup", "status": "PASS", "attempted": True, "observed": True,
                     "completed": True, "evidence": "LIVE", "http_status": 202,
                     "checked_assertions": [], "failed_assertions": []}
            with self.assertRaises(e2e.Blocked):
                receipt.append("case-observed", event)
            self.assertIsNone(receipt.last("case-observed"))

    # Additional §8 gaps: G-1 lock/slot, G-2 durable private JSON, G-3 binding,
    # G-4 terminal repair, G-5 owned teardown, G-6 source assertion manifest.
    # Production was read in this context; the oracles are the ratified §8, not
    # observations promoted into expected behaviour. No live qualification here.

    def capability(self, fixture):
        return e2e._Retention(fixture.driver, fixture.inputs, fixture.bound,
                              lambda: copy.deepcopy(fixture.bound))

    def prepared_receipt(self, fixture):
        receipt = e2e.Receipt(fixture.inputs["receipt_path"], fixture.driver, fixture.bound, create=True)
        receipt.append("retention-prepared", copy.deepcopy(e2e.RETENTION_PREPARED))
        return receipt

    def test_g1_lock_contention_and_stale_marker_never_release_another_owner(self):
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            first, second = self.capability(f), self.capability(f)
            lock = f.authority / "operation.lock"
            with first.operation():
                before = lock.read_bytes()
                with self.assertRaises(e2e.Blocked), second.operation():
                    self.fail("contending operation entered")
                self.assertEqual(lock.read_bytes(), before)
            self.assertFalse(lock.exists())
            lock.write_bytes(b'{"version":1,"owner":"stale-foreign"}')
            stale = lock.read_bytes()
            with self.assertRaises(e2e.Blocked), second.operation():
                self.fail("stale marker was adopted")
            self.assertEqual(lock.read_bytes(), stale)
            self.assertFalse((f.authority / "claim.json").exists())

    def test_g1_replaced_lock_survives_owner_exit_without_business_effect(self):
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            cap = self.capability(f)
            lock = f.authority / "operation.lock"
            foreign = b'{"version":1,"owner":"foreign-replacement"}'
            with self.assertRaises(e2e.Blocked), cap.operation():
                lock.unlink()
                lock.write_bytes(foreign)
                with self.assertRaises(e2e.Blocked):
                    cap.prepare()
            self.assertEqual(lock.read_bytes(), foreign)
            self.assertFalse((f.authority / "claim.json").exists())
            self.assertEqual(f.http.calls, [])

    def test_g1_exclusive_claim_and_consumed_collisions_preserve_winner(self):
        # Insert the competing file at O_EXCL, after the preliminary lexists.
        for name in ("claim.json", "consumed.json"):
            with self.subTest(record=name), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                cap = self.capability(f)
                original_open = os.open
                record = f.authority / name
                winner = b'{"foreign":"do-not-adopt"}'

                def competing_open(path, flags, *args, **kwargs):
                    if Path(path) == record:
                        record.write_bytes(winner)
                    return original_open(path, flags, *args, **kwargs)

                with cap.operation():
                    if name == "consumed.json":
                        cap.prepare()
                        receipt = self.prepared_receipt(f)
                    with patch.object(e2e.os, "open", side_effect=competing_open), self.assertRaises(e2e.Blocked):
                        cap.prepare() if name == "claim.json" else cap.consume(receipt)
                    self.assertEqual(record.read_bytes(), winner)
                    self.assertFalse(cap.active)
                    self.assertFalse(cap.consumption_owned)
                with self.action(f), self.assertRaises(e2e.Blocked):
                    e2e.run_action(f.driver, f.args, {})
                self.assertEqual(record.read_bytes(), winner)
                self.assertEqual(f.http.calls, [])

    def test_g2_failed_fsync_leaves_claim_or_consumed_spent_without_capability(self):
        for name in ("claim.json", "consumed.json"):
            with self.subTest(record=name), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                cap = self.capability(f)
                with cap.operation():
                    if name == "consumed.json":
                        cap.prepare()
                        receipt = self.prepared_receipt(f)
                    with patch.object(e2e.os, "fsync", side_effect=OSError("PRIVATE-HMAC-SENTINEL")), self.assertRaises(e2e.Blocked) as raised:
                        cap.prepare() if name == "claim.json" else cap.consume(receipt)
                    self.assertNotIn("PRIVATE-HMAC-SENTINEL", str(raised.exception))
                    self.assertFalse(cap.active)
                    self.assertEqual(cap.consumption_owned, name == "consumed.json")
                    if name == "consumed.json":
                        self.assertIsNone(receipt.last("execution-started"))
                record = f.authority / name
                before = record.read_bytes()
                self.assertTrue(before, "the exclusive record exists despite failed durability ACK")
                with self.action(f), self.assertRaises(e2e.Blocked):
                    e2e.run_action(f.driver, f.args, {})
                self.assertEqual(record.read_bytes(), before)
                self.assertEqual(f.http.calls, [])

    def test_g2_failed_privacy_or_readback_never_advances_partial_receipt(self):
        for failure in ("privacy", "readback"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound, create=True)
                file = receipt.path / "000001.json"
                original_read = Path.read_bytes

                def readback(path):
                    return b"PRIVATE-READBACK-SENTINEL" if path == file else original_read(path)

                seam = (patch.object(f.driver.anchor_module, "secure", side_effect=ValueError("PRIVATE-ACL-SENTINEL"))
                        if failure == "privacy" else patch.object(Path, "read_bytes", readback))
                with seam, self.assertRaises(e2e.Blocked) as raised:
                    receipt.append("retention-prepared", copy.deepcopy(e2e.RETENTION_PREPARED))
                self.assertNotIn("PRIVATE-", str(raised.exception))
                self.assertIsNone(receipt.last("retention-prepared"))
                before = file.read_bytes()
                with self.assertRaises(e2e.Blocked):
                    receipt.append("retention-prepared", copy.deepcopy(e2e.RETENTION_PREPARED))
                self.assertEqual(file.read_bytes(), before, "no replacement/retry of an uncertain event")

    def test_g2_bom_valid_and_duplicate_nonfinite_or_partial_json_refused(self):
        for name in ("inputs", "consent"):
            with self.subTest(record=name), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                path = f.args.inputs if name == "inputs" else f.authority / "consent.json"
                raw = path.read_bytes()
                path.write_bytes(b"\xef\xbb\xbf" + raw)
                if name == "inputs":
                    self.assertEqual(e2e.read_inputs(path, f.driver)["cleanup_policy"], self.POLICY)
                else:
                    self.assertEqual(self.capability(f).claim["consent_sha256"], hashlib.sha256(path.read_bytes()).hexdigest())
                for invalid in (b'{"version":2,' + raw[1:], b'{"unknown":NaN,' + raw[1:],
                                b'{"unknown":Infinity,' + raw[1:], b'{"unknown":1e999,' + raw[1:],
                                b'{"unknown":"PRIVATE-JSON-SENTINEL",' + raw[1:], b"{", b"\xff"):
                    with self.subTest(invalid=invalid[:25]):
                        path.write_bytes(invalid)
                        with self.assertRaises(e2e.Blocked) as raised:
                            e2e.read_inputs(path, f.driver) if name == "inputs" else self.capability(f)
                        self.assertNotIn("PRIVATE-JSON-SENTINEL", str(raised.exception))
                self.assertFalse((f.authority / "claim.json").exists())
                self.assertEqual(f.http.calls, [])

    def test_g2_receipt_closed_envelope_and_strict_json_rejected_on_reload(self):
        for bad in ("extra", "duplicate", "nonfinite", "partial"):
            with self.subTest(bad=bad), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                receipt = self.prepared_receipt(f)
                file = receipt.path / "000001.json"
                raw = file.read_bytes()
                if bad == "extra":
                    changed = json.loads(raw)
                    changed["foreign"] = "PRIVATE-RECEIPT-SENTINEL"
                    file.write_text(json.dumps(changed), encoding="utf-8")
                elif bad == "duplicate":
                    file.write_bytes(b'{"kind":"retention-prepared",' + raw[1:])
                elif bad == "nonfinite":
                    file.write_bytes(b'{"foreign":NaN,' + raw[1:])
                else:
                    file.write_bytes(b"{")
                before = file.read_bytes()
                with self.assertRaises(e2e.Blocked) as raised:
                    e2e.Receipt(receipt.path, f.driver, f.bound)
                self.assertNotIn("PRIVATE-RECEIPT-SENTINEL", str(raised.exception))
                self.assertEqual(file.read_bytes(), before)

    def test_g2_private_acl_and_reparse_refusals_precede_claim(self):
        for failure in ("acl", "reparse"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                target = f.driver.runtime.target()
                consent = f.authority / "consent.json"
                # Actual path traversal; only Windows lstat metadata or ACL
                # verdict is doubled. No junction or elevated privilege required.
                original_lstat = Path.lstat

                def lstat(path, *args, **kwargs):
                    info = original_lstat(path, *args, **kwargs)
                    return SimpleNamespace(st_mode=info.st_mode, st_file_attributes=0x400) if path == consent else info

                seam = (patch.object(target, "privacy", side_effect=lambda path: path != consent)
                        if failure == "acl" else patch.object(Path, "lstat", lstat))
                with seam, self.assertRaises(e2e.Blocked):
                    self.capability(f)
                self.assertFalse((f.authority / "operation.lock").exists())
                self.assertFalse((f.authority / "claim.json").exists())

    @contextlib.contextmanager
    def real_binding(self, fixture):
        f = fixture
        f.driver.ROOT = Path(__file__).resolve().parents[2]
        f.driver.CONTEXT = "kind-aiforall-local-full"
        f.driver.NAMESPACES = {"namespace": e2e.NAMESPACE, "apparatus_namespace": "aiforall-local-apparatus",
                               "cluster_namespace": "aiforall-local-cluster"}
        f.driver.git_revision = Mock(return_value="c" * 40)
        f.data = {"namespaceUids": {name: "uid-" + name for name in f.driver.NAMESPACES.values()},
                  "task_id": "retention-test-only", "kube_system_uid": "uid-system",
                  "nodeContainerIds": ["3" * 64, "1" * 64, "2" * 64]}
        f.current = copy.deepcopy(f.data)
        f.driver.runtime.target().check = Mock(side_effect=lambda: copy.deepcopy(f.current))
        f.http.pki_root = Mock(return_value=f.inputs["pki_directory"])
        f.epoch = "b" * 64
        # binding remains real, including file fingerprints and target checks.
        # Only the existing coffer_epoch transport seam is replaced, not a new
        # target API or an operator boolean qualifying the private IAM source.
        module = SimpleNamespace(coffer_epoch=lambda target: f.epoch)
        spec = SimpleNamespace(loader=SimpleNamespace(exec_module=lambda module: None))
        with patch.object(e2e.importlib.util, "spec_from_file_location", return_value=spec), patch.object(
                e2e.importlib.util, "module_from_spec", return_value=module):
            yield lambda: e2e.binding(f.driver, f.args, f.data, f.inputs, f.http)

    def test_g3_real_binding_tracks_three_uids_target_epoch_sources_head_and_consent(self):
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            with self.real_binding(f) as bind:
                f.bound = bind()
                self.assertEqual(f.bound["namespace_uids"], f.data["namespaceUids"])
                self.assertEqual(len(f.bound["namespace_uids"]), 3)
                self.assertEqual(f.bound["target_anchor"], {"task_id": "retention-test-only",
                    "kube_system_uid": "uid-system", "nodeContainerIds": [str(n) * 64 for n in (1, 2, 3)],
                    "openbao_epoch_sha256": "b" * 64})
                self.assertEqual(f.bound["git"], "c" * 40)
                self.assertEqual(set(f.bound["source"]), {"local_full_e2e.py", "local-full-acceptance.py",
                    "prepare-local-full-certs.py", "mesh-authn-kind-e2e-cases.sh"})
                self.assertTrue(all(len(value) == 64 for value in f.bound["source"].values()))
                self.assertEqual(f.driver.namespace_owned.call_args_list,
                                 [unittest.mock.call(f.data, name) for name in f.driver.NAMESPACES.values()])
                self.rebind_attempt(f)
                cap = e2e._Retention(f.driver, f.inputs, f.bound, bind)
                with cap.operation():
                    cap.prepare()
                self.assertEqual(json.loads((f.authority / "claim.json").read_bytes()), cap.claim)
                self.assertEqual(cap.binding["consent_sha256"], f.bound["consent_sha256"])

    def test_g3_real_binding_drift_blocks_before_claim_instead_of_recapturing(self):
        for drift in ("namespace", "task", "system", "nodes", "epoch", "head", "source", "consent"):
            with self.subTest(drift=drift), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                with self.real_binding(f) as bind:
                    f.bound = bind()
                    self.rebind_attempt(f)
                    cap = e2e._Retention(f.driver, f.inputs, f.bound, bind)
                    with cap.operation():
                        if drift == "namespace":
                            f.current["namespaceUids"][e2e.NAMESPACE] = "replacement-uid"
                        elif drift == "task":
                            f.current["task_id"] = "another-task"
                        elif drift == "system":
                            f.current["kube_system_uid"] = "replacement-system"
                        elif drift == "nodes":
                            f.current["nodeContainerIds"][0] = "4" * 64
                        elif drift == "epoch":
                            f.epoch = "d" * 64
                        elif drift == "head":
                            f.driver.git_revision.return_value = "d" * 40
                        elif drift == "consent":
                            path = f.authority / "consent.json"
                            path.write_bytes(path.read_bytes() + b" ")
                        original_read = Path.read_bytes

                        def source_read(path):
                            raw = original_read(path)
                            return raw + b"\n# changed source" if drift == "source" and path == Path(e2e.__file__) else raw

                        with patch.object(Path, "read_bytes", source_read), self.assertRaises(e2e.Blocked):
                            cap.prepare()
                    self.assertFalse((f.authority / "claim.json").exists())
                    self.assertFalse(f.inputs["receipt_path"].exists())
                    self.assertEqual(f.http.calls, [])

    def test_g4_cleanup_repairs_only_identical_missing_terminal_and_is_idempotent(self):
        for missing in ("none", "authority", "receipt"):
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                with self.action(f):
                    e2e.run_action(f.driver, f.args, {})
                    f.args.action = "cleanup-probes"
                    first = e2e.run_action(f.driver, f.args, {})["cleanup"]
                    self.assertEqual((first["status"], first["effects_possible"]), ("RETAINED", False))
                    terminal_path = f.authority / "terminal.json"
                    terminal = terminal_path.read_bytes()
                    receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound)
                    event_path = receipt.path / f"{len(receipt.events) - 1:06d}.json"
                    event = event_path.read_bytes()
                    if missing == "authority":
                        terminal_path.unlink()  # simulate crash between the two durable writes
                    elif missing == "receipt":
                        event_path.unlink()
                    repaired = e2e.run_action(f.driver, f.args, {})["cleanup"]
                    self.assertEqual(repaired, first)
                    self.assertEqual(terminal_path.read_bytes(), terminal)
                    self.assertEqual(event_path.read_bytes(), event)
                    before = {p: p.read_bytes() for folder in (f.authority, receipt.path) for p in folder.iterdir()}
                    self.assertEqual(e2e.run_action(f.driver, f.args, {})["cleanup"], first)
                    self.assertEqual({p: p.read_bytes() for folder in (f.authority, receipt.path) for p in folder.iterdir()}, before)
                self.assertEqual(f.http.calls, [])
                self.assertFalse(f.pg.account)

    def test_g4_partial_or_contradictory_terminal_is_blocked_without_rewrite(self):
        for failure in ("partial", "foreign", "contradictory"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                with self.action(f):
                    e2e.run_action(f.driver, f.args, {})
                    f.args.action = "cleanup-probes"
                    e2e.run_action(f.driver, f.args, {})
                    terminal = f.authority / "terminal.json"
                    value = json.loads(terminal.read_bytes())
                    if failure == "partial":
                        terminal.write_bytes(b"{")
                    else:
                        value["run_id" if failure == "foreign" else "effects_possible"] = USER if failure == "foreign" else True
                        terminal.write_text(json.dumps(value), encoding="utf-8")
                    before = {p: p.read_bytes() for folder in (f.authority, f.inputs["receipt_path"]) for p in folder.iterdir()}
                    outcome = e2e.run_action(f.driver, f.args, {})["cleanup"]
                self.assertEqual((outcome["status"], outcome["removed"], outcome["complete"]), ("BLOCKED", 0, False))
                self.assertEqual({p: p.read_bytes() for folder in (f.authority, f.inputs["receipt_path"]) for p in folder.iterdir()}, before)
                self.assertEqual(f.http.calls, [])

    def test_g6_fixed_manifest_accepts_observed_pass_fail_and_rejects_false_started(self):
        # A simple fixed source oracle avoids deriving expectations by calling
        # _required_assertions itself. Other cases remain in the 39-case inventory.
        valid = {"id": "me-no-jwt", "status": "PASS", "attempted": True, "observed": True,
                 "completed": True, "evidence": "LIVE", "http_status": 401,
                 "checked_assertions": ["http-status"], "failed_assertions": []}
        variants = [{"id": "jwks"}, {"id": "smoke"}, {"id": "unknown"}, {"status": "BLOCKED"},
                    {"attempted": 1}, {"observed": False}, {"completed": False}, {"evidence": "NOT RUN"},
                    {"http_status": True}, {"http_status": 99}, {"http_status": 600},
                    {"checked_assertions": ["response-bound"]}, {"checked_assertions": ["http-status", "http-status"]},
                    {"checked_assertions": ["PRIVATE-BODY-SENTINEL"]}, {"failed_assertions": ["http-status"]},
                    {"status": "FAIL"}, {"status": "FAIL", "failed_assertions": ["response-bound"]},
                    {"extra": "PRIVATE-HMAC-SENTINEL"}]
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            cap = self.capability(f)
            with cap.operation():
                cap.prepare()
                receipt = self.prepared_receipt(f)
                cap.consume(receipt)
                receipt.append("case-observed", valid)
                self.assertEqual(e2e.Receipt(receipt.path, f.driver, f.bound).last("case-observed"), valid)
                failed = {**valid, "status": "FAIL", "http_status": 500, "failed_assertions": ["http-status"]}
                receipt.append("case-observed", failed)
                self.assertEqual(receipt.last("case-observed"), failed)
                before = {p.name: p.read_bytes() for p in receipt.path.iterdir()}
                for change in variants:
                    with self.subTest(change=change), self.assertRaises(e2e.Blocked) as raised:
                        receipt.append("case-observed", {**valid, **change})
                    self.assertNotIn("PRIVATE-", str(raised.exception))
                self.assertEqual({p.name: p.read_bytes() for p in receipt.path.iterdir()}, before)
                self.assertEqual(set(e2e.CASE_ASSERTIONS), {case for case, _ in e2e.mesh_cases()} - {"jwks"})
                self.assertEqual((len(e2e.mesh_cases()), len(e2e.APPARATUS_CASES)), (39, 6))

    def test_g6_case_journal_ack_failure_never_attests_started_or_leaks_body(self):
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            with self.action(f):
                e2e.run_action(f.driver, f.args, {})
                f.args.action, f.args.suite = "e2e", "mesh-authn"
                original_append = e2e.Receipt.append

                def append(receipt, kind, data):
                    if kind == "case-observed":
                        raise e2e.Blocked("PRIVATE-OAUTH-BODY-SENTINEL")
                    return original_append(receipt, kind, data)

                def business(mesh):
                    def oracle(case):
                        case.update(attempted=True, observed=True, http_status=401)
                        e2e._check(case, "http-status", lambda: True)
                    mesh.results.case("me-no-jwt", oracle)
                    mesh.results.block_remaining("isolated oracle")

                with patch.object(e2e.Receipt, "append", append), patch.object(e2e.Mesh, "run", autospec=True, side_effect=business):
                    report = e2e.run_action(f.driver, f.args, {})
            self.assertEqual((report["passed"], report["executed"], report["expected"]), (1, 1, 39))
            self.assertEqual(report["launch_status"], "NOT_PROVEN")
            self.assertEqual(report["cleanup"]["status"], "BLOCKED")
            self.assertIn("case-journal", report["cleanup"]["uncertainties"])
            receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound)
            self.assertIsNone(receipt.last("case-observed"))
            self.assertNotIn("PRIVATE-OAUTH-BODY-SENTINEL", json.dumps(report) + json.dumps(receipt.events))

    def test_g5_forward_closes_and_joins_only_owned_handle_on_body_failure(self):
        for body_fails in (False, True):
            with self.subTest(body_fails=body_fails), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                owned, foreign = Mock(name="owned-handle"), Mock(name="foreign-handle")

                @contextlib.contextmanager
                def forward(service, **identity):
                    self.assertEqual((service, identity), ("iam", {"namespace_uid": "namespace-fixture-uid", "service_uid": "service-fixture-uid"}))
                    try:
                        yield 12345
                    finally:
                        owned.close()
                        owned.wait()

                f.driver.runtime.target().e2e_forward = forward
                services = e2e.Services(f.driver, f.args, {"namespaceUids": {e2e.NAMESPACE: "namespace-fixture-uid"}}, f.inputs)
                with patch.object(services, "resource", return_value={"metadata": {"uid": "service-fixture-uid"}}):
                    try:
                        with services.forward("iam") as port:
                            self.assertEqual(port, 12345)
                            if body_fails:
                                raise ValueError("PRIVATE-BUSINESS-SENTINEL")
                    except e2e.Blocked as error:
                        self.assertTrue(body_fails)
                        self.assertNotIn("PRIVATE-BUSINESS-SENTINEL", str(error))
                self.assertEqual(owned.mock_calls, [unittest.mock.call.close(), unittest.mock.call.wait()])
                self.assertEqual(foreign.mock_calls, [])
                self.assertFalse(getattr(services, "technical_cleanup_failed", False))

    def test_g5_body_and_teardown_failures_never_report_technical_pass(self):
        # T-1: a failed business oracle must not hide a failed context teardown.
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            owned = Mock(name="owned-handle")

            @contextlib.contextmanager
            def forward(service, **identity):
                try:
                    yield 12345
                finally:
                    owned.close()
                    owned.wait()
                    raise RuntimeError("PRIVATE-TEARDOWN-SENTINEL")

            f.driver.runtime.target().e2e_forward = forward
            services = e2e.Services(f.driver, f.args, {"namespaceUids": {e2e.NAMESPACE: "namespace-fixture-uid"}}, f.inputs)
            cap = self.capability(f)
            with cap.operation():
                cap.prepare()
                receipt = self.prepared_receipt(f)
                cap.consume(receipt)
                receipt.append("execution-started", {"suite": "mesh-authn"})
                with patch.object(services, "resource", return_value={"metadata": {"uid": "service-fixture-uid"}}):
                    with self.assertRaises(e2e.Blocked) as raised, services.forward("iam"):
                        raise ValueError("PRIVATE-BUSINESS-SENTINEL")
                self.assertNotIn("PRIVATE-", str(raised.exception))
                outcome = e2e._terminalize(cap, receipt, services)
            self.assertEqual(owned.mock_calls, [unittest.mock.call.close(), unittest.mock.call.wait()])
            self.assertEqual((outcome["status"], outcome["technical_status"], outcome["complete"]),
                             ("BLOCKED", "BLOCKED", False), "body failure cannot erase teardown uncertainty")


    # Complements the integrated lock/binding/terminal/teardown tests.
    # Oracles: ratified contract §8.2–8.3; doubles do not prove LIVE.
    def test_g2_authority_readback_failure_spends_slot_without_capability(self):
        for name in ("claim.json", "consumed.json"):
            with self.subTest(record=name), tempfile.TemporaryDirectory() as directory:
                f = self.fixture(Path(directory).resolve())
                cap = e2e._Retention(f.driver, f.inputs, f.bound, lambda: copy.deepcopy(f.bound))
                record = f.authority / name
                original_read = Path.read_bytes

                def uncertain_readback(path):
                    return b"PRIVATE-READBACK-SENTINEL" if path == record else original_read(path)

                with cap.operation():
                    if name == "consumed.json":
                        cap.prepare()
                        receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound, create=True)
                        receipt.append("retention-prepared", copy.deepcopy(e2e.RETENTION_PREPARED))
                    with patch.object(Path, "read_bytes", uncertain_readback), self.assertRaises(e2e.Blocked) as raised:
                        cap.prepare() if name == "claim.json" else cap.consume(receipt)
                    self.assertNotIn("PRIVATE-READBACK-SENTINEL", str(raised.exception))
                    self.assertFalse(cap.active, "no capability before successful authority read-back")
                    if name == "consumed.json":
                        self.assertIsNone(receipt.last("execution-started"))
                        self.assertIsNone(receipt.retention)
                before = record.read_bytes()
                self.assertEqual(json.loads(before), f.claim, "the write succeeded; only its read-back ACK failed")
                with self.action(f), self.assertRaises(e2e.Blocked):
                    e2e.run_action(f.driver, f.args, {})
                self.assertEqual(record.read_bytes(), before, "uncertain ACK must not rearm or replace the record")
                self.assertEqual(f.http.calls, [])
                self.assertFalse(f.pg.account)

    def test_g6_subject_manifest_requires_both_status_subject_and_response_json(self):
        # me-sub must check both the response body and its authenticated subject;
        # evaluating one assertion is not a complete PASS for this business case.
        valid = {"id": "me-sub", "status": "PASS", "attempted": True, "observed": True,
                 "completed": True, "evidence": "LIVE", "http_status": 200,
                 "checked_assertions": ["response-json", "status-subject"], "failed_assertions": []}
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            cap = e2e._Retention(f.driver, f.inputs, f.bound, lambda: copy.deepcopy(f.bound))
            with cap.operation():
                cap.prepare()
                receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound, create=True)
                receipt.append("retention-prepared", copy.deepcopy(e2e.RETENTION_PREPARED))
                cap.consume(receipt)
                receipt.append("case-observed", valid)
                self.assertEqual(e2e.Receipt(receipt.path, f.driver, f.bound).last("case-observed"), valid)
                before = {p.name: p.read_bytes() for p in receipt.path.iterdir()}
                for missing in ("response-json", "status-subject"):
                    with self.subTest(missing=missing), self.assertRaises(e2e.Blocked):
                        receipt.append("case-observed", {**valid, "checked_assertions": [
                            value for value in valid["checked_assertions"] if value != missing]})
                self.assertEqual({p.name: p.read_bytes() for p in receipt.path.iterdir()}, before)
                self.assertEqual(receipt.last("case-observed"), valid)

    def test_g6_response_bound_only_failure_never_attests_required_business_start(self):
        # G-6: a real Services.request under a doubled connection can fail its
        # safety bound before me-no-jwt's required HTTP-status oracle runs.
        # That must not be promoted to a business launch merely because HTTP
        # was observed. All authority files and bodies are synthetic/temp-only.
        with tempfile.TemporaryDirectory() as directory:
            f = self.fixture(Path(directory).resolve())
            f.driver.runtime.target().check = Mock(return_value={})
            connection = Mock()
            response = connection.getresponse.return_value
            response.status = 401
            response.read.return_value = b"PRIVATE-MAIL-HMAC-SENTINEL" + b"x" * e2e.LIMIT
            services_class = e2e.Services  # action() doubles construction, not this transport under test.

            def business(mesh):
                transport = services_class(f.driver, f.args, {}, f.inputs)
                transport.retention = mesh.receipt.retention

                def oracle(case):
                    status, _ = transport.request("GET", "/iam/api/me", case=case)
                    e2e._check(case, "http-status", lambda: status == 401)

                mesh.results.case("me-no-jwt", oracle)
                mesh.results.block_remaining("isolated response-bound negative control")

            with self.action(f):
                e2e.run_action(f.driver, f.args, {})
                f.args.action, f.args.suite = "e2e", "mesh-authn"
                with patch.object(e2e.http.client, "HTTPConnection", return_value=connection), patch.object(
                        e2e.Mesh, "run", autospec=True, side_effect=business):
                    report = e2e.run_action(f.driver, f.args, {})
            connection.request.assert_called_once_with("GET", "/iam/api/me", None, {})
            connection.close.assert_called_once_with()
            response.read.assert_called_once_with(e2e.LIMIT + 1)
            case = next(value for value in report["cases"] if value["id"] == "me-no-jwt")
            self.assertEqual((case["checked_assertions"], case["failed_assertions"]),
                             (["response-bound"], ["response-bound"]))
            self.assertNotIn("http-status", case["checked_assertions"], "required business oracle was never evaluated")
            self.assertEqual((report["expected"], len(report["cases"]), len(e2e.APPARATUS_CASES)), (39, 39, 6))
            receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound)
            self.assertNotIn("PRIVATE-MAIL-HMAC-SENTINEL", json.dumps(report) + json.dumps(receipt.events))
            self.assertEqual(report["launch_status"], "NOT_PROVEN",
                             "response size gate alone cannot attest a required business oracle")
            self.assertIsNone(receipt.last("case-observed"))


class TransportTests(unittest.TestCase):
    def services(self):
        anchor = SimpleNamespace(check=Mock())
        driver = SimpleNamespace(runtime=SimpleNamespace(target=lambda: anchor))
        return e2e.Services(driver, SimpleNamespace(), {}, {}), anchor

    def test_public_connection_rechecks_anchor_no_redirect_or_proxy(self):
        services, anchor = self.services()
        connection = Mock()
        connection.getresponse.return_value.status = 302
        connection.getresponse.return_value.read.return_value = b"redacted"
        with patch.object(e2e.http.client, "HTTPConnection", return_value=connection) as constructor:
            for _ in range(2):
                self.assertEqual(services.request("GET", "/iam/api/me")[0], 302)
        self.assertEqual(anchor.check.call_count, 2)
        self.assertEqual(constructor.call_count, 2)
        self.assertEqual(constructor.call_args.args, ("127.0.0.1", 18080))
        connection.getresponse.return_value.read.assert_called_with(e2e.LIMIT + 1)
        self.assertEqual(connection.close.call_count, 2)

    def test_transport_timeout_is_attempted_not_completed_and_redacted(self):
        services, _ = self.services()
        connection = Mock()
        connection.getresponse.side_effect = TimeoutError("SECRET")
        result = e2e.Results("mesh-authn")
        with patch.object(e2e.http.client, "HTTPConnection", return_value=connection):
            result.case("me-no-jwt", lambda case: services.request("GET", "/iam/api/me", case=case))
        report = result.report()
        self.assertEqual((report["attempted"], report["executed"], report["blocked"]), (1, 0, 39))
        self.assertNotIn("SECRET", json.dumps(report))
        connection.close.assert_called_once()

    def test_oversized_body_fails_without_persisting_content(self):
        services, _ = self.services()
        connection = Mock()
        connection.getresponse.return_value.status = 200
        connection.getresponse.return_value.read.return_value = b"S" * (e2e.LIMIT + 1)
        with patch.object(e2e.http.client, "HTTPConnection", return_value=connection):
            with self.assertRaises(AssertionError):
                services.request("GET", "/iam/api/me")
        connection.close.assert_called_once()

    def test_missing_forward_never_uses_an_alternate_transport(self):
        services, _ = self.services()
        with patch.object(e2e.http.client, "HTTPConnection") as connection:
            with self.assertRaisesRegex(e2e.Blocked, "e2e_forward"):
                services.request("GET", "/telegraph/api/notifications", service="telegraph")
        connection.assert_not_called()

    def test_unknown_identity_refused_before_opening_any_key(self):
        services, _ = self.services()
        with patch.object(e2e.ssl, "create_default_context") as tls:
            with self.assertRaises(e2e.Blocked):
                services.pki("ca")
        tls.assert_not_called()

    def test_existing_writable_attribute_leafs_use_read_only_pki_guard_not_seal(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve() / "certificates"
            mesh = root / "mesh"
            mesh.mkdir(parents=True)
            files = {}
            for name in ("ca.crt", "mesh-client.crt", "mesh-client.key", "envoy-mesh.crt", "envoy-mesh.key"):
                path = mesh / name
                path.write_text("PUBLIC-FIXTURE" if name.endswith("crt") else "FAKE-KEY-FOR-UNIT-ONLY")
                info = path.stat()
                attributes = getattr(info, "st_file_attributes", 0)
                self.assertFalse(attributes & 1)  # Windows FILE_ATTRIBUTE_READONLY is absent.
                files[path] = (info.st_mode, attributes, path.read_bytes())
            driver = SimpleNamespace(ROOT=Path(__file__).parents[2], runtime=SimpleNamespace(
                target=lambda: SimpleNamespace(privacy=Mock(return_value=False))))
            guard = e2e.certificate_guard(driver)
            contexts = [Mock(), Mock()]
            services = e2e.Services(driver, SimpleNamespace(environment_data={"public_ca_path": str(mesh / "ca.crt")}), {}, {"pki_directory": root})
            def decode(path):
                return {"subjectAltName": (("DNS", Path(path).stem),)}
            with patch.object(e2e, "certificate_guard", return_value=guard), patch.object(
                    guard.subprocess, "run", return_value=SimpleNamespace(returncode=0)) as process, patch.object(
                    e2e.ssl._ssl, "_test_decode_cert", side_effect=decode), patch.object(
                    e2e.ssl, "create_default_context", side_effect=contexts):
                self.assertIs(services.pki("mesh-client"), contexts[0])
                self.assertIs(services.pki("envoy-mesh"), contexts[1])
            if guard.os.name == "nt":
                self.assertEqual(process.call_count, 1)
                self.assertEqual(process.call_args.kwargs["env"]["AIFORALL_CERT_ACL_CREATE"], "0")
            for context, identity in zip(contexts, ("mesh-client", "envoy-mesh")):
                context.load_cert_chain.assert_called_once_with(str(mesh / (identity + ".crt")), str(mesh / (identity + ".key")))
            for path, before in files.items():
                self.assertEqual((path.stat().st_mode, getattr(path.stat(), "st_file_attributes", 0), path.read_bytes()), before)
            self.assertFalse(any(path.name == "ca.key" for path in files))

    def test_authority_inputs_still_require_sealed_files(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory).resolve() / "inputs.json"
            path.write_text('{}')
            with self.assertRaises(e2e.Blocked):
                e2e.private_path(str(path), SimpleNamespace(privacy=lambda p: False))

    def test_pki_foreign_acl_and_bad_san_rejected_without_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / "mesh").mkdir()
            for name in ("ca.crt", "mesh-client.crt", "mesh-client.key"):
                (root / "mesh" / name).write_text("FAKE")
            args = SimpleNamespace(environment_data={"public_ca_path": str(root / "mesh/ca.crt")})
            guard = SimpleNamespace(checked_path=lambda p: p, private_directory=Mock(side_effect=ValueError("foreign ACL")))
            with patch.object(e2e, "certificate_guard", return_value=guard), patch.object(e2e.ssl, "create_default_context") as tls:
                with self.assertRaises(ValueError):
                    e2e.Services(SimpleNamespace(), args, {}, {"pki_directory": root}).pki("mesh-client")
            tls.assert_not_called()
            guard.private_directory.assert_called_once_with(root, create=False)
            guard.private_directory.side_effect = None
            with patch.object(e2e, "certificate_guard", return_value=guard), patch.object(
                    e2e.ssl._ssl, "_test_decode_cert", return_value={"subjectAltName": (("DNS", "envoy-mesh"),)}), patch.object(
                    e2e.ssl, "create_default_context") as tls:
                with self.assertRaises(e2e.Blocked):
                    e2e.Services(SimpleNamespace(), args, {}, {"pki_directory": root}).pki("mesh-client")
            tls.assert_not_called()


class ProviderProjectionTests(unittest.TestCase):
    """G-2/3: standard snapshot provenance and nonsecret C projection.

    The production source was read to use real seams, not to derive the oracle:
    the GitHub-only mission briefing supplies the profile and confidentiality rules.
    """

    def fixture(self, projection="CONNECTORS 1 1 0 1 0 0", *, retention=False):
        pod = {"metadata": {"name": "iam-test", "uid": "iam-pod", "ownerReferences": [
            {"kind": "ReplicaSet", "name": "iam-rs", "uid": "iam-rs-uid"}]},
            "spec": {"containers": [{"name": "iam", "env": [{"name": "RUST_ENVIRONMENT", "value": "development"}],
                       "volumeMounts": [{"name": "config", "mountPath": "/app/config/development.toml", "subPath": "development.toml", "readOnly": True}]}],
                     "volumes": [{"name": "config", "secret": {"secretName": "owned-config-reference-only"}}]},
            "status": {"phase": "Running", "containerStatuses": [{"name": "iam", "restartCount": 0, "imageID": "sha256:fixture"}]}}
        if retention:
            # Reuse the real, in-memory deployment helper. No Secret contents,
            # lease, runtime, source-qualification issuer or new harness.
            bao.snapshot(pod["spec"], "iam-hmac", "iam-openbao", "local-full-bootstrap-scripts-fixture")
            pod["status"]["initContainerStatuses"] = [{"name": "openbao-snapshot", "restartCount": 0,
                "state": {"terminated": {"exitCode": 0, "reason": "Completed"}}}]
        raw = projection + "\n" if retention else (
            "a" * 64 + "  /app/config/development.toml\n" + projection + "\n" + "a" * 64 + "  /app/config/development.toml")
        def kubectl(*args, **kwargs):
            return json.dumps({"items": [pod]}) if args[0] == "get" else raw
        driver = SimpleNamespace(namespace_owned=Mock(), owned=Mock(return_value={"metadata": {"uid": "iam-deployment"}}),
            obj=Mock(side_effect=lambda kind, *args: {"metadata": {"uid": "iam-rs-uid", "ownerReferences": [
                {"kind": "Deployment", "uid": "iam-deployment"}]}} if kind == "ReplicaSet" else copy.deepcopy(pod)),
            runtime=SimpleNamespace(kubectl=Mock(side_effect=kubectl)))
        data = {"namespaceUids": {e2e.NAMESPACE: "namespace-uid"}}
        inputs = {"version": 2, "cleanup_policy": dict(RetentionContractTests.POLICY)} if retention else {}
        return e2e.Services(driver, SimpleNamespace(), data, inputs), driver, pod

    def test_gh_g2_standard_openbao_snapshot_admits_github_only_without_private_hash(self):
        services, driver, pod = self.fixture("CONNECTORS 1 1 1 0 0 0", retention=True)
        self.assertEqual(pod["spec"]["serviceAccountName"], "iam-openbao")
        self.assertEqual(pod["spec"]["initContainers"][0]["image"], bao.PYTHON)
        self.assertEqual(pod["spec"]["initContainers"][0]["command"],
                         ["python", "-B", "/scripts/openbao-workload.py", "iam-hmac"])
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            proof = services.iam_providers()
        self.assertEqual(proof, {"namespace_uid": "namespace-uid", "deployment_uid": "iam-deployment",
            "replica_set_uid": "iam-rs-uid", "pod_uid": "iam-pod", "providers": ["github"]})
        self.assertEqual((stdout.getvalue(), stderr.getvalue()), ("", ""))
        commands = [call.args for call in driver.runtime.kubectl.call_args_list]
        self.assertEqual([command[0] for command in commands], ["get", "exec"])
        self.assertEqual(commands[-1][:-1], ("exec", "-n", e2e.NAMESPACE, "iam-test", "-c", "iam", "--", "sh", "-ec"))
        self.assertFalse(any("secret" in str(command).lower() for command in commands))
        script = commands[-1][-1]
        self.assertIn("CONNECTORS", script)
        self.assertNotIn("cat ", script)
        # CLI source guard only: private hashes may be compared inside the Pod,
        # never emitted as the old standalone stdout lines. No shell is started.
        self.assertNotIn("\nsha256sum /app/config/development.toml\n", script)

    def test_gh_g2_snapshot_writer_accepts_omitted_readonly_false(self):
        services, driver, pod = self.fixture("CONNECTORS 1 1 1 0 0 0", retention=True)
        writer = pod["spec"]["initContainers"][0]["volumeMounts"][4]
        self.assertEqual(writer, {"name": "private-config", "mountPath": "/runtime", "readOnly": False})
        del writer["readOnly"]
        proof = services.iam_providers()
        self.assertEqual(proof, {"namespace_uid": "namespace-uid", "deployment_uid": "iam-deployment",
            "replica_set_uid": "iam-rs-uid", "pod_uid": "iam-pod", "providers": ["github"]})
        self.assertEqual([call.args[0] for call in driver.runtime.kubectl.call_args_list], ["get", "exec"])

    def test_gh_g2_snapshot_readonly_writer_refused_before_exec(self):
        services, driver, pod = self.fixture("CONNECTORS 1 1 1 0 0 0", retention=True)
        pod["spec"]["initContainers"][0]["volumeMounts"][4]["readOnly"] = True
        with self.assertRaises(e2e.Blocked):
            services.iam_providers()
        self.assertEqual([call.args[0] for call in driver.runtime.kubectl.call_args_list], ["get"])

    def test_g1_real_snapshot_preflight_precedes_claim_and_journals_no_private_hash(self):
        # Retained from the independent author's late, otherwise duplicate delta.
        with tempfile.TemporaryDirectory() as directory:
            f = RetentionContractTests().fixture(Path(directory).resolve())
            services, driver, _ = self.fixture("CONNECTORS 1 1 1 0 0 0", retention=True)
            f.http.iam_providers = services.iam_providers
            with RetentionContractTests().action(f):
                report = e2e.run_action(f.driver, f.args, {})
            self.assertTrue(report["prepared"], report)
            self.assertEqual([call.args[0] for call in driver.runtime.kubectl.call_args_list], ["get", "exec"])
            self.assertEqual(f.http.calls, [])
            self.assertEqual(json.loads((f.authority / "claim.json").read_text(encoding="utf-8")), f.claim)
            receipt = e2e.Receipt(f.inputs["receipt_path"], f.driver, f.bound)
            self.assertEqual(receipt.last("iam-provider-registry"), {
                "namespace_uid": "namespace-uid", "deployment_uid": "iam-deployment",
                "replica_set_uid": "iam-rs-uid", "pod_uid": "iam-pod", "providers": ["github"]})
            self.assertNotIn("config_sha256", json.dumps(receipt.events))

    def test_gh_g2_unqualified_snapshot_variants_refused_before_claim_and_exec(self):
        mutations = {
            "direct-source-not-C": lambda p: p["spec"]["containers"][0]["volumeMounts"][0].update(name="config"),
            "arbitrary-source": lambda p: p["spec"]["volumes"][0].update(secret=None, hostPath={"path": "/foreign"}),
            "ambiguous-source": lambda p: p["spec"]["volumes"][0].update(configMap={"name": "foreign"}),
            "arbitrary-emptydir": lambda p: p["spec"].update(initContainers=[]),
            "wrong-service-account": lambda p: p["spec"].update(serviceAccountName="other"),
            "automatic-token": lambda p: p["spec"].update(automountServiceAccountToken=True),
            "wrong-audience": lambda p: p["spec"]["volumes"][1]["projected"]["sources"][0]["serviceAccountToken"].update(audience="other"),
            "wrong-role": lambda p: p["spec"]["initContainers"][0]["command"].__setitem__(-1, "github-seed"),
            "wrong-command": lambda p: p["spec"]["initContainers"][0].update(command=["sh", "-c", "true"]),
            "extra-args": lambda p: p["spec"]["initContainers"][0].update(args=["other"]),
            "wrong-image": lambda p: p["spec"]["initContainers"][0].update(image="python:latest"),
            "missing-init-status": lambda p: p["status"].update(initContainerStatuses=[]),
            "failed-init": lambda p: p["status"]["initContainerStatuses"][0]["state"]["terminated"].update(exitCode=1),
            "unfinished-init": lambda p: p["status"]["initContainerStatuses"][0].update(state={"running": {}}),
            "nonmemory-output": lambda p: p["spec"]["volumes"][2]["emptyDir"].update(medium=""),
            "oversized-output": lambda p: p["spec"]["volumes"][2]["emptyDir"].update(sizeLimit="2Mi"),
            "writable-template": lambda p: p["spec"]["initContainers"][0]["volumeMounts"][3].update(readOnly=False),
            "wrong-template-mount": lambda p: p["spec"]["initContainers"][0]["volumeMounts"][3].update(mountPath="/elsewhere"),
            "wrong-output-mount": lambda p: p["spec"]["initContainers"][0]["volumeMounts"][4].update(mountPath="/elsewhere"),
            "writable-app": lambda p: p["spec"]["containers"][0]["volumeMounts"][0].update(readOnly=False),
            "wrong-app-subpath": lambda p: p["spec"]["containers"][0]["volumeMounts"][0].update(subPath="other.toml"),
            "second-writer": lambda p: p["spec"]["containers"].append({"name": "other", "volumeMounts": [
                {"name": "private-config", "mountPath": "/other", "readOnly": False}]}),
            "second-init-writer": lambda p: p["spec"]["initContainers"].append({"name": "other", "volumeMounts": [
                {"name": "private-config", "mountPath": "/other", "readOnly": False}]}),
        }
        for name, mutate in mutations.items():
            with self.subTest(profile=name), tempfile.TemporaryDirectory() as directory:
                f = RetentionContractTests().fixture(Path(directory).resolve())
                services, driver, pod = self.fixture("CONNECTORS 1 1 1 0 0 0", retention=True)
                mutate(pod)
                f.http.iam_providers = services.iam_providers
                with RetentionContractTests().action(f), self.assertRaises(e2e.Blocked):
                    e2e.run_action(f.driver, f.args, {})
                self.assertFalse((f.authority / "claim.json").exists())
                self.assertFalse(f.inputs["receipt_path"].exists())
                self.assertEqual(f.http.calls, [])
                self.assertEqual(f.pg.calls, [])
                self.assertTrue(all(call.args[0] == "get" for call in driver.runtime.kubectl.call_args_list))

    def test_gh_g3_malformed_or_private_projection_never_validates_or_claims(self):
        for raw in ("", "CONNECTORS 1 1 1 0 0 0\nCONNECTORS 1 1 1 0 0 0", "CONNECTORS 1 1 1 0 0 0 extra",
                    "CONNECTORS 1 2 1 0 0 0", "CONNECTORS 2 2 1 1 0 0", "CONNECTORS 1 1 0 0 1 0",
                    "CONNECTORS 1 1 1 0 0 1", "CONNECTORS 0 0 0 0 0 0", "CONNECTORS 1 1 0 1 0 0",
                    "a" * 64 + "  /app/config/development.toml\nCONNECTORS 1 1 1 0 0 0\n" + "a" * 64 + "  /app/config/development.toml"):
            with self.subTest(response_kind=raw.splitlines()[0][:30] if raw else "empty"), tempfile.TemporaryDirectory() as directory:
                f = RetentionContractTests().fixture(Path(directory).resolve())
                services, driver, _ = self.fixture(raw, retention=True)
                f.http.iam_providers = services.iam_providers
                stdout, stderr = io.StringIO(), io.StringIO()
                with RetentionContractTests().action(f), contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                    with self.assertRaises(e2e.Blocked) as failure:
                        e2e.run_action(f.driver, f.args, {})
                self.assertEqual((stdout.getvalue(), stderr.getvalue()), ("", ""))
                self.assertNotIn("a" * 64, "".join(traceback.format_exception(failure.exception)))
                self.assertFalse((f.authority / "claim.json").exists())
                self.assertFalse(f.inputs["receipt_path"].exists())
                self.assertEqual(f.http.calls, [])
                self.assertEqual(f.pg.calls, [])
                self.assertEqual(sum(call.args[0] == "exec" for call in driver.runtime.kubectl.call_args_list), 1,
                                 "response rejection must reach the same projection as the valid snapshot")

    def test_gh_g3_snapshot_pod_change_during_projection_never_validates(self):
        services, driver, _ = self.fixture("CONNECTORS 1 1 1 0 0 0", retention=True)
        original = driver.obj.side_effect
        observations = []
        def obj(kind, *args):
            value = original(kind, *args)
            if kind == "Pod":
                observations.append(value)
                if len(observations) == 2:
                    value["metadata"]["uid"] = "replacement-pod"
            return value
        driver.obj.side_effect = obj
        with self.assertRaises(e2e.Blocked):
            services.iam_providers()
        self.assertEqual(len(observations), 2, "replacement must be tested across the executed projection")
        self.assertEqual(sum(call.args[0] == "exec" for call in driver.runtime.kubectl.call_args_list), 1)

    def test_gh_g3_cli_failures_redact_sentinels_before_diagnostics_and_exception(self):
        """G-3: existing CLI runner with a fake process, not an alternate client."""
        deploy = Path(__file__).resolve().parents[1] / "deploy"
        diagnostic_spec = importlib.util.spec_from_file_location("private_diagnostics", deploy / "private_diagnostics.py")
        diagnostics = importlib.util.module_from_spec(diagnostic_spec)
        diagnostic_spec.loader.exec_module(diagnostics)
        target_spec = importlib.util.spec_from_file_location("local_full_target_e2e_unit", deploy / "local_full_target.py")
        target = importlib.util.module_from_spec(target_spec)
        # dataclasses resolves annotations against the defining module.
        with patch.dict(sys.modules, {target_spec.name: target}):
            target_spec.loader.exec_module(target)
        sentinels = ("OAUTH_PRIVATE_SYNTHETIC_SENTINEL", "HMAC_PRIVATE_SYNTHETIC_SENTINEL", "a" * 64)
        for fault in ("success", "nonzero", "timeout"):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as directory:
                services, driver, _ = self.fixture("CONNECTORS 1 1 1 0 0 0", retention=True)
                original = driver.runtime.kubectl.side_effect
                def kubectl(*args, **kwargs):
                    if args[0] == "get":
                        return original(*args, **kwargs)
                    return target.run(["kubectl", *args], text=True, **kwargs)
                driver.runtime.kubectl.side_effect = kubectl
                payload = " ".join(sentinels).encode()
                if fault == "timeout":
                    result = target.subprocess.TimeoutExpired(["kubectl"], 20, output=payload, stderr=payload)
                elif fault == "success":
                    result = target.subprocess.CompletedProcess(["kubectl"], 0, b"CONNECTORS 1 1 1 0 0 0\n", b"")
                else:
                    result = target.subprocess.CompletedProcess(["kubectl"], 1, payload, payload)
                stdout, stderr = io.StringIO(), io.StringIO()
                with patch.dict(sys.modules, {"private_diagnostics": diagnostics}), patch.object(
                        target.subprocess, "run", side_effect=result if isinstance(result, BaseException) else None,
                        return_value=result) as process, patch.object(diagnostics, "_directory", return_value=Path(directory)), diagnostics.operation(
                        "github-projection", Path(directory)), contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                    scope = diagnostics.CURRENT.get()
                    if fault == "success":
                        proof = services.iam_providers()
                        self.assertEqual(proof, {"namespace_uid": "namespace-uid", "deployment_uid": "iam-deployment",
                            "replica_set_uid": "iam-rs-uid", "pod_uid": "iam-pod", "providers": ["github"]})
                    else:
                        with self.assertRaises(e2e.Blocked) as failure:
                            services.iam_providers()
                    self.assertIs(diagnostics.CURRENT.get(), scope, "projection must restore the outer diagnostics scope")
                captured = b"".join(path.read_bytes() for path in Path(directory).iterdir() if path.is_file())
                formatted = "" if fault == "success" else "".join(traceback.format_exception(failure.exception))
                for sentinel in sentinels:
                    self.assertNotIn(sentinel, stdout.getvalue() + stderr.getvalue() + formatted)
                    self.assertNotIn(sentinel.encode(), captured)
                if fault != "success":
                    self.assertIsNone(failure.exception.__cause__)
                    self.assertIsNone(failure.exception.__context__)
                process.assert_called_once()  # No pre-projection refusal may masquerade as redaction.

    def test_current_gitlab_projection_bound_to_owned_pod_and_hash(self):
        services, driver, _ = self.fixture()
        proof = services.iam_providers()
        self.assertEqual(proof, {"namespace_uid": "namespace-uid", "deployment_uid": "iam-deployment",
            "replica_set_uid": "iam-rs-uid", "pod_uid": "iam-pod", "config_sha256": "a" * 64, "providers": ["gitlab"]})
        commands = [call.args for call in driver.runtime.kubectl.call_args_list]
        self.assertFalse(any("secret" in str(c).lower() for c in commands))
        self.assertEqual(commands[-1][-1], e2e.IAM_PROVIDER_SCRIPT)
        self.assertIn("sha256sum /app/config/development.toml", commands[-1][-1])
        self.assertNotIn("cat ", commands[-1][-1])

    def test_ambiguous_projection_or_changed_hash_refused(self):
        for projection in ("CONNECTORS 1 2 1 1 0 0", "CONNECTORS 1 1 1 0 1 0", "CONNECTORS 1 1 1 0 0 1"):
            services, _, _ = self.fixture(projection)
            with self.assertRaises(e2e.Blocked):
                services.iam_providers()
        services, driver, _ = self.fixture()
        original = driver.runtime.kubectl.side_effect
        driver.runtime.kubectl.side_effect = lambda *a, **k: original(*a, **k).replace("a" * 64, "b" * 64, 1) if a[0] == "exec" else original(*a, **k)
        with self.assertRaises(e2e.Blocked):
            services.iam_providers()

    def test_effective_override_or_foreign_owner_blocks_before_exec(self):
        for mutation in (lambda p: p["spec"]["containers"][0]["env"].append({"name": "IAM_IDP__CONNECTORS", "value": "redacted"}),
                         lambda p: p["spec"]["containers"][0].update(envFrom=[{"secretRef": {"name": "foreign"}}]),
                         lambda p: p["metadata"]["ownerReferences"][0].update(uid="replaced-rs")):
            services, driver, pod = self.fixture()
            mutation(pod)
            with self.assertRaises(e2e.Blocked):
                services.iam_providers()
            self.assertTrue(all(c.args[0] == "get" for c in driver.runtime.kubectl.call_args_list))

    def test_pod_replaced_during_projection_blocks(self):
        services, driver, pod = self.fixture()
        counter = [0]
        original = driver.obj.side_effect
        def obj(kind, *args):
            value = original(kind, *args)
            if kind == "Pod":
                counter[0] += 1
                if counter[0] == 2:
                    value["metadata"]["uid"] = "foreign-replacement"
            return value
        driver.obj.side_effect = obj
        with self.assertRaises(e2e.Blocked):
            services.iam_providers()


if __name__ == "__main__":
    unittest.main()
