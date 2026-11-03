"""Bounded adapters for the existing mesh39/Apparatus6 oracles, not a deployer.

The parent supplies private inputs and owns the runtime. No legacy driver, image
build, free-form SQL, URL, command, expected status or Kubernetes deletion lives
on this interface. Missing capabilities block individual obligations, never pass.
"""
import base64
from contextlib import contextmanager
import hashlib
import html
from email import policy
from email.parser import Parser
import http.client
import importlib.util
import json
from pathlib import Path
import re
import ssl
import time
from urllib.parse import parse_qs, quote, urlencode, urlsplit
import uuid

NAMESPACE = "aiforall-local-full"
ISSUER = "http://127.0.0.1:8080/iam"
IDENTITIES = ("mesh-client", "envoy-mesh", "iam-service", "hive-service",
              "telegraph-service", "manifesto-service")
INTERNAL = {"iam": (8443, "iam.aiforall-local-full.svc.cluster.local"),
            "hive": (8443, "hive.aiforall-local-full.svc.cluster.local"),
            "telegraph": (8443, "telegraph.aiforall-local-full.svc.cluster.local"),
            "mailhog": (8025, None)}
LIMIT = 1_048_576
NOTIFICATION_CLEANUP_GAP = (
    "Telegraph notification/delivery inverse unqualified: no delete API or source-event/consumer completion fence; signup forbidden")

# Fixed projection of the one mounted startup profile. No config/key contents or
# arbitrary connector IDs leave the Pod. Noncanonical/ambiguous TOML is rejected.
IAM_PROVIDER_AWK = r'''
function finish() { if (inside && ids_in_table != 1) bad=1; inside=0 }
{
    line=$0; gsub(/^[ \t]+|[ \t\r]+$/, "", line)
    if (index(line,sprintf("%c%c%c",34,34,34)) || index(line,sprintf("%c%c%c",39,39,39))) bad=1
    if (line ~ /^\[\[idp\.connectors\]\]([ \t]*#.*)?$/) {
        finish(); inside=1; ids_in_table=0; tables++; next
    }
    if (line ~ /^\[/) { finish(); next }
    if (line ~ /^connectors[ \t]*=/ && line !~ /^connectors[ \t]*=[ \t]*\[\][ \t]*(#.*)?$/) bad=1
    if (inside && line ~ /^id[ \t]*=/) {
        ids++; ids_in_table++
        if (line ~ /^id[ \t]*=[ \t]*"github"[ \t]*(#.*)?$/) github++
        else if (line ~ /^id[ \t]*=[ \t]*"gitlab"[ \t]*(#.*)?$/) gitlab++
        else unknown++
    }
}
END { finish(); printf "CONNECTORS %d %d %d %d %d %d\n",tables,ids,github,gitlab,unknown,bad }
'''
IAM_PROVIDER_SCRIPT = ("set -eu\n"
    'test "$RUST_ENVIRONMENT" = development\n'
    "test ! -e /app/config/local.toml\n"
    "sha256sum /app/config/development.toml\n"
    "awk '" + IAM_PROVIDER_AWK + "' /app/config/development.toml\n"
    "sha256sum /app/config/development.toml\n")


class Blocked(ValueError):
    """A named, nonsecret prerequisite is unavailable; no fabricated evidence."""


def mesh_cases():
    cases = [(name, positive) for name, positive in (
        ("signup", True), ("completion", True), ("login", True), ("jwks", True),
        ("me-no-jwt", False), ("me-sub", True), ("me-bearer-spoof", True),
        ("refresh-persisted", True), ("me-spoof-no-jwt", False),
        ("notifications-sub", True), ("telegraph-bypass", False), ("hive-bypass", False))]
    for operation in ("token", "revoke"):
        cases.append((f"{operation}-gateway-no-principal", False))
        cases.extend((f"{operation}-mesh-{mode}", False)
                     for mode in ("bearer", "spoof", "internal", "all"))
        cases.extend((f"{operation}-{identity}", False)
                     for identity in ("iam-service", "hive-service", "telegraph-service", "manifesto-service"))
        cases.extend((f"{operation}-gate-{mode}", False) for mode in ("missing", "wrong"))
    cases.extend((("s2s-token", True), ("s2s-revoke", True), ("s2s-after-revoke", False),
                  ("organization", True), ("membership-issuer", True)))
    return cases


APPARATUS_CASES = (("h1-signed-envelope", True), ("h2-persisted-scheduled", True),
                   ("h3-no-consent-zero-hop", False), ("h3-invoke-hostname", True),
                   ("h4-live-canary", True), ("h4-plugin-connect-timeout", False))


class Results:
    def __init__(self, suite):
        inventory = mesh_cases() if suite == "mesh-authn" else APPARATUS_CASES
        self.cases = {name: {"id": name, "positive": positive, "status": "NOT RUN",
                             "attempted": False, "completed": False, "evidence": "NOT RUN"} for name, positive in inventory}

    def case(self, name, operation):
        case = self.cases[name]
        try:
            # attempted means an HTTP/oracle operation started, not that transport succeeded.
            operation(case)
        except Blocked as error:
            case.update(status="BLOCKED", reason=str(error))
        except Exception:
            case.update(status="FAIL", completed=case.get("observed", False),
                        evidence="LIVE" if case.get("observed") else "NOT RUN",
                        reason="oracle failed; response and credentials redacted")
        else:
            case.update(status="PASS", completed=True, evidence="LIVE")
        return case["status"] == "PASS"

    def block_remaining(self, reason):
        for case in self.cases.values():
            if case["status"] == "NOT RUN":
                case.update(status="BLOCKED", reason=reason)

    def report(self):
        values = list(self.cases.values())
        counts = {status: sum(c["status"] == status for c in values)
                  for status in ("PASS", "FAIL", "BLOCKED", "NOT RUN")}
        return {"expected": len(values), "executed": sum(c["completed"] for c in values),
                "passed": counts["PASS"], "failed": counts["FAIL"],
                "blocked": counts["BLOCKED"] + counts["NOT RUN"],
                "attempted": sum(c["attempted"] for c in values), "cases": values}


def canonical_uuid(value):
    try:
        parsed = uuid.UUID(value)
    except (ValueError, TypeError, AttributeError) as error:
        raise Blocked("canonical UUID required") from error
    if parsed.version != 4 or str(parsed) != value:
        raise Blocked("canonical UUIDv4 required")
    return value


def private_path(value, anchor, *, directory=False):
    path = Path(value)
    if not path.is_absolute() or path.is_symlink() or path.resolve() != path:
        raise Blocked("canonical absolute private path required")
    if directory:
        # Read-only marker qualifies owner and ACLs of its parent directory too.
        return path
    if not path.is_file() or path.stat().st_size > LIMIT or not anchor.privacy(path):
        raise Blocked("owner-only read-only private file required")
    return path


def certificate_guard(driver):
    """Reuse the PKI owner's read-only ACL/reparse guard, NOT authority-file sealing.

    private_directory(create=False) only inspects metadata. It does not require
    the Windows ReadOnly attribute and never changes existing ACLs/attributes.
    """
    path = driver.ROOT / "ops/scripts/prepare-local-full-certs.py"
    spec = importlib.util.spec_from_file_location("local_full_e2e_pki_guard", path)
    guard = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(guard)
    return guard


def assert_run_open(receipt):
    if any(e["kind"] == "cleanup" and e["data"].get("complete") is True for e in receipt.events):
        raise Blocked("run terminal after cleanup; no E2E replay or new mutation permitted")


