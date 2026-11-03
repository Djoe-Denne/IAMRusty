"""Pure protocol/PG/provenance doubles: no sockets, subprocess, runtime or Cargo."""
import base64
import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import Mock, patch
from urllib.parse import parse_qs, urlsplit
import uuid

SPEC = importlib.util.spec_from_file_location("local_full_e2e_unit", Path(__file__).with_name("local_full_e2e.py"))
e2e = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(e2e)
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
    def fixture(self, projection="CONNECTORS 1 1 0 1 0 0"):
        pod = {"metadata": {"name": "iam-test", "uid": "iam-pod", "ownerReferences": [
            {"kind": "ReplicaSet", "name": "iam-rs", "uid": "iam-rs-uid"}]},
            "spec": {"containers": [{"name": "iam", "env": [{"name": "RUST_ENVIRONMENT", "value": "development"}],
                       "volumeMounts": [{"name": "config", "mountPath": "/app/config/development.toml", "subPath": "development.toml", "readOnly": True}]}],
                     "volumes": [{"name": "config", "secret": {"secretName": "owned-config-reference-only"}}]},
            "status": {"phase": "Running", "containerStatuses": [{"name": "iam", "restartCount": 0, "imageID": "sha256:fixture"}]}}
        raw = "a" * 64 + "  /app/config/development.toml\n" + projection + "\n" + "a" * 64 + "  /app/config/development.toml"
        def kubectl(*args, **kwargs):
            return json.dumps({"items": [pod]}) if args[0] == "get" else raw
        driver = SimpleNamespace(namespace_owned=Mock(), owned=Mock(return_value={"metadata": {"uid": "iam-deployment"}}),
            obj=Mock(side_effect=lambda kind, *args: {"metadata": {"uid": "iam-rs-uid", "ownerReferences": [
                {"kind": "Deployment", "uid": "iam-deployment"}]}} if kind == "ReplicaSet" else copy.deepcopy(pod)),
            runtime=SimpleNamespace(kubectl=Mock(side_effect=kubectl)))
        data = {"namespaceUids": {e2e.NAMESPACE: "namespace-uid"}}
        return e2e.Services(driver, SimpleNamespace(), data, {}), driver, pod

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