def read_inputs(path, driver):
    anchor = driver.runtime.target()
    path = private_path(str(path), anchor)
    value = driver.strict_json(path)
    required = {"version", "run_id", "suite", "provider_mode", "credentials_path", "pki_directory", "receipt_path"}
    if not isinstance(value, dict) or set(value) != required or type(value["version"]) is not int or value["version"] != 1:
        raise Blocked("closed E2E input v1 schema required")
    canonical_uuid(value["run_id"])
    if value["suite"] not in {"mesh-authn", "apparatus"} or value["provider_mode"] != "SIMULATED_PROVIDER":
        raise Blocked("declared suite and SIMULATED_PROVIDER required")
    value["credentials_path"] = private_path(value["credentials_path"], anchor)
    value["pki_directory"] = private_path(value["pki_directory"], anchor, directory=True)
    value["receipt_path"] = private_path(value["receipt_path"], anchor, directory=True)
    return value


class Receipt:
    """Append-only owner-only events. An intent without its ACK is never adopted."""
    def __init__(self, path, driver, binding, create=False):
        self.path, self.driver, self.binding = path, driver, binding
        self.events = []
        if create:
            path.mkdir()  # no exist_ok: never adopt a previous run
            driver.anchor_module.secure(path, directory=True)
            self.append("binding", binding)
        else:
            if not path.is_dir() or path.is_symlink():
                raise Blocked("prepared private receipt required")
            files = sorted(path.glob("*.json"))
            if len(files) > 256:
                raise Blocked("receipt event count exceeds bound")
            for index, file in enumerate(files):
                private_path(str(file), driver.runtime.target())
                event = driver.strict_json(file)
                if file.name != f"{index:06d}.json" or set(event) != {"kind", "data", "previous"} or event["previous"] != self.digest():
                    raise Blocked("receipt event chain invalid")
                self.events.append(event)
            if not self.events or self.events[0] != {"kind": "binding", "data": binding, "previous": ""}:
                raise Blocked("receipt source/run/target binding changed")

    def digest(self):
        return hashlib.sha256(json.dumps(self.events[-1], sort_keys=True).encode()).hexdigest() if self.events else ""

    def append(self, kind, data):
        if kind in {"execution-started", "signup-intent", "provider-intent", "organization-intent",
                    "account-created", "provider-created", "organization-created"}:
            assert_run_open(self)
        # Freeze the data: subsequent updates to a caller's dict must not mutate
        # the in-memory predecessor hash after its immutable file was persisted.
        event = {"kind": kind, "data": json.loads(json.dumps(data)), "previous": self.digest()}
        file = self.path / f"{len(self.events):06d}.json"
        with file.open("x", encoding="utf-8") as stream:
            json.dump(event, stream, sort_keys=True)
        self.driver.anchor_module.secure(file)
        self.events.append(event)

    def last(self, kind):
        return next((e["data"] for e in reversed(self.events) if e["kind"] == kind), None)


class TlsLoopback(http.client.HTTPConnection):
    def __init__(self, port, context, hostname):
        super().__init__("127.0.0.1", port, timeout=20)
        self.context, self.hostname = context, hostname

    def connect(self):
        super().connect()
        self.sock = self.context.wrap_socket(self.sock, server_hostname=self.hostname)


def response_json(response):
    status, raw = response
    try:
        value = json.loads(raw)
    except (ValueError, UnicodeError) as error:
        raise AssertionError("JSON response required; redacted") from error
    if not isinstance(value, dict):
        raise AssertionError("object response required; redacted")
    return status, value


class Services:
    def __init__(self, driver, args, data, inputs):
        self.driver, self.args, self.data, self.inputs = driver, args, data, inputs
        self.contexts = {}
        self._pki_root = None
        self._pki_guard = None

    def resource(self, kind, name):
        return self.driver.owned(self.data, kind, name, NAMESPACE)

    def pki_root(self):
        if self._pki_root is None:
            guard = certificate_guard(self.driver)
            root = guard.checked_path(self.inputs["pki_directory"])
            if root != self.inputs["pki_directory"] or not root.is_dir():
                raise Blocked("canonical existing PKI directory required")
            guard.private_directory(root, create=False)
            self._pki_root, self._pki_guard = root, guard
        return self._pki_root

    def pki(self, identity):
        if identity not in IDENTITIES:
            raise Blocked("fixed mesh identity required")
        if identity in self.contexts:
            return self.contexts[identity]
        root = self.pki_root() / "mesh"
        paths = [self._pki_guard.checked_path(root / name) for name in
                 ("ca.crt", identity + ".crt", identity + ".key")]
        if any(not p.is_file() or p.stat().st_size > LIMIT for p in paths):
            raise Blocked("bounded existing private PKI files required")
        ca, cert, key = paths
        public_ca = self.args.environment_data.get("public_ca_path")
        if not public_ca or hashlib.sha256(ca.read_bytes()).digest() != hashlib.sha256(Path(public_ca).read_bytes()).digest():
            raise Blocked("mesh CA does not match the parent environment")
        try:
            decoded = ssl._ssl._test_decode_cert(str(cert))
            if ("DNS", identity) not in decoded.get("subjectAltName", ()):
                raise Blocked("leaf SAN differs from its fixed oracle identity")
            context = ssl.create_default_context(cafile=str(ca))
            context.load_cert_chain(str(cert), str(key))  # read-only; never copy or return PEM
        except (OSError, ValueError) as error:
            raise Blocked("fixed leaf TLS context unavailable; details redacted") from error
        self.contexts[identity] = context
        return context

    def iam_providers(self):
        """Nonsecret effective-profile observation tied to current owned IAM Pod.

        The sealed inputs are not provider registration authority. Reject envFrom,
        IdP/config overrides, custom startup commands and an unqualified mount.
        """
        driver = self.driver
        try:
            driver.namespace_owned(self.data, NAMESPACE)
            deployment = self.resource("Deployment", "iam")
            pods = json.loads(driver.runtime.kubectl("get", "pods", "-n", NAMESPACE,
                "-l", "app.kubernetes.io/name=iam", "-o", "json"))["items"]
            running = [p for p in pods if p.get("status", {}).get("phase") == "Running"]
            if len(running) != 1:
                raise Blocked("one current IAM Pod required for provider qualification")
            pod = running[0]
            metadata = pod["metadata"]
            owners = metadata.get("ownerReferences", [])
            if len(owners) != 1 or owners[0].get("kind") != "ReplicaSet":
                raise Blocked("IAM Pod owner chain unavailable")
            rs = driver.obj("ReplicaSet", owners[0]["name"], NAMESPACE)
            if rs["metadata"]["uid"] != owners[0]["uid"] or not any(
                    o.get("kind") == "Deployment" and o.get("uid") == deployment["metadata"]["uid"]
                    for o in rs["metadata"].get("ownerReferences", [])):
                raise Blocked("IAM Pod is not owned by the leased Deployment")
            containers = [c for c in pod["spec"]["containers"] if c.get("name") == "iam"]
            if len(containers) != 1:
                raise Blocked("fixed IAM container unavailable")
            container = containers[0]
            environment = container.get("env", [])
            if (container.get("envFrom") or container.get("command") or container.get("args")
                    or any("IDP" in e.get("name", "").upper() or "CONFIG" in e.get("name", "").upper() for e in environment)
                    or [e.get("value") for e in environment if e.get("name") == "RUST_ENVIRONMENT"] != ["development"]):
                raise Blocked("IAM startup profile/env overrides are not qualified")
            mounts = [m for m in container.get("volumeMounts", []) if m.get("mountPath") == "/app/config/development.toml"]
            if len(mounts) != 1 or mounts[0].get("subPath") != "development.toml" or mounts[0].get("readOnly") is not True:
                raise Blocked("fixed read-only IAM startup config mount required")
            volumes = [v for v in pod["spec"].get("volumes", []) if v.get("name") == mounts[0]["name"]]
            if len(volumes) != 1 or not (volumes[0].get("secret") or volumes[0].get("configMap")):
                raise Blocked("IAM startup config source reference unavailable")
            before = driver.obj("Pod", metadata["name"], NAMESPACE)
            if before != pod:
                raise Blocked("IAM Pod changed before provider projection")
            raw = driver.runtime.kubectl("exec", "-n", NAMESPACE, metadata["name"], "-c", "iam", "--",
                                         "sh", "-ec", IAM_PROVIDER_SCRIPT, timeout=20)
            match = re.fullmatch(r"([a-f0-9]{64})  /app/config/development\.toml\nCONNECTORS ([0-9]+) ([0-9]+) ([0-9]+) ([0-9]+) ([0-9]+) ([0-9]+)\n\1  /app/config/development\.toml", raw.strip())
            if not match:
                raise Blocked("IAM config hash/projection malformed or changed")
            tables, ids, github, gitlab, unknown, bad = map(int, match.groups()[1:])
            if bad or unknown or tables != ids or ids != github + gitlab or github > 1 or gitlab > 1:
                raise Blocked("IAM provider projection ambiguous or unsupported")
            after = driver.obj("Pod", metadata["name"], NAMESPACE)
            current_deployment = self.resource("Deployment", "iam")
            if before != after or current_deployment["metadata"]["uid"] != deployment["metadata"]["uid"]:
                raise Blocked("IAM Pod/incarnation changed during provider projection")
            return {"namespace_uid": self.data["namespaceUids"][NAMESPACE], "deployment_uid": deployment["metadata"]["uid"],
                    "replica_set_uid": rs["metadata"]["uid"], "pod_uid": metadata["uid"], "config_sha256": match[1],
                    "providers": [name for name, present in (("github", github), ("gitlab", gitlab)) if present]}
        except Blocked:
            raise
        except Exception as error:
            raise Blocked("guarded effective IAM provider qualification unavailable; details redacted") from error

    @contextmanager
    def forward(self, service):
        if service not in INTERNAL:
            raise Blocked("fixed E2E forward target required")
        anchor = self.driver.runtime.target()
        capability = getattr(anchor, "e2e_forward", None)
        if not callable(capability):
            raise Blocked("parent e2e_forward capability unavailable: " + service)
        try:
            resource = self.resource("Service", service)
        except Exception as error:
            raise Blocked("fixed forward service UID unavailable: " + service) from error
        # The specialized parent capability selects/attests the exact backing Pod,
        # validates the fixed remote port and owns/closes its subprocess.
        try:
            with capability(service, namespace_uid=self.data["namespaceUids"][NAMESPACE],
                            service_uid=resource["metadata"]["uid"]) as port:
                if type(port) is not int or not 1 <= port <= 65535:
                    raise Blocked("invalid guarded forward port")
                yield port
        except (ValueError, RuntimeError) as error:
            raise Blocked("guarded forward identity/lifecycle unavailable: " + service) from error

    def request(self, method, path, body=None, headers=None, *, service=None, identity="mesh-client", case=None):
        if not path.startswith(("/iam/", "/hive/", "/telegraph/", "/manifesto/", "/api/v1/", "/api/v2/")) or any(c in path for c in "\r\n#\\"):
            raise Blocked("fixed service path required")
        try:
            self.driver.runtime.target().check()
        except Exception as error:
            raise Blocked("target/lease revalidation failed before HTTP") from error
        payload = json.dumps(body).encode() if body is not None else None
        headers = dict(headers or {})
        if payload is not None:
            headers["Content-Type"] = "application/json"

        def perform(connection):
            try:
                if case is not None:
                    case["attempted"] = True
                connection.request(method, path, payload, headers)
                response = connection.getresponse()
                raw = response.read(LIMIT + 1)
                if case is not None:
                    case["observed"] = True
                if len(raw) > LIMIT:
                    raise AssertionError("response exceeds bound; redacted")
                # No redirect handling, proxy environment, cookies or HTTP logging.
                return response.status, raw
            finally:
                connection.close()

        try:
            if service is None:
                return perform(http.client.HTTPConnection("127.0.0.1", 18080, timeout=20))
            with self.forward(service) as port:
                hostname = INTERNAL[service][1]
                connection = (TlsLoopback(port, self.pki(identity), hostname) if hostname else
                              http.client.HTTPConnection("127.0.0.1", port, timeout=20))
                return perform(connection)
        except Blocked:
            raise
        except (OSError, http.client.HTTPException) as error:
            raise Blocked("HTTP transport unavailable; details redacted") from error


# Every database statement below is fixed source. Only data is bound via psql's
# quoted variable syntax / PREPARE; no credentials, SQL or command comes from inputs.
FK_QUERY = """SELECT coalesce(jsonb_agg(jsonb_build_array(c.conrelid::regclass::text,
 a.attname,c.confdeltype,cardinality(c.conkey),b.attname,cardinality(c.confkey)) ORDER BY c.conrelid::regclass::text),'[]')
 FROM pg_constraint c JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=c.conkey[1]
 JOIN pg_attribute b ON b.attrelid=c.confrelid AND b.attnum=c.confkey[1]
 WHERE c.contype='f' AND c.confrelid='users'::regclass"""
EXPECTED_FKS = [[table, column, "c", 1, "id", 1] for table, column in (
    ("identities", "user_id"), ("oauth_transactions", "target_user_id"),
    ("password_reset_tokens", "user_id"), ("provider_tokens", "user_id"),
    ("refresh_tokens", "user_id"), ("user_emails", "user_id"))]


def snapshot_sql():
    # Hash rows in PostgreSQL, so password/session/verification material never
    # leaves the server. These PK+fingerprint ACKs constrain the eventual inverse.
    parts = ["'users',(SELECT coalesce(jsonb_agg(jsonb_build_object('id',id,'sha',encode(sha256(convert_to(row_to_json(t)::text,'UTF8')),'hex')) ORDER BY id),'[]') FROM users t WHERE id=($1->>'user')::uuid)"]
    for table in ("user_emails", "identities", "refresh_tokens", "provider_tokens", "password_reset_tokens", "oauth_transactions", "user_email_verification"):
        predicate = ("email=$1->>'email'" if table == "user_email_verification" else
                     ("target_user_id" if table == "oauth_transactions" else "user_id") + "=($1->>'user')::uuid")
        parts.append(f"'{table}',(SELECT coalesce(jsonb_agg(jsonb_build_object('id',id,'sha',encode(sha256(convert_to(row_to_json(t)::text,'UTF8')),'hex')) ORDER BY id),'[]') FROM {table} t WHERE {predicate})")
    return "SELECT jsonb_build_object(" + ",".join(parts) + ")"


SNAPSHOT_SQL = snapshot_sql()
DB_SQL = {
    "fks": FK_QUERY,
    "absence": "SELECT jsonb_build_object('absent', NOT EXISTS(SELECT 1 FROM user_emails WHERE email=$1->>'email') AND NOT EXISTS(SELECT 1 FROM users WHERE username=$1->>'username') AND NOT EXISTS(SELECT 1 FROM user_email_verification WHERE email=$1->>'email'))",
    "snapshot": SNAPSHOT_SQL,
    "account-owned": """SELECT jsonb_build_object('owned',EXISTS(SELECT 1 FROM users u JOIN user_emails e ON e.user_id=u.id
 WHERE u.id=($1->>'user')::uuid AND e.email=$1->>'email' AND (u.username IS NULL OR u.username=$1->>'username'))
 AND (SELECT count(*) FROM user_emails WHERE user_id=($1->>'user')::uuid)=1
 AND (SELECT count(*) FROM identities WHERE user_id=($1->>'user')::uuid)<=1
 AND NOT EXISTS(SELECT 1 FROM identities WHERE user_id=($1->>'user')::uuid AND
 (issuer<>'http://127.0.0.1:8080/iam' OR subject<>$1->>'user' OR kind<>'platform')))""",
    "refresh-ids": "SELECT coalesce(jsonb_agg(id ORDER BY id),'[]') FROM refresh_tokens WHERE user_id=($1->>'user')::uuid AND token IN (SELECT jsonb_array_elements_text($1->'hashes'))",
    "refresh": "SELECT jsonb_build_object('count',count(*)) FROM refresh_tokens WHERE user_id=($1->>'user')::uuid AND token=$1->>'hash' AND is_valid=true AND expires_at>NOW()",
    "provider-insert": """INSERT INTO provider_tokens AS p(user_id,provider,provider_user_id,access_token,expires_in)
 SELECT ($1->>'user')::uuid,'github',$1->>'nonce',$1->>'token',3600
 WHERE NOT EXISTS(SELECT 1 FROM provider_tokens WHERE (user_id=($1->>'user')::uuid AND provider='github') OR (provider='github' AND provider_user_id=$1->>'nonce'))
 RETURNING jsonb_build_object('id',id,'sha',encode(sha256(convert_to(row_to_json(p)::text,'UTF8')),'hex'))""",
    "provider-read": """SELECT jsonb_build_object('count',count(*),'matches',count(*) FILTER (WHERE id=($1->>'id')::integer
 AND provider_user_id=$1->>'nonce' AND access_token=$1->>'token'
 AND encode(sha256(convert_to(row_to_json(provider_tokens)::text,'UTF8')),'hex')=$1->>'sha')) FROM provider_tokens
 WHERE user_id=($1->>'user')::uuid AND provider='github'""",
    "provider-delete": """DELETE FROM provider_tokens AS p WHERE id=($1->>'id')::integer AND user_id=($1->>'user')::uuid
 AND provider='github' AND provider_user_id=$1->>'nonce' AND access_token=$1->>'token'
 AND encode(sha256(convert_to(row_to_json(p)::text,'UTF8')),'hex')=$1->>'sha'
 RETURNING jsonb_build_object('id',id,'sha',encode(sha256(convert_to(row_to_json(p)::text,'UTF8')),'hex'))""",
    "provider-state": """SELECT jsonb_build_object('exists',EXISTS(SELECT 1 FROM provider_tokens WHERE id=($1->>'id')::integer),
 'matches',EXISTS(SELECT 1 FROM provider_tokens WHERE id=($1->>'id')::integer AND user_id=($1->>'user')::uuid
 AND provider='github' AND provider_user_id=$1->>'nonce' AND access_token=$1->>'token'
 AND encode(sha256(convert_to(row_to_json(provider_tokens)::text,'UTF8')),'hex')=$1->>'sha'))""",
}

# The initial IAM schema has six incoming users FKs, all CASCADE. We explicitly
# remove ACKed child PKs first, under locks, so a cascade never deletes an unseen
# child. Unknown nested FK or user trigger makes qualification fail closed.
ACCOUNT_TABLES = ("users", "user_emails", "identities", "refresh_tokens", "provider_tokens",
                  "password_reset_tokens", "oauth_transactions", "user_email_verification")
SAFE_GRAPH = """NOT EXISTS (SELECT 1 FROM pg_constraint WHERE contype='f' AND confrelid IN
 ('user_emails'::regclass,'identities'::regclass,'refresh_tokens'::regclass,'provider_tokens'::regclass,
  'password_reset_tokens'::regclass,'oauth_transactions'::regclass,'user_email_verification'::regclass))
 AND NOT EXISTS (SELECT 1 FROM pg_trigger WHERE NOT tgisinternal AND tgrelid IN
 ('users'::regclass,'user_emails'::regclass,'identities'::regclass,'refresh_tokens'::regclass,
  'provider_tokens'::regclass,'password_reset_tokens'::regclass,'oauth_transactions'::regclass,'user_email_verification'::regclass))"""
DB_SQL["cleanup-ready"] = "SELECT jsonb_build_object('safe'," + SAFE_GRAPH + ")"


def account_delete_sql():
    guard = ("WITH observed AS (" + SNAPSHOT_SQL + "), guard AS MATERIALIZED (SELECT 1/(CASE WHEN "
             "observed.jsonb_build_object=$1->'snapshot' AND EXISTS(SELECT 1 FROM user_emails WHERE "
             "user_id=($1->>'user')::uuid AND email=$1->>'email') AND (" + SAFE_GRAPH + ") AND (" + FK_QUERY +
             ")=$1->'fks' THEN 1 ELSE 0 END) AS ok FROM observed)")
    deletes = []
    for table in ("user_emails", "identities", "refresh_tokens", "user_email_verification"):
        deletes.append(f",d_{table} AS (DELETE FROM {table} WHERE id::text IN "
                       f"(SELECT e->>'id' FROM jsonb_array_elements($1->'snapshot'->'{table}') e) "
                       "AND (SELECT ok FROM guard)=1 RETURNING id)")
    # Assert provider rows have already been removed by the exact provider inverse;
    # no OAuth/password-reset creation belongs to this password-only campaign.
    predicate = " AND ".join(f"jsonb_array_length($1->'snapshot'->'{t}')=0" for t in
                              ("provider_tokens", "oauth_transactions", "password_reset_tokens"))
    return (guard + "".join(deletes) + " DELETE FROM users WHERE id=($1->>'user')::uuid "
            "AND (SELECT ok FROM guard)=1 AND " + predicate +
            " RETURNING jsonb_build_object('id',id)")


DB_SQL["account-delete"] = account_delete_sql()
DB_SQL["cleanup-compile"] = DB_SQL["account-delete"]


class Database:
    def __init__(self, driver, data):
        self.driver, self.data = driver, data

    def execute(self, operation, values):
        try:
            return self._execute(operation, values)
        except Blocked:
            raise
        except Exception as error:
            raise Blocked("PostgreSQL target/provenance unavailable; details redacted") from error

    def _execute(self, operation, values):
        if operation not in DB_SQL:
            raise Blocked("fixed database operation required")
        canonical_uuid(values["run"])
        if "user" in values:
            canonical_uuid(values["user"])
        if operation.startswith("provider-"):
            canonical_uuid(values["nonce"])
            if values["nonce"] != values["run"] or values["token"] != "simulated-provider-" + values["run"]:
                raise Blocked("provider fixture nonce/content mismatch")
            if operation != "provider-insert" and (type(values["id"]) is not int or values["id"] < 1):
                raise Blocked("provider ACK integer PK required")
            if operation != "provider-insert" and not re.fullmatch(r"[a-f0-9]{64}", values.get("sha", "")):
                raise Blocked("complete provider row fingerprint ACK required")
        deployment = self.driver.owned(self.data, "Deployment", "postgres", NAMESPACE)
        pods = json.loads(self.driver.runtime.kubectl("get", "pods", "-n", NAMESPACE,
                           "-l", "app.kubernetes.io/name=postgres", "-o", "json"))["items"]
        running = [p for p in pods if p.get("status", {}).get("phase") == "Running"]
        if len(running) != 1:
            raise Blocked("one attested PostgreSQL pod required")
        pod = running[0]
        owners = pod["metadata"].get("ownerReferences", [])
        if len(owners) != 1 or owners[0]["kind"] != "ReplicaSet":
            raise Blocked("PostgreSQL owner chain unavailable")
        rs = self.driver.obj("ReplicaSet", owners[0]["name"], NAMESPACE)
        if rs["metadata"]["uid"] != owners[0]["uid"] or not any(
                o["uid"] == deployment["metadata"]["uid"] and o["kind"] == "Deployment"
                for o in rs["metadata"].get("ownerReferences", [])):
            raise Blocked("PostgreSQL owner chain changed")
        if self.driver.obj("Pod", pod["metadata"]["name"], NAMESPACE)["metadata"]["uid"] != pod["metadata"]["uid"]:
            raise Blocked("PostgreSQL pod replaced")
        encoded = base64.b64encode(json.dumps(values).encode()).decode()
        sql = DB_SQL[operation]
        # Read-only queries without a parameter still use the same closed data binding.
        statement = "PREPARE local_e2e(jsonb) AS " + sql + ";\n"
        locks = ("LOCK TABLE " + ",".join(ACCOUNT_TABLES) + " IN SHARE ROW EXCLUSIVE MODE;\n"
                 if operation == "account-delete" else "LOCK TABLE provider_tokens IN SHARE ROW EXCLUSIVE MODE;\n"
                 if operation in {"provider-insert", "provider-delete"} else "")
        execution = ("SELECT jsonb_build_object('prepared',true);" if operation == "cleanup-compile" else
                     "EXECUTE local_e2e(convert_from(decode('" + encoded + "','base64'),'UTF8')::jsonb);")
        script = "\\set QUIET on\nBEGIN;\n" + locks + statement + execution + "\nCOMMIT;\n"
        try:
            raw = self.driver.runtime.kubectl("exec", "-i", "-n", NAMESPACE, pod["metadata"]["name"], "--",
                    "psql", "-X", "-q", "-A", "-t", "-v", "ON_ERROR_STOP=1", "-U", "postgres", "-d", "iam_dev",
                    stdin=script, timeout=30)
            return json.loads(raw) if raw else None
        except Exception as error:
            raise Blocked("fixed PostgreSQL operation failed; details redacted") from error

    def qualify(self, values):
        if self.execute("fks", values) != EXPECTED_FKS:
            raise Blocked("IAM cleanup foreign-key graph differs from qualified source")

    def qualify_account_cleanup(self, values):
        self.qualify(values)
        if self.execute("cleanup-ready", values) != {"safe": True}:
            raise Blocked("IAM cleanup nested FK/trigger graph is not qualified")
        if self.execute("cleanup-compile", values) != {"prepared": True}:
            raise Blocked("IAM cleanup statement did not prepare before signup")

    def qualify_notifications_cleanup(self, values):
        # C-3: Telegraph's API only lists/marks read (http/src/lib.rs:30-49).
        # communication_factory.rs:169 sets id=None; notification_write.rs:49,
        # 96-105 generates fresh notification/delivery PKs asynchronously. The
        # schema has no source-event ID column or exposed consumer completion
        # fence. A bounded quiet interval/user-filtered SQL delete cannot prove
        # that a late/replayed event will not create unreceipted rows afterward.
        # No SQL/audit/outbox purge or new product protocol is authorized here.
        raise Blocked(NOTIFICATION_CLEANUP_GAP)

    def cleanup_account(self, values, expected):
        self.qualify_account_cleanup(values)
        if not isinstance(expected, dict) or set(expected) != set(ACCOUNT_TABLES) or len(expected["users"]) != 1 or expected["users"][0]["id"] != values["user"]:
            raise Blocked("exact account snapshot ACK required")
        if any(expected[t] for t in ("provider_tokens", "oauth_transactions", "password_reset_tokens")):
            raise Blocked("unexpected dependent IAM rows; cleanup refused")
        if self.execute("account-owned", values) != {"owned": True}:
            raise Blocked("account email/user/username creation binding changed")
        if self.execute("account-delete", {**values, "snapshot": expected, "fks": EXPECTED_FKS}) != {"id": values["user"]}:
            raise Blocked("account inverse ACK missing; no retry/adoption")


def token_fields(raw):
    try:
        header, payload, signature = raw.split(".")
        decode = lambda part: json.loads(base64.urlsafe_b64decode(part + "=" * (-len(part) % 4)))
        h, p = decode(header), decode(payload)
        canonical_uuid(p["sub"])
        if not signature or p["iss"] != ISSUER or not h.get("kid"):
            raise ValueError()
        return h, p
    except Exception as error:
        raise AssertionError("issued JWT shape invalid; redacted") from error


def require(condition):
    if not condition:
        raise AssertionError("oracle mismatch; response redacted")


class Mesh:
    def __init__(self, services, database, receipt, credentials, inputs, results):
        self.http, self.db, self.receipt, self.credentials = services, database, receipt, credentials
        self.inputs, self.results = inputs, results
        run = inputs["run_id"]
        self.values = {"run": run, "email": "local-full-" + run + "@example.com", "username": "lf" + run.replace("-", "")}
        self.token = self.refresh = self.registration = None
        self.claims = self.header = None
        self.provider = None
        self.provider_attempted = False

    def request(self, case, method, path, body=None, headers=None, **kwargs):
        return response_json(self.http.request(method, path, body, headers, case=case, **kwargs))

    def auth(self):
        if not self.token:
            raise Blocked("real IAM login unavailable")
        return {"authorization": "Bearer " + self.token}

    def capture(self):
        if "user" in self.values:
            require(self.db.execute("account-owned", self.values) == {"owned": True})
            snapshot = self.db.execute("snapshot", self.values)
            require(len(snapshot["users"]) == len(snapshot["user_emails"]) == 1)
            require(not snapshot["password_reset_tokens"] and not snapshot["oauth_transactions"])
            allowed = self.receipt.last("refresh-hashes") or []
            ids = self.db.execute("refresh-ids", {**self.values, "hashes": allowed})
            require(sorted(ids) == sorted(r["id"] for r in snapshot["refresh_tokens"]))
            provider_rows = snapshot["provider_tokens"]
            require(provider_rows == ([] if not self.provider or self.receipt.last("provider-revoked") else
                                      [{"id": self.provider["id"], "sha": self.provider["sha"]}]))
            self.receipt.append("account-snapshot", snapshot)

    def signup(self, case):
        # This gate is BEFORE the first mutating HTTP, not a best-effort cleanup.
        assert_run_open(self.receipt)
        if self.inputs["suite"] == "mesh-authn":
            registry = self.http.iam_providers()
            self.receipt.append("iam-provider-registry", registry)
            if "github" not in registry["providers"]:
                raise Blocked("effective IAM connector github absent; mesh39 expectations unchanged; signup forbidden")
        self.db.qualify_notifications_cleanup(self.values)
        self.db.qualify(self.values)
        self.db.qualify_account_cleanup(self.values)
        if self.db.execute("absence", self.values) != {"absent": True}:
            raise Blocked("run email/username/verification already present; no adoption")
        # A safe verification channel is required before creating the account.
        status, body = response_json(self.http.request("GET", "/api/v2/search?" + urlencode(
            {"kind": "to", "query": self.values["email"], "start": 0, "limit": 20}), service="mailhog"))
        if status != 200 or body.get("total") != 0:
            raise Blocked("safe empty run-recipient verification channel unavailable")
        self.receipt.append("signup-intent", self.values)
        status, body = self.request(case, "POST", "/iam/api/auth/signup",
                                    {"email": self.values["email"], "password": self.credentials["password"]})
        require(200 <= status < 300)
        user = canonical_uuid(body["user"]["id"])
        require(body["user"]["email"] == self.values["email"])
        self.values["user"] = user
        self.receipt.append("account-created", dict(self.values))
        self.registration = body.get("registration_token")
        require(isinstance(self.registration, str) and bool(self.registration))
        self.capture()

    def complete(self, case):
        if not self.registration:
            raise Blocked("signup registration token unavailable")
        status, body = self.request(case, "POST", "/iam/api/auth/complete-registration",
                                    {"registration_token": self.registration, "username": self.values["username"]})
        require(200 <= status < 300 and body["user"]["id"] == self.values["user"])
        require(not body.get("access_token") and not body.get("refresh_token"))
        self.capture()

    def verify_mail(self):
        if "user" not in self.values:
            raise Blocked("owned signup unavailable for verification")
        deadline = time.monotonic() + 45
        while True:
            status, body = response_json(self.http.request("GET", "/api/v2/search?" + urlencode(
                {"kind": "to", "query": self.values["email"], "start": 0, "limit": 20}), service="mailhog"))
            require(status == 200 and type(body.get("total")) is int and body["total"] <= 20)
            matches = []
            for message in body.get("items", []):
                recipients = [p.get("Mailbox", "") + "@" + p.get("Domain", "") for p in message.get("To", [])]
                if recipients != [self.values["email"]]:
                    continue
                raw = message.get("Raw", {}).get("Data")
                texts = []
                if isinstance(raw, str):
                    parsed = Parser(policy=policy.default).parsestr(raw)
                    for part in parsed.walk():
                        if part.get_content_type() in {"text/plain", "text/html"} and not part.is_multipart():
                            texts.append(part.get_content())
                else:
                    texts.append(message.get("Content", {}).get("Body", ""))
                    texts.extend(p.get("Body", "") for p in message.get("MIME", {}).get("Parts", []))
                for link in re.findall(r"https?[^\s<>\"]+", html.unescape(" ".join(texts))):
                    query = parse_qs(urlsplit(link).query)
                    if query.get("email") == [self.values["email"]] and len(query.get("token", [])) == 1:
                        matches.append((message["ID"], query["token"][0]))
            if matches:
                ids = {m[0] for m in matches}
                tokens = {m[1] for m in matches}
                require(len(ids) == len(tokens) == 1)
                self.receipt.append("mail-message", {"id": next(iter(ids)), "email": self.values["email"]})
                status, _ = response_json(self.http.request("GET", "/iam/api/auth/verify?" + urlencode(
                    {"email": self.values["email"], "token": next(iter(tokens))})))
                require(status == 200)
                self.capture()
                return
            if time.monotonic() >= deadline:
                raise Blocked("run-recipient verification email unavailable")
            time.sleep(1)

    def login(self, case):
        self.verify_mail()
        status, body = self.request(case, "POST", "/iam/api/auth/login",
                                    {"email": self.values["email"], "password": self.credentials["password"]})
        require(200 <= status < 300)
        self.token, self.refresh = body["access_token"], body["refresh_token"]
        require(isinstance(self.token, str) and bool(self.token) and isinstance(self.refresh, str) and bool(self.refresh))
        self.header, self.claims = token_fields(self.token)
        require(self.claims["sub"] == self.values["user"])
        self.receipt.append("refresh-hashes", [hashlib.sha256(self.refresh.encode()).hexdigest()])
        self.capture()

    def jwks(self, case):
        status, body = self.request(case, "GET", "/iam/.well-known/jwks.json")
        if self.header is None:
            raise Blocked("issued JWT kid unavailable")
        require(200 <= status < 300 and any(k.get("kid") == self.header["kid"] and k.get("n") for k in body["keys"]))

    def me(self, case, mode):
        spoof = {"x-principal-iss": ISSUER, "x-principal-sub": "00000000-0000-0000-0000-000000000099", "x-principal-org": "attacker-org"}
        headers = self.auth() if mode in {"sub", "bearer-spoof"} else {}
        if "spoof" in mode:
            headers.update(spoof)
        status, raw = self.http.request("GET", "/iam/api/me", headers=headers, case=case)
        body = response_json((status, raw))[1] if mode != "no-jwt" else {}
        if mode in {"sub", "bearer-spoof"}:
            require(status == 200 and body["id"] == self.claims["sub"])
        else:
            require(status in {401, 403})
            if mode == "spoof-no-jwt":
                require("access_token" not in body)

    def rotate(self, case):
        if not self.refresh:
            raise Blocked("login-issued refresh token unavailable")
        status, body = self.request(case, "POST", "/iam/api/token/refresh", {"refresh_token": self.refresh})
        replacement = body.get("refresh_token")
        require(200 <= status < 300 and isinstance(replacement, str) and replacement and replacement != self.refresh)
        self.refresh = replacement
        self.receipt.append("refresh-hashes", [*(self.receipt.last("refresh-hashes") or []), hashlib.sha256(replacement.encode()).hexdigest()])
        require(self.db.execute("refresh", {**self.values, "hash": hashlib.sha256(replacement.encode()).hexdigest()}) == {"count": 1})
        self.capture()

    def notifications(self, case):
        status, body = self.request(case, "GET", "/telegraph/api/notifications", headers=self.auth())
        require(status == 200 and body["user_id"] == self.claims["sub"])

    def bypass(self, case, service):
        path = "/telegraph/api/notifications" if service == "telegraph" else "/hive/api/organizations/search?query=mesh"
        status, _ = self.http.request("GET", path, headers={"x-principal-iss": ISSUER,
                "x-principal-sub": "00000000-0000-0000-0000-000000000099"}, service=service, case=case)
        require(status == 401)

    def provider_fixture(self):
        if self.provider:
            return self.provider
        if self.provider_attempted:
            raise Blocked("provider creation ACK uncertain; no retry/adoption")
        self.auth()
        if not callable(getattr(self.http.driver.runtime.target(), "e2e_forward", None)):
            raise Blocked("parent e2e_forward capability unavailable: iam")
        for identity in IDENTITIES:
            try:
                self.http.pki(identity)
            except (OSError, ValueError) as error:
                raise Blocked("fixed TLS identity unavailable: " + identity) from error
        values = {**self.values, "nonce": self.inputs["run_id"], "token": "simulated-provider-" + self.inputs["run_id"]}
        self.receipt.append("provider-intent", {k: v for k, v in values.items() if k != "token"})
        self.provider_attempted = True
        ack = self.db.execute("provider-insert", values)
        if (not isinstance(ack, dict) or type(ack.get("id")) is not int or ack["id"] < 1
                or not isinstance(ack.get("sha"), str) or not re.fullmatch(r"[a-f0-9]{64}", ack["sha"])):
            raise Blocked("provider insert ACK unavailable; no retry or adoption")
        self.provider = {**values, "id": ack["id"], "sha": ack["sha"]}
        self.receipt.append("provider-created", {k: v for k, v in self.provider.items() if k != "token"})
        self.capture()
        return self.provider

    def s2s(self, case, operation, mode, identity="envoy-mesh"):
        self.auth()
        fixture = self.provider_fixture()
        principal = {"x-principal-iss": self.claims["iss"], "x-principal-sub": self.claims["sub"]}
        gate = {"x-iam-internal-token": self.credentials["internal_token"]}
        headers = {}
        if mode in {"bearer", "all", "no-principal", "app"}:
            headers.update(self.auth())
        if mode in {"spoof", "all", "app", "missing", "wrong", "authorized"}:
            headers.update(principal)
        if mode in {"internal", "all", "no-principal", "app", "authorized"}:
            headers.update(gate)
        if mode == "wrong":
            headers["x-iam-internal-token"] = "wrong-internal-token"
        expected = 200 if mode == "authorized" else 403 if mode in {"missing", "wrong"} else 401
        if operation == "after-revoke":
            operation, expected = "token", 404
        method = "DELETE" if operation == "revoke" else "POST"
        status, raw = self.http.request(method, f"/iam/internal/github/{operation}", headers=headers,
                                        service="iam", identity=identity, case=case)
        require(status == expected)
        body = response_json((status, raw))[1] if expected in {200, 403, 404} else {}
        if expected != 200:
            require(b'"access_token"' not in raw)
        elif operation == "token":
            require(body.get("access_token") == fixture["token"])
        if operation == "revoke" and expected == 200:
            require(self.db.execute("provider-read", fixture) == {"count": 0, "matches": 0})
            self.receipt.append("provider-revoked", {"id": fixture["id"]})
            self.capture()
        elif expected != 404:
            require(self.db.execute("provider-read", fixture) == {"count": 1, "matches": 1})

    def organization(self, case):
        self.auth()
        self.receipt.append("organization-intent", {"run": self.inputs["run_id"], "user": self.values.get("user")})
        status, body = self.request(case, "POST", "/hive/api/organizations",
                {"name": "Local full " + self.inputs["run_id"], "slug": "lf-" + self.inputs["run_id"]}, self.auth())
        require(status in {200, 201})
        self.receipt.append("organization-created", {"id": canonical_uuid(body["id"]), "user": self.values["user"],
                                                     "slug": "lf-" + self.inputs["run_id"]})

    def membership(self, case):
        organization = self.receipt.last("organization-created")
        if not organization:
            raise Blocked("owned Hive organization unavailable")
        deadline = time.monotonic() + 45
        while True:
            status, body = self.request(case, "GET", "/hive/api/organizations/" + organization["id"] +
                                        "/members/" + self.values["user"], headers=self.auth())
            if status == 200:
                require(body["issuer"] == self.claims["iss"] and body["user_id"] == self.values["user"])
                return
            require(status == 403)
            if time.monotonic() >= deadline:
                raise Blocked("Hive sentinel owner projection unavailable within bound")
            time.sleep(1)

    def run(self):
        r = self.results
        for name, operation in (("signup", self.signup), ("completion", self.complete), ("login", self.login), ("jwks", self.jwks)):
            r.case(name, operation)
        for mode in ("no-jwt", "sub", "bearer-spoof", "spoof-no-jwt"):
            r.case("me-" + mode, lambda c, mode=mode: self.me(c, mode))
        r.case("refresh-persisted", self.rotate)
        r.case("notifications-sub", self.notifications)
        for service in ("telegraph", "hive"):
            r.case(service + "-bypass", lambda c, service=service: self.bypass(c, service))
        for operation in ("token", "revoke"):
            r.case(operation + "-gateway-no-principal", lambda c, op=operation: self.s2s(c, op, "no-principal"))
            for mode in ("bearer", "spoof", "internal", "all"):
                r.case(f"{operation}-mesh-{mode}", lambda c, op=operation, mode=mode: self.s2s(c, op, mode, "mesh-client"))
            for identity in ("iam-service", "hive-service", "telegraph-service", "manifesto-service"):
                r.case(f"{operation}-{identity}", lambda c, op=operation, identity=identity: self.s2s(c, op, "app", identity))
            for mode in ("missing", "wrong"):
                r.case(f"{operation}-gate-{mode}", lambda c, op=operation, mode=mode: self.s2s(c, op, mode))
        r.case("s2s-token", lambda c: self.s2s(c, "token", "authorized"))
        r.case("s2s-revoke", lambda c: self.s2s(c, "revoke", "authorized"))
        r.case("s2s-after-revoke", lambda c: self.s2s(c, "after-revoke", "authorized"))
        r.case("organization", self.organization)
        r.case("membership-issuer", self.membership)


def cleanup(mesh):
    """Run independent exact inverses even when another inverse is blocked."""
    receipt, services, db = mesh.receipt, mesh.http, mesh.db
    failures = []
    account = receipt.last("account-created")
    before = len(receipt.events)
    terminal = next((i for i, e in enumerate(receipt.events)
                     if e["kind"] == "cleanup" and e["data"].get("complete") is True), None)
    if terminal is not None:
        if any(e["kind"] in {"execution-started", "signup-intent", "account-created", "provider-intent",
                             "provider-created", "organization-intent", "organization-created"}
               for e in receipt.events[terminal + 1:]):
            return {"status": "BLOCKED", "residuals": ["creation-after-terminal-cleanup"], "removed": 0}
        return {"status": "PASS", "residuals": [], "removed": 0}
    if account or receipt.last("signup-intent"):
        try:
            db.qualify_notifications_cleanup(account or mesh.values)
        except Exception:
            receipt.append("cleanup", {"complete": False, "residuals": ["telegraph-notifications-unqualified"]})
            return {"status": "BLOCKED", "residuals": ["telegraph-notifications-unqualified"], "removed": 0}

    def attempt(name, operation):
        try:
            operation()
        except Exception:
            failures.append(name)  # never interpolate database/HTTP exception contents

    def organization():
        ack = receipt.last("organization-created")
        if receipt.last("organization-intent") and not ack:
            raise Blocked("organization ACK uncertain; no adoption by slug")
        if not ack or receipt.last("organization-deleted"):
            return
        if not account or ack["user"] != account["user"]:
            raise Blocked("organization creation provenance differs")
        if not mesh.token:
            status, body = response_json(services.request("POST", "/iam/api/auth/login",
                {"email": account["email"], "password": mesh.credentials["password"]}))
            require(status == 200 and body["user"]["id"] == account["user"])
            mesh.token = body["access_token"]
            receipt.append("refresh-hashes", [*(receipt.last("refresh-hashes") or []),
                                              hashlib.sha256(body["refresh_token"].encode()).hexdigest()])
            mesh.capture()
        status, current = response_json(services.request("GET", "/hive/api/organizations/" + ack["id"], headers=mesh.auth()))
        require(status == 200 and current["id"] == ack["id"] and current["slug"] == ack["slug"])
        status, _ = services.request("DELETE", "/hive/api/organizations/" + ack["id"], headers=mesh.auth())
        require(status in {200, 204})
        receipt.append("organization-deleted", {"id": ack["id"]})

    def provider():
        ack = receipt.last("provider-created")
        if receipt.last("provider-intent") and not ack:
            raise Blocked("provider insert ACK uncertain; no adoption")
        if not ack or receipt.last("provider-cleaned"):
            return
        require(account is not None and ack["user"] == account["user"] and ack["nonce"] == mesh.inputs["run_id"])
        fixture = {**ack, "token": "simulated-provider-" + mesh.inputs["run_id"]}
        state = db.execute("provider-state", fixture)
        removed = False
        if state == {"exists": True, "matches": True}:
            require(db.execute("provider-delete", fixture) == {"id": ack["id"], "sha": ack["sha"]})
            removed = True
        else:
            require(state == {"exists": False, "matches": False})
        receipt.append("provider-cleaned", {"id": ack["id"], "removed": removed})
        # This is the ACKed inverse's exact, expected snapshot transition.
        expected = receipt.last("account-snapshot")
        if expected is not None:
            expected = {**expected, "provider_tokens": []}
            require(db.execute("snapshot", account) == expected)
            receipt.append("account-snapshot", expected)

    def mail():
        ack = receipt.last("mail-message")
        if not ack and account:
            raise Blocked("run email message ACK unavailable; residue not adopted")
        if not ack or receipt.last("mail-deleted"):
            return
        require(account is not None and ack["email"] == account["email"])
        path = "/api/v1/messages/" + quote(ack["id"], safe="")
        status, message = response_json(services.request("GET", path, service="mailhog"))
        require(status == 200 and message["ID"] == ack["id"])
        require([p.get("Mailbox", "") + "@" + p.get("Domain", "") for p in message["To"]] == [ack["email"]])
        status, _ = services.request("DELETE", path, service="mailhog")
        require(status == 200)
        receipt.append("mail-deleted", {"id": ack["id"]})

    def account_inverse():
        if receipt.last("signup-intent") and not account:
            raise Blocked("signup ACK uncertain; no adoption by email")
        if not account or receipt.last("account-deleted"):
            return
        # Do not remove the identity needed to finish an outstanding API inverse.
        if failures:
            raise Blocked("dependent inverse incomplete")
        db.cleanup_account(account, receipt.last("account-snapshot"))
        receipt.append("account-deleted", {"user": account["user"]})

    attempt("organization", organization)
    attempt("provider", provider)
    attempt("mail", mail)
    attempt("account", account_inverse)
    receipt.append("cleanup", {"complete": not failures, "residuals": failures})
    # Count ACKed inverses in this invocation, not resources inferred by names.
    removed = sum(e["kind"] in {"organization-deleted", "provider-cleaned", "mail-deleted", "account-deleted"}
        and e["data"].get("removed", True)
        for e in receipt.events[before:])
    return {"status": "BLOCKED" if failures else "PASS", "residuals": failures, "removed": removed}


def binding(driver, args, data, inputs, services):
    source = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in
              (Path(__file__), Path(__file__).with_name("local-full-acceptance.py"),
               driver.ROOT / "ops/scripts/prepare-local-full-certs.py",
               driver.ROOT / "ops/scripts/mesh-authn-kind-e2e-cases.sh")}
    # Public leaf fingerprints bind the read-only host PKI without ever reading
    # a CA key or hashing/copying private key bytes. load_cert_chain checks pairing.
    certificates = {}
    root = services.pki_root()
    for name in ("ca", *IDENTITIES):
        path = root / "mesh" / (name + ".crt")
        certificates[name] = hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None
    return {"version": 1, "run_id": inputs["run_id"], "suite": inputs["suite"],
            "source": source, "git": driver.git_revision(), "context": driver.CONTEXT,
            "namespace_uids": {n: data.get("namespaceUids", {}).get(n) for n in driver.NAMESPACES.values()},
            "credential_sha256": hashlib.sha256(inputs["credentials_path"].read_bytes()).hexdigest(),
            "public_certificates_sha256": certificates,
            "input_sha256": hashlib.sha256(Path(args.inputs).read_bytes()).hexdigest()}


def run_action(driver, args, data):
    inputs = read_inputs(args.inputs, driver)
    if args.action == "e2e" and args.suite != inputs["suite"]:
        raise Blocked("input suite differs from selected suite")
    for namespace in driver.NAMESPACES.values():
        driver.namespace_owned(data, namespace)
    services = Services(driver, args, data, inputs)
    database = Database(driver, data)
    bound = binding(driver, args, data, inputs, services)
    credentials = driver.strict_json(inputs["credentials_path"])
    if not isinstance(credentials, dict) or set(credentials) != {"password", "internal_token"} or any(
            not isinstance(v, str) or not 8 <= len(v) <= 256 or any(ord(c) < 32 for c in v) for v in credentials.values()):
        raise Blocked("closed private credential schema required")
    if args.action == "prepare-probes":
        # Qualification only. Business rows are created by e2e and ACKed immediately.
        values = {"run": inputs["run_id"]}
        prerequisites = []
        registry = None
        if inputs["suite"] == "mesh-authn":
            try:
                registry = services.iam_providers()
                if "github" not in registry["providers"]:
                    prerequisites.append("effective IAM connector github absent; mesh39 expectations unchanged")
            except Blocked as error:
                prerequisites.append(str(error))
        try:
            database.qualify_notifications_cleanup(values)
        except Blocked as error:
            prerequisites.append(str(error))
        if not prerequisites:
            database.qualify_account_cleanup(values)
        receipt = Receipt(inputs["receipt_path"], driver, bound, create=True)
        if registry is not None:
            receipt.append("iam-provider-registry", registry)
        capabilities = {"e2e_forward": callable(getattr(driver.runtime.target(), "e2e_forward", None))}
        missing_identities = []
        for identity in IDENTITIES:
            try:
                services.pki(identity)
            except Exception:
                missing_identities.append(identity)
        receipt.append("prepared", {"capabilities": capabilities, "missing_identities": missing_identities,
                                    "blocked_prerequisites": prerequisites, "created_resources": []})
        args.preparation_report = {"prepared": capabilities["e2e_forward"] and not missing_identities and not prerequisites and inputs["suite"] == "mesh-authn", "capabilities": capabilities,
                                   "missing_identities": missing_identities,
                                   "blocked_prerequisites": prerequisites, "effective_provider_registry": registry,
                                   "unwired_obligations": [name for name, _ in APPARATUS_CASES] if inputs["suite"] == "apparatus" else [],
                                   "created_resources": [], "qualification": "preconditions inspected; no fixture created; IAM inverse SQL prepared only when prerequisite list is empty"}
        args.executed = int(args.preparation_report["prepared"])
        return args.preparation_report
    receipt = Receipt(inputs["receipt_path"], driver, bound)
    if not receipt.last("prepared"):
        raise Blocked("preparation ACK required")
    if args.action == "cleanup-probes":
        mesh = Mesh(services, database, receipt, credentials, inputs, Results(inputs["suite"]))
        if receipt.last("account-created"):
            mesh.values = receipt.last("account-created")
        outcome = cleanup(mesh)
        args.cleanup_report = outcome
        args.executed = 1
        if outcome["status"] != "PASS":
            raise Blocked("strict cleanup incomplete: " + ",".join(outcome["residuals"]))
        return {"cleanup": outcome, "created_resources": []}
    results = Results(inputs["suite"])
    assert_run_open(receipt)
    if receipt.last("execution-started"):
        raise Blocked("E2E already started; cleanup required, no replay/adoption")
    receipt.append("execution-started", {"suite": inputs["suite"]})
    mesh = Mesh(services, database, receipt, credentials, inputs, results)
    if inputs["suite"] == "mesh-authn":
        try:
            mesh.run()
        finally:
            cleanup_outcome = cleanup(mesh)
    else:
        # Individual source gaps are explicit. The source M6 injects these ports;
        # none can be replaced with a trusted JSON boolean or a 403-only assertion.
        reasons = {"h1-signed-envelope": "adapter gap: fresh signing/admission Job ACK and verifiable envelope not wired",
                   "h2-persisted-scheduled": "adapter gap: digest collision preflight and admission/scheduling ACK not wired",
                   "h3-no-consent-zero-hop": "source gap: CountingLocator is test-only; no deployed zero-hop observer interface",
                   "h3-invoke-hostname": "adapter gap: owned enrolled plugin and in-place session client not wired",
                   "h4-live-canary": "adapter gap: current owned canary and positive HTTP control not wired",
                   "h4-plugin-connect-timeout": "adapter gap: owned endpoint UID and plugin GNU wget observation not wired"}
        for name, reason in reasons.items():
            def blocked(case, reason=reason):
                raise Blocked(reason)
            results.case(name, blocked)
        cleanup_outcome = cleanup(mesh)
    report = results.report()
    report["provider_fixture"] = "SIMULATED_PROVIDER"
    if inputs["suite"] == "apparatus":
        report["implementation"] = "INCOMPLETE: inventory only; not a deployed M6 proof"
    report["cleanup"] = cleanup_outcome
    args.executed = report["executed"]
    args.e2e_report = report
    receipt.append("suite-results", report)
    return report
