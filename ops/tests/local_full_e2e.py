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
import math
import os
from pathlib import Path
import re
import ssl
import stat
import sys
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
RETAIN_MODE = "RETAIN_LOCAL_V1"
RESIDUAL_CLASSES = ["accounts", "sessions", "provider-fixture", "organizations", "mail",
                    "notifications", "deliveries", "outbox", "fga"]
UNCERTAINTIES = ("signup-ack", "provider-ack", "organization-ack", "case-journal", "receipt", "technical-cleanup")
_IAM_PROVIDER_IDENTITIES = ("namespace_uid", "deployment_uid", "replica_set_uid", "pod_uid")
RETENTION_PREPARED = {"mode": RETAIN_MODE, "prepared": True,
    "inverse_qualification": "UNQUALIFIED_NOT_REQUIRED_BY_CONSENT",
    "remaining_prerequisites": [], "created_resources": [],
    "exceptions": ["telegraph-final-inverse", "account-final-inverse"]}

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
IAM_RETAIN_PROVIDER_SCRIPT = ("set -eu\nexec 2>/dev/null\n"
    'test "$RUST_ENVIRONMENT" = development\n'
    "test ! -e /app/config/local.toml\n"
    'before=$(sha256sum /app/config/development.toml)\n'
    "projection=$(awk '" + IAM_PROVIDER_AWK + "' /app/config/development.toml)\n"
    'after=$(sha256sum /app/config/development.toml)\n'
    'test "$before" = "$after"\n'
    'printf "%s\\n" "$projection"\n')


class Blocked(ValueError):
    """A named, nonsecret prerequisite is unavailable; no fabricated evidence."""


def _standard_iam_snapshot(pod):
    """Recognize the existing local OpenBao profile, not arbitrary emptyDir."""
    path = Path(__file__).resolve().parents[1] / "deploy/local_full_openbao.py"
    spec = importlib.util.spec_from_file_location("local_full_e2e_openbao_profile", path)
    profile = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(profile)
    actual = pod["spec"]
    volumes = {v["name"]: v for v in actual["volumes"]}
    if len(volumes) != len(actual["volumes"]):
        raise Blocked("ambiguous IAM snapshot volumes")
    snapshots = [c for c in actual.get("initContainers", []) if c.get("name") == "openbao-snapshot"]
    if len(snapshots) != 1:
        raise Blocked("standard IAM snapshot init required")
    init = snapshots[0]
    templates = [m for m in init.get("volumeMounts", []) if m.get("mountPath") == "/template"]
    if len(templates) != 1:
        raise Blocked("standard IAM snapshot template required")
    source = volumes[templates[0]["name"]]
    if set(source) not in ({"name", "secret"}, {"name", "configMap"}):
        raise Blocked("standard IAM snapshot source reference required")
    scripts = volumes["openbao-scripts"]["configMap"]["name"]
    if not re.fullmatch(r"local-full-bootstrap-scripts-[A-Za-z0-9-]+", scripts):
        raise Blocked("standard IAM snapshot scripts required")
    expected = {"containers": [{"name": "iam", "volumeMounts": []}], "volumes": [source]}
    profile.snapshot(expected, "iam-hmac", "iam-openbao", scripts, config_volume=source["name"])
    for key in ("serviceAccountName", "automountServiceAccountToken"):
        if actual.get(key) != expected[key]:
            raise Blocked("standard IAM snapshot identity required")
    for volume in expected["volumes"][1:]:
        observed = volumes.get(volume["name"], {})
        if "configMap" in observed:
            reference = observed["configMap"]
            if reference.get("defaultMode", 0o644) != 0o644 or reference.get("optional", False) is not False:
                raise Blocked("standard IAM snapshot ConfigMap defaults required")
            observed = {**observed, "configMap": {k: v for k, v in reference.items() if k not in {"defaultMode", "optional"}}}
        if observed != volume:
            raise Blocked("standard IAM snapshot volume required")
    fixed = expected["initContainers"][0]
    # Kubernetes omits the false default when serializing writable mounts.
    observed_mounts = [{**mount, "readOnly": mount.get("readOnly", False)} for mount in init.get("volumeMounts", [])]
    if (observed_mounts != fixed["volumeMounts"]
            or any(init.get(key) != fixed[key] for key in ("image", "command", "securityContext"))
            or any(init.get(key) for key in ("args", "env", "envFrom"))):
        raise Blocked("standard IAM snapshot command/mounts required")
    states = [s for s in pod.get("status", {}).get("initContainerStatuses", []) if s.get("name") == init["name"]]
    if len(states) != 1 or states[0].get("state", {}).get("terminated", {}).get("exitCode") != 0:
        raise Blocked("successful IAM snapshot init required")
    writers = [(kind, c["name"], m.get("mountPath")) for kind, containers in
               (("init", actual.get("initContainers", [])), ("app", actual["containers"])) for c in containers
               for m in c.get("volumeMounts", []) if m.get("name") == "private-config" and m.get("readOnly") is not True]
    if writers != [("init", "openbao-snapshot", "/runtime")]:
        raise Blocked("exclusive IAM snapshot writer required")


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
        self.receipt = None

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
        if self.receipt is not None and name != "jwks":
            checked = case.get("checked_assertions", [])
            if case["status"] == "FAIL" and not case.get("failed_assertions"):
                case.update(status="BLOCKED", completed=False, evidence="NOT RUN",
                            reason="business assertion not completed; details redacted")
            if (case["status"] in {"PASS", "FAIL"} and case.get("attempted") is True
                    and case.get("observed") is True and checked and type(case.get("http_status")) is int
                    and _required_assertions(name).intersection(checked)):
                event = {key: case[key] for key in ("id", "status", "attempted", "observed", "completed", "evidence",
                                                   "http_status", "checked_assertions", "failed_assertions")}
                try:
                    self.receipt.append("case-observed", event)
                except Exception:
                    capability = self.receipt.retention
                    capability.journal_failed = capability.poisoned = True
            for resource, ack in (("signup", "account-created"), ("provider", "provider-created"),
                                  ("organization", "organization-created")):
                if self.receipt.last(resource + "-intent") and not self.receipt.last(ack):
                    self.receipt.retention.poisoned = True
            if set(case.get("failed_assertions", [])) & set(_CAPTURE_ASSERTIONS):
                self.receipt.retention.poisoned = True
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
    if not isinstance(value, (str, Path)):
        raise Blocked("canonical absolute private path required")
    path = Path(value)
    if not path.is_absolute() or path.is_symlink() or path.resolve() != path:
        raise Blocked("canonical absolute private path required")
    for component in (path, *path.parents):
        if os.path.lexists(component):
            info = component.lstat()
            if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
                raise Blocked("linked/reparse private path forbidden")
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
    if any(e["kind"] == "retained" or e["kind"] == "cleanup" and e["data"].get("complete") is True for e in receipt.events):
        raise Blocked("run terminal after cleanup; no E2E replay or new mutation permitted")


def read_inputs(path, driver):
    anchor = driver.runtime.target()
    path = private_path(str(path), anchor)
    value = _strict_private_json(path)
    required = {"version", "run_id", "suite", "provider_mode", "credentials_path", "pki_directory", "receipt_path"}
    if not isinstance(value, dict) or type(value.get("version")) is not int or value["version"] not in {1, 2}:
        raise Blocked("closed E2E input v1/v2 schema required")
    if set(value) != (required if value["version"] == 1 else required | {"cleanup_policy"}):
        raise Blocked("closed E2E input v1/v2 schema required")
    if value["version"] == 2:
        policy = value["cleanup_policy"]
        if (not isinstance(policy, dict) or set(policy) != {"mode", "consent_id"}
                or policy["mode"] != RETAIN_MODE or value["suite"] != "mesh-authn"):
            raise Blocked("closed mesh-authn retention policy required")
        canonical_uuid(policy["consent_id"])
    canonical_uuid(value["run_id"])
    if type(value["suite"]) is not str or value["suite"] not in {"mesh-authn", "apparatus"} or value["provider_mode"] != "SIMULATED_PROVIDER":
        raise Blocked("declared suite and SIMULATED_PROVIDER required")
    value["credentials_path"] = private_path(value["credentials_path"], anchor)
    value["pki_directory"] = private_path(value["pki_directory"], anchor, directory=True)
    value["receipt_path"] = private_path(value["receipt_path"], anchor, directory=True)
    if value["version"] == 2:
        for location in (path, value["credentials_path"], value["pki_directory"], value["receipt_path"]):
            if any((parent / ".git").exists() for parent in (location, *location.parents)):
                raise Blocked("retention private paths must be outside Git working trees")
        parent = value["receipt_path"].parent
        if not parent.is_dir():
            raise Blocked("existing private receipt parent required")
        # privacy(file) checks its parent ACL too. Otherwise reuse the PKI
        # owner's read-only directory guard; never change an existing DACL.
        if parent not in {path.parent, value["credentials_path"].parent}:
            try:
                certificate_guard(driver).private_directory(parent, create=False)
            except Exception as error:
                raise Blocked("owner-only receipt parent required") from error
    return value


def _canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True, allow_nan=False).encode("utf-8")


def _strict_private_json(path):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError()
            result[key] = value
        return result

    def number(raw):
        value = float(raw)
        if not math.isfinite(value):
            raise ValueError()
        return value

    try:
        with path.open("rb") as stream:
            raw = stream.read(LIMIT + 1)
        if len(raw) > LIMIT:
            raise ValueError()
        return json.loads(raw.decode("utf-8-sig"), object_pairs_hook=pairs, parse_float=number,
                          parse_constant=lambda _: (_ for _ in ()).throw(ValueError()))
    except (OSError, ValueError, UnicodeError, RecursionError) as error:
        raise Blocked("bounded strict private JSON required; contents redacted") from error


def _retaining(inputs):
    return inputs.get("version") == 2 and inputs.get("cleanup_policy", {}).get("mode") == RETAIN_MODE


def _authority_path(driver):
    target = driver.runtime.target()
    config = getattr(target, "config", None)
    if config is None:
        return None  # Legacy protocol doubles have no target; real anchors always do.
    config = private_path(config, target)
    return config.parent / "e2e-retention-v1"


def _slot_claimed(driver):
    target = driver.runtime.target()
    config = getattr(target, "config", None)
    if config is None:
        return False
    config = private_path(config, target, directory=True)
    authority = private_path(config.parent / "e2e-retention-v1", target, directory=True)
    if any(os.path.lexists(authority / name) for name in
           ("claim.json", "consumed.json", "terminal.json", "operation.lock")):
        return True  # Even an incomplete record/absent config is a refusal, not absence.
    _authority_path(driver)  # Prove the original private anchor before declaring absence.
    return False


def _write_private_json(path, value, driver, *, _on_create=None):
    """Exclusive, private BEFORE bytes, durable BEFORE read-back/advancement."""
    raw = _canonical(value)
    if len(raw) > LIMIT:
        raise Blocked("private journal record exceeds bound")
    try:
        private_path(path.parent, driver.runtime.target(), directory=True)
        with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "wb") as stream:
            if _on_create is not None:
                _on_create()
            driver.anchor_module.secure(path)
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        private_path(path, driver.runtime.target())
        if path.read_bytes() != raw or _strict_private_json(path) != value:
            raise Blocked("private journal read-back differs")
    except (OSError, ValueError) as error:
        raise Blocked("exclusive private journal unavailable; no retry/adoption") from error


def _retained_schema(value):
    expected = {"status", "complete", "mode", "business_cleanup", "technical_cleanup", "effects_possible",
                "inventory_complete", "residual_classes", "uncertainties"}
    if (not isinstance(value, dict) or set(value) != expected or value["status"] != "RETAINED"
            or value["complete"] is not False or value["mode"] != RETAIN_MODE
            or value["business_cleanup"] != "NOT_ATTEMPTED" or type(value["technical_cleanup"]) is not str
            or value["technical_cleanup"] not in {"PASS", "BLOCKED"}
            or type(value["effects_possible"]) is not bool or value["inventory_complete"] is not False
            or value["residual_classes"] != RESIDUAL_CLASSES or not isinstance(value["uncertainties"], list)
            or any(type(v) is not str or v not in UNCERTAINTIES for v in value["uncertainties"])
            or len(set(value["uncertainties"])) != len(value["uncertainties"])):
        raise Blocked("closed retained record required")


def _iam_provider_evidence(registry):
    """Nonsecret projection only: the private configuration hash stays in memory.

    This record is an observation, never a source-qualification capability. The
    native Services qualifier must establish provenance before returning it.
    """
    providers = registry.get("providers")
    if (not isinstance(providers, list) or any(type(v) is not str or v not in {"github", "gitlab"} for v in providers)
            or len(providers) != len(set(providers))):
        raise Blocked("closed nonsecret IAM provider evidence required")
    result = {"providers": list(providers)}
    for name in _IAM_PROVIDER_IDENTITIES:
        if name in registry:
            value = registry[name]
            if not isinstance(value, str) or not re.fullmatch(r"[A-Za-z0-9-]{1,128}", value):
                raise Blocked("bounded nonsecret IAM identity required")
            result[name] = value
    return result


class _Retention:
    """Private invocation capability, never reconstructed from serialized records.

    The driver's lease mutex is outermost. This second, fixed-location mutex and
    exclusive claim/consumed survive receipt aliases, lost ACKs and process death.
    """
    def __init__(self, driver, inputs, bound, rebind):
        self.driver, self.inputs, self.rebind = driver, inputs, rebind
        self.path = _authority_path(driver)
        if self.path is None or not self.path.is_dir():
            raise Blocked("original private retention authority required")
        private_path(self.path, driver.runtime.target(), directory=True)
        self.bound = json.loads(json.dumps(bound))
        self.bound.pop("consent_sha256", None)
        self.binding_sha = hashlib.sha256(_canonical(self.bound)).hexdigest()
        self.active = self.locked = self.poisoned = False
        self.consumption_owned = False
        self.receipt = None
        self.journal_failed = False
        self.lock_identity = None
        self.lock_bytes = _canonical({"version": 1, "owner": uuid.uuid4().hex})
        self._consent()

    def __reduce__(self):
        raise TypeError("retention invocation capabilities cannot be serialized")

    def _consent(self):
        for entry in self.path.iterdir():
            if entry.name not in {"consent.json", "claim.json", "consumed.json", "terminal.json", "operation.lock"}:
                raise Blocked("unknown retention authority entry; no adoption")
            private_path(entry, self.driver.runtime.target(), directory=True)
            if not entry.is_file():
                raise Blocked("regular retention authority records required")
        file = private_path(self.path / "consent.json", self.driver.runtime.target())
        info = file.stat()
        reference = (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns)
        if hasattr(self, "consent_reference") and self.consent_reference != reference:
            raise Blocked("retention consent replaced or modified")
        value = _strict_private_json(file)
        expected = {"version": 1, "consent_id": self.inputs["cleanup_policy"]["consent_id"],
            "decision_ref": "20261103-user-retain-local-one-run", "run_id": self.inputs["run_id"],
            "mode": RETAIN_MODE, "suite": "mesh-authn", "binding_sha256": self.binding_sha,
            "max_executions": 1, "final_business_cleanup": "FORBIDDEN", "required_business_revoke": "PRESERVED",
            "sessions": "MAY_REMAIN", "disposition_owner": "mission-lead"}
        if (value != expected or type(value.get("version")) is not int
                or type(value.get("max_executions")) is not int):
            raise Blocked("exact bound one-run consent required")
        digest = hashlib.sha256(file.read_bytes()).hexdigest()
        if hasattr(self, "consent_sha") and digest != self.consent_sha:
            raise Blocked("retention consent changed")
        if self.bound.get("version") != 2 or self.bound.get("cleanup_policy") != self.inputs["cleanup_policy"]:
            raise Blocked("retention execution binding required")
        self.consent_sha = digest
        self.consent_reference = reference
        self.claim = {"version": 1, "consent_id": expected["consent_id"], "consent_sha256": digest,
                      "run_id": expected["run_id"], "binding_sha256": self.binding_sha,
                      "receipt_path": str(self.inputs["receipt_path"])}
        self.binding = {**self.bound, "consent_sha256": digest}

    def _read(self, name):
        path = private_path(self.path / name, self.driver.runtime.target())
        return _strict_private_json(path)

    def _same_claim(self, name):
        value = self._read(name)
        if value != self.claim or type(value.get("version")) is not int:
            raise Blocked("retention authority/receipt contradiction")

    def _lock_check(self):
        file = self.path / "operation.lock"
        private_path(file, self.driver.runtime.target(), directory=True)
        info = file.stat()
        if (not self.locked or (info.st_dev, info.st_ino) != self.lock_identity or file.read_bytes() != self.lock_bytes):
            raise Blocked("retention operation lock ownership lost")

    @contextmanager
    def operation(self):
        file = self.path / "operation.lock"
        try:
            # Inherits the verified private authority's ACL; deliberately writable
            # so only this invocation can release its marker without unsealing it.
            with os.fdopen(os.open(file, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "wb") as stream:
                info = os.fstat(stream.fileno())
                self.lock_identity = (info.st_dev, info.st_ino)
                marker_private = getattr(self.driver.anchor_module, "_admin_marker_private", self.driver.anchor_module.secure)
                marker_private(file)
                stream.write(self.lock_bytes)
                stream.flush()
                os.fsync(stream.fileno())
            self.locked = True
        except OSError as error:
            raise Blocked("retention operation lock exists/unavailable; parent diagnosis required") from error
        try:
            self._lock_check()
            self._consent()
            yield self
        finally:
            self.active = False
            try:
                self._lock_check()
                file.unlink()  # Never remove a foreign/replaced/stale marker.
            finally:
                self.locked = False

    def current(self):
        self._lock_check()
        if _authority_path(self.driver) != self.path:
            raise Blocked("original retention authority changed")
        self._consent()
        current = dict(self.rebind())
        current.pop("consent_sha256", None)
        if current != self.bound:
            raise Blocked("retention source/HEAD/target/epoch binding changed")

    def prepare(self):
        self.current()
        if any(os.path.lexists(self.path / n) for n in ("claim.json", "consumed.json", "terminal.json")):
            raise Blocked("retention slot already claimed; no new preparation/rearm")
        _write_private_json(self.path / "claim.json", self.claim, self.driver)
        self._same_claim("claim.json")

    def consume(self, receipt):
        self.current()
        self._same_claim("claim.json")
        assert_run_open(receipt)
        if (receipt.last("retention-prepared") != RETENTION_PREPARED or receipt.last("execution-started")
                or os.path.lexists(self.path / "terminal.json") or os.path.lexists(self.path / "consumed.json")):
            raise Blocked("retention execution unavailable; no replay or crash adoption")
        _write_private_json(self.path / "consumed.json", self.claim, self.driver,
                            _on_create=lambda: setattr(self, "consumption_owned", True))
        self._same_claim("consumed.json")
        self.receipt, self.active = receipt, True
        receipt.retention = self

    def check(self, receipt):
        try:
            self._check(receipt)
        except Exception:
            self.poisoned = True
            raise

    def _check(self, receipt):
        if not self.active or self.poisoned or receipt is not self.receipt:
            raise Blocked("live retention invocation required; serialized receipt is not a capability")
        self.current()
        self._same_claim("claim.json")
        self._same_claim("consumed.json")
        if os.path.lexists(self.path / "terminal.json"):
            raise Blocked("retention terminal forbids mutation")
        assert_run_open(receipt)
        # Detect missing/partial/replaced receipt records before further effects.
        restored = Receipt(receipt.path, self.driver, self.binding)
        if restored.events != receipt.events:
            raise Blocked("retention receipt changed after known progression")


def _live(receipt):
    capability = getattr(receipt, "retention", None)
    if not isinstance(capability, _Retention):
        raise Blocked("live retention invocation required; no receipt adoption")
    capability.check(receipt)
    return capability


class Receipt:
    """Append-only owner-only events. An intent without its ACK is never adopted."""
    def __init__(self, path, driver, binding, create=False):
        self.path, self.driver, self.binding = path, driver, binding
        self.retention = None
        self.is_retention = binding.get("version") == 2
        self.events = []
        if create:
            path.mkdir()  # no exist_ok: never adopt a previous run
            driver.anchor_module.secure(path, directory=True)
            self.append("binding", binding)
        else:
            if not path.is_dir() or path.is_symlink():
                raise Blocked("prepared private receipt required")
            private_path(path, driver.runtime.target(), directory=True)
            files = sorted(path.iterdir())
            if len(files) > 256:
                raise Blocked("receipt event count exceeds bound")
            for index, file in enumerate(files):
                private_path(str(file), driver.runtime.target())
                event = _strict_private_json(file)
                if (file.name != f"{index:06d}.json" or not isinstance(event, dict)
                        or set(event) != {"kind", "data", "previous"} or event["previous"] != self.digest()):
                    raise Blocked("receipt event chain invalid")
                self._schema(event["kind"], event["data"])
                if self.is_retention and self.last("retained") and event["kind"] != "suite-results":
                    raise Blocked("receipt mutation after retention terminal")
                self.events.append(event)
            if not self.events or self.events[0] != {"kind": "binding", "data": binding, "previous": ""}:
                raise Blocked("receipt source/run/target binding changed")

    def digest(self):
        return hashlib.sha256(json.dumps(self.events[-1], sort_keys=True).encode()).hexdigest() if self.events else ""

    def append(self, kind, data):
        self._schema(kind, data)
        if len(self.events) >= 256:
            raise Blocked("receipt event count exceeds bound")
        if self.is_retention:
            if self.last("retained") and kind != "suite-results":
                raise Blocked("retention terminal forbids journal mutation")
            if kind not in {"binding", "retention-prepared", "retained", "suite-results", "iam-provider-registry"}:
                _live(self)
            if kind == "iam-provider-registry" and self.last("retention-prepared"):
                _live(self)
        if kind in {"execution-started", "signup-intent", "provider-intent", "organization-intent",
                    "account-created", "provider-created", "organization-created"}:
            assert_run_open(self)
            if not self.is_retention and _slot_claimed(self.driver):
                raise Blocked("claimed retention authority forbids strict receipt mutation")
        # Freeze the data: subsequent updates to a caller's dict must not mutate
        # the in-memory predecessor hash after its immutable file was persisted.
        event = {"kind": kind, "data": json.loads(json.dumps(data)), "previous": self.digest()}
        file = self.path / f"{len(self.events):06d}.json"
        _write_private_json(file, event, self.driver)
        self.events.append(event)

    def _schema(self, kind, data):
        if kind == "retention-prepared" and (data != RETENTION_PREPARED or data.get("prepared") is not True):
            raise Blocked("closed retention-prepared record required")
        if kind == "retained":
            _retained_schema(data)
        if kind == "case-observed":
            _case_schema(data)
        if self.is_retention:
            allowed = {"binding", "retention-prepared", "iam-provider-registry", "execution-started", "signup-intent",
                       "account-created", "account-snapshot", "mail-message", "refresh-hashes", "provider-intent",
                       "provider-created", "provider-revoked", "organization-intent", "organization-created",
                       "case-observed", "retained", "suite-results"}
            if kind not in allowed:
                raise Blocked("unknown/cleanup inverse forbidden in retention receipt")
            if kind == "iam-provider-registry" and (not isinstance(data, dict)
                    or set(data) - {"providers", *_IAM_PROVIDER_IDENTITIES}
                    or _iam_provider_evidence(data) != data):
                raise Blocked("private IAM configuration hashes forbidden in retention evidence")
            if kind == "binding" and (self.events or data != self.binding):
                raise Blocked("single exact retention binding required")
            if kind == "retention-prepared" and (self.last("retention-prepared") or self.last("execution-started")):
                raise Blocked("single retention preparation ACK required")
            if kind == "suite-results" and not self.last("retained"):
                raise Blocked("retention suite report requires its terminal")
            if kind == "execution-started" and data != {"suite": "mesh-authn"}:
                raise Blocked("closed execution-started record required")

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
        self._pki_references = {}
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
        elif _retaining(self.inputs):
            self._pki_guard.checked_path(self._pki_root)
            self._pki_guard.private_directory(self._pki_root, create=False)
        return self._pki_root

    def pki(self, identity):
        if identity not in IDENTITIES:
            raise Blocked("fixed mesh identity required")
        if identity in self.contexts and not _retaining(self.inputs):
            return self.contexts[identity]
        root = self.pki_root() / "mesh"
        paths = [self._pki_guard.checked_path(root / name) for name in
                 ("ca.crt", identity + ".crt", identity + ".key")]
        if any(not p.is_file() or p.stat().st_size > LIMIT for p in paths):
            raise Blocked("bounded existing private PKI files required")
        references = [(p.stat().st_dev, p.stat().st_ino, p.stat().st_size, p.stat().st_mtime_ns) for p in paths]
        ca, cert, key = paths
        public_ca = self.args.environment_data.get("public_ca_path")
        if not public_ca or hashlib.sha256(ca.read_bytes()).digest() != hashlib.sha256(Path(public_ca).read_bytes()).digest():
            raise Blocked("mesh CA does not match the parent environment")
        if identity in self.contexts:
            if references != self._pki_references[identity]:
                raise Blocked("private TLS source changed; no implicit recapture")
            return self.contexts[identity]
        try:
            decoded = ssl._ssl._test_decode_cert(str(cert))
            if ("DNS", identity) not in decoded.get("subjectAltName", ()):
                raise Blocked("leaf SAN differs from its fixed oracle identity")
            context = ssl.create_default_context(cafile=str(ca))
            context.load_cert_chain(str(cert), str(key))  # read-only; never copy or return PEM
        except (OSError, ValueError) as error:
            raise Blocked("fixed leaf TLS context unavailable; details redacted") from error
        self.contexts[identity] = context
        self._pki_references[identity] = references
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
            retaining = _retaining(self.inputs)
            if retaining:
                if len(volumes) != 1 or volumes[0].get("name") != "private-config":
                    raise Blocked("standard IAM snapshot output required")
                _standard_iam_snapshot(pod)
            elif len(volumes) != 1 or not (volumes[0].get("secret") or volumes[0].get("configMap")):
                raise Blocked("IAM startup config source reference unavailable")
            before = driver.obj("Pod", metadata["name"], NAMESPACE)
            if before != pod:
                raise Blocked("IAM Pod changed before provider projection")
            # Keep the same guarded CLI. Its optional raw failure capture must
            # not persist a private projection or a subprocess exception payload.
            diagnostics = None
            if retaining:
                diagnostics = sys.modules.get("private_diagnostics")
                if diagnostics is None:
                    path = Path(__file__).resolve().parents[1] / "deploy/private_diagnostics.py"
                    spec = importlib.util.spec_from_file_location("private_diagnostics", path)
                    diagnostics = importlib.util.module_from_spec(spec)
                    spec.loader.exec_module(diagnostics)
                    sys.modules[spec.name] = diagnostics
            token = diagnostics.CURRENT.set(None) if retaining else None
            try:
                raw = driver.runtime.kubectl("exec", "-n", NAMESPACE, metadata["name"], "-c", "iam", "--",
                    "sh", "-ec", IAM_RETAIN_PROVIDER_SCRIPT if retaining else IAM_PROVIDER_SCRIPT, timeout=20)
            finally:
                if retaining:
                    diagnostics.CURRENT.reset(token)
            pattern = (r"CONNECTORS ([0-9]+) ([0-9]+) ([0-9]+) ([0-9]+) ([0-9]+) ([0-9]+)" if retaining else
                r"([a-f0-9]{64})  /app/config/development\.toml\nCONNECTORS ([0-9]+) ([0-9]+) ([0-9]+) ([0-9]+) ([0-9]+) ([0-9]+)\n\1  /app/config/development\.toml")
            match = re.fullmatch(pattern, raw.strip())
            if not match:
                raise Blocked("IAM config hash/projection malformed or changed")
            tables, ids, github, gitlab, unknown, bad = map(int, match.groups() if retaining else match.groups()[1:])
            if bad or unknown or tables != ids or ids != github + gitlab or github > 1 or gitlab > 1:
                raise Blocked("IAM provider projection ambiguous or unsupported")
            if retaining and (tables, ids, github, gitlab) != (1, 1, 1, 0):
                raise Blocked("effective IAM GitHub-only profile required")
            after = driver.obj("Pod", metadata["name"], NAMESPACE)
            current_deployment = self.resource("Deployment", "iam")
            if before != after or current_deployment["metadata"]["uid"] != deployment["metadata"]["uid"]:
                raise Blocked("IAM Pod/incarnation changed during provider projection")
            return {"namespace_uid": self.data["namespaceUids"][NAMESPACE], "deployment_uid": deployment["metadata"]["uid"],
                    "replica_set_uid": rs["metadata"]["uid"], "pod_uid": metadata["uid"],
                    **({} if retaining else {"config_sha256": match[1]}),
                    "providers": [name for name, present in (("github", github), ("gitlab", gitlab)) if present]}
        except Blocked:
            raise
        except Exception:
            pass
        # Raise outside the handler so neither context nor cause retains output.
        raise Blocked("guarded effective IAM provider qualification unavailable; details redacted")

    @contextmanager
    def forward(self, service):
        if service not in INTERNAL:
            raise Blocked("fixed E2E forward target required")
        anchor = self.driver.runtime.target()
        capability = getattr(anchor, "e2e_forward", None)
        if not callable(capability):
            raise Blocked("parent e2e_forward capability unavailable: " + service)
        body_error = None
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
                try:
                    yield port
                except BaseException as error:
                    body_error = error
                    raise
        except (ValueError, RuntimeError) as error:
            # Propagating the body exception is not a failed close; a distinct
            # lifecycle exception must remain uncertain even when the body failed.
            if error is not body_error:
                self.technical_cleanup_failed = True
            raise Blocked("guarded forward identity/lifecycle unavailable: " + service) from error

    def request(self, method, path, body=None, headers=None, *, service=None, identity="mesh-client", case=None):
        if getattr(self, "retention", None) is not None:
            self.retention.check(self.retention.receipt)
            if method == "DELETE" and path != "/iam/internal/github/revoke":
                raise Blocked("final HTTP inverse forbidden by retention consent")
        elif (method != "GET" or path.startswith("/iam/api/auth/verify?")) and _slot_claimed(self.driver):
            raise Blocked("claimed retention slot requires its live invocation before HTTP mutation")
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
                    case["http_status"] = response.status
                _check(case, "response-bound", lambda: len(raw) <= LIMIT)
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
DB_SQL["fixture-compile"] = DB_SQL["provider-insert"]


class Database:
    def __init__(self, driver, data):
        self.driver, self.data = driver, data

    def execute(self, operation, values):
        try:
            capability = getattr(self, "retention", None)
            if capability is not None:
                capability.check(capability.receipt)
                if operation in {"account-delete", "provider-delete", "cleanup-compile"}:
                    raise Blocked("final database inverse forbidden by retention consent")
            elif operation in {"provider-insert", "provider-delete", "account-delete", "cleanup-compile"} and callable(
                    getattr(self.driver.runtime, "target", None)) and _slot_claimed(self.driver):
                raise Blocked("claimed retention slot requires its live invocation before database mutation")
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
        execution = ("SELECT jsonb_build_object('prepared',true);" if operation in {"cleanup-compile", "fixture-compile"} else
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
        if getattr(self, "retention_mode", False) and self.execute("fixture-compile", values) != {"prepared": True}:
            raise Blocked("fixed provider fixture SQL unavailable")

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


# Source assertion IDs, not received prose. Capture assertions are evaluated only
# after the primary HTTP oracle; setup/transport/JWKS never attest business start.
_CAPTURE_ASSERTIONS = ("account-owned", "snapshot-cardinality", "snapshot-dependents", "refresh-ownership", "provider-ownership")
CASE_ASSERTIONS = {
    "signup": ("response-json", "http-status", "user-id", "user-email", "registration-token", *_CAPTURE_ASSERTIONS),
    "completion": ("response-json", "status-user", "no-issued-tokens", *_CAPTURE_ASSERTIONS),
    "login": ("response-json", "http-status", "issued-tokens", "jwt-shape", "subject", *_CAPTURE_ASSERTIONS),
    "me-no-jwt": ("http-status",),
    "me-sub": ("response-json", "status-subject"),
    "me-bearer-spoof": ("response-json", "status-subject"),
    "me-spoof-no-jwt": ("response-json", "http-status", "no-access-token"),
    "refresh-persisted": ("response-json", "rotation", "refresh-persisted", *_CAPTURE_ASSERTIONS),
    "notifications-sub": ("response-json", "status-user"),
    "telegraph-bypass": ("http-status",), "hive-bypass": ("http-status",),
    "organization": ("response-json", "http-status", "organization-id"),
    "membership-issuer": ("response-json", "membership-principal", "projection-pending"),
}
for _case_id, _positive in mesh_cases():
    if _case_id.startswith(("token-", "revoke-", "s2s-")):
        CASE_ASSERTIONS[_case_id] = ("http-status", "response-json", "no-access-token", "fixture-token",
                                    "provider-state", *_CAPTURE_ASSERTIONS)
for _case_id in CASE_ASSERTIONS:
    CASE_ASSERTIONS[_case_id] = (*CASE_ASSERTIONS[_case_id], "response-bound")


def _required_assertions(case_id):
    if case_id == "membership-issuer":
        return {"response-json", "membership-principal"}
    if case_id == "s2s-token":
        return {"http-status", "response-json", "fixture-token", "provider-state"}
    if case_id == "s2s-revoke":
        return {"http-status", "response-json", "provider-state", *_CAPTURE_ASSERTIONS}
    if case_id == "s2s-after-revoke":
        return {"http-status", "response-json", "no-access-token"}
    if case_id.startswith(("token-", "revoke-")):
        required = {"http-status", "no-access-token", "provider-state"}
        return required | {"response-json"} if "-gate-" in case_id else required
    return set(CASE_ASSERTIONS[case_id]) - {"response-bound"}


def _check(case, assertion, operation):
    """Record an assertion actually evaluated, including malformed-body failure."""
    if case is not None:
        if "id" in case and assertion not in CASE_ASSERTIONS.get(case["id"], ()) and case["id"] != "jwks":
            raise Blocked("source assertion ID unavailable")
        checked = case.setdefault("checked_assertions", [])
        case.setdefault("failed_assertions", [])
        if assertion not in checked:
            checked.append(assertion)
    try:
        value = operation()
        require(value)
        return value
    except Exception:
        if case is not None and assertion not in case["failed_assertions"]:
            case["failed_assertions"].append(assertion)
        raise AssertionError("source oracle assertion failed; details redacted") from None


def _case_schema(data):
    fields = {"id", "status", "attempted", "observed", "completed", "evidence", "http_status",
              "checked_assertions", "failed_assertions"}
    if (not isinstance(data, dict) or set(data) != fields or type(data["id"]) is not str
            or data["id"] not in CASE_ASSERTIONS or type(data["status"]) is not str or data["status"] not in {"PASS", "FAIL"}
            or any(data[k] is not True for k in ("attempted", "observed", "completed")) or data["evidence"] != "LIVE"
            or type(data["http_status"]) is not int or not 100 <= data["http_status"] <= 599):
        raise Blocked("closed observed business case required")
    checked, failed = data["checked_assertions"], data["failed_assertions"]
    allowed = CASE_ASSERTIONS[data["id"]]
    if (not isinstance(checked, list) or not checked or not isinstance(failed, list)
            or any(type(v) is not str or v not in allowed for v in checked + failed)
            or len(set(checked)) != len(checked) or len(set(failed)) != len(failed)
            or not set(failed) <= set(checked) or bool(failed) != (data["status"] == "FAIL")):
        raise Blocked("source assertion manifest required")
    required = _required_assertions(data["id"])
    # A partial FAIL can attest business start, but a transport bound alone cannot.
    if not required.intersection(checked):
        raise Blocked("required business assertion not evaluated")
    if data["status"] == "PASS" and not required <= set(checked):
        raise Blocked("complete source assertion manifest required for PASS")


class Mesh:
    def __init__(self, services, database, receipt, credentials, inputs, results):
        self.http, self.db, self.receipt, self.credentials = services, database, receipt, credentials
        self.inputs, self.results = inputs, results
        self._retention_mode = _retaining(inputs)
        run = inputs["run_id"]
        self.values = {"run": run, "email": "local-full-" + run + "@example.com", "username": "lf" + run.replace("-", "")}
        self.token = self.refresh = self.registration = None
        self.claims = self.header = None
        self.provider = None
        self.provider_attempted = False

    def request(self, case, method, path, body=None, headers=None, **kwargs):
        response = self._http(case, method, path, body, headers, **kwargs)
        return _check(case, "response-json", lambda: response_json(response))

    def _guard(self):
        assert_run_open(self.receipt)
        if self._retention_mode:
            _live(self.receipt)
        elif _slot_claimed(self.http.driver):
            raise Blocked("claimed retention slot forbids strict business mutation")

    def _http(self, case, method, path, body=None, headers=None, **kwargs):
        self._guard()
        response = self.http.request(method, path, body, headers, case=case, **kwargs)
        case["http_status"] = response[0]
        return response

    def auth(self):
        if not self.token:
            raise Blocked("real IAM login unavailable")
        return {"authorization": "Bearer " + self.token}

    def capture(self, case=None):
        if "user" in self.values:
            _check(case, "account-owned", lambda: self.db.execute("account-owned", self.values) == {"owned": True})
            snapshot = self.db.execute("snapshot", self.values)
            _check(case, "snapshot-cardinality", lambda: len(snapshot["users"]) == len(snapshot["user_emails"]) == 1)
            _check(case, "snapshot-dependents", lambda: not snapshot["password_reset_tokens"] and not snapshot["oauth_transactions"])
            allowed = self.receipt.last("refresh-hashes") or []
            ids = self.db.execute("refresh-ids", {**self.values, "hashes": allowed})
            _check(case, "refresh-ownership", lambda: sorted(ids) == sorted(r["id"] for r in snapshot["refresh_tokens"]))
            provider_rows = snapshot["provider_tokens"]
            _check(case, "provider-ownership", lambda: provider_rows == ([] if not self.provider or self.receipt.last("provider-revoked") else
                                      [{"id": self.provider["id"], "sha": self.provider["sha"]}]))
            self.receipt.append("account-snapshot", snapshot)

    def signup(self, case):
        # This gate is BEFORE the first mutating HTTP, not a best-effort cleanup.
        self._guard()
        if self.inputs["suite"] == "mesh-authn":
            registry = self.http.iam_providers()
            self.receipt.append("iam-provider-registry", _iam_provider_evidence(registry) if self._retention_mode else registry)
            if "github" not in registry["providers"] or self._retention_mode and registry["providers"] != ["github"]:
                raise Blocked("effective IAM connector github absent; mesh39 expectations unchanged; signup forbidden")
        if not self._retention_mode:
            self.db.qualify_notifications_cleanup(self.values)
        self.db.qualify(self.values)
        if not self._retention_mode:
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
        _check(case, "http-status", lambda: 200 <= status < 300)
        user = _check(case, "user-id", lambda: canonical_uuid(body["user"]["id"]))
        _check(case, "user-email", lambda: body["user"]["email"] == self.values["email"])
        self.values["user"] = user
        self.receipt.append("account-created", dict(self.values))
        self.registration = body.get("registration_token")
        _check(case, "registration-token", lambda: isinstance(self.registration, str) and bool(self.registration))
        self.capture(case)

    def complete(self, case):
        if not self.registration:
            raise Blocked("signup registration token unavailable")
        status, body = self.request(case, "POST", "/iam/api/auth/complete-registration",
                                    {"registration_token": self.registration, "username": self.values["username"]})
        _check(case, "status-user", lambda: 200 <= status < 300 and body["user"]["id"] == self.values["user"])
        _check(case, "no-issued-tokens", lambda: not body.get("access_token") and not body.get("refresh_token"))
        self.capture(case)

    def verify_mail(self):
        self._guard()
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
        self._guard()
        self.verify_mail()
        status, body = self.request(case, "POST", "/iam/api/auth/login",
                                    {"email": self.values["email"], "password": self.credentials["password"]})
        _check(case, "http-status", lambda: 200 <= status < 300)
        _check(case, "issued-tokens", lambda: isinstance(body["access_token"], str) and bool(body["access_token"])
               and isinstance(body["refresh_token"], str) and bool(body["refresh_token"]))
        self.token, self.refresh = body["access_token"], body["refresh_token"]
        self.header, self.claims = _check(case, "jwt-shape", lambda: token_fields(self.token))
        _check(case, "subject", lambda: self.claims["sub"] == self.values["user"])
        self.receipt.append("refresh-hashes", [hashlib.sha256(self.refresh.encode()).hexdigest()])
        self.capture(case)

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
        status, raw = self._http(case, "GET", "/iam/api/me", headers=headers)
        body = _check(case, "response-json", lambda: response_json((status, raw)))[1] if mode != "no-jwt" else {}
        if mode in {"sub", "bearer-spoof"}:
            _check(case, "status-subject", lambda: status == 200 and body["id"] == self.claims["sub"])
        else:
            _check(case, "http-status", lambda: status in {401, 403})
            if mode == "spoof-no-jwt":
                _check(case, "no-access-token", lambda: "access_token" not in body)

    def rotate(self, case):
        if not self.refresh:
            raise Blocked("login-issued refresh token unavailable")
        status, body = self.request(case, "POST", "/iam/api/token/refresh", {"refresh_token": self.refresh})
        replacement = body.get("refresh_token")
        _check(case, "rotation", lambda: 200 <= status < 300 and isinstance(replacement, str) and replacement and replacement != self.refresh)
        self.refresh = replacement
        self.receipt.append("refresh-hashes", [*(self.receipt.last("refresh-hashes") or []), hashlib.sha256(replacement.encode()).hexdigest()])
        _check(case, "refresh-persisted", lambda: self.db.execute("refresh", {**self.values, "hash": hashlib.sha256(replacement.encode()).hexdigest()}) == {"count": 1})
        self.capture(case)

    def notifications(self, case):
        status, body = self.request(case, "GET", "/telegraph/api/notifications", headers=self.auth())
        _check(case, "status-user", lambda: status == 200 and body["user_id"] == self.claims["sub"])

    def bypass(self, case, service):
        path = "/telegraph/api/notifications" if service == "telegraph" else "/hive/api/organizations/search?query=mesh"
        status, _ = self._http(case, "GET", path, headers={"x-principal-iss": ISSUER,
                "x-principal-sub": "00000000-0000-0000-0000-000000000099"}, service=service)
        _check(case, "http-status", lambda: status == 401)

    def provider_fixture(self):
        self._guard()
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
        status, raw = self._http(case, method, f"/iam/internal/github/{operation}", headers=headers,
                                        service="iam", identity=identity)
        _check(case, "http-status", lambda: status == expected)
        body = _check(case, "response-json", lambda: response_json((status, raw)))[1] if expected in {200, 403, 404} else {}
        if expected != 200:
            _check(case, "no-access-token", lambda: b'"access_token"' not in raw)
        elif operation == "token":
            _check(case, "fixture-token", lambda: body.get("access_token") == fixture["token"])
        if operation == "revoke" and expected == 200:
            _check(case, "provider-state", lambda: self.db.execute("provider-read", fixture) == {"count": 0, "matches": 0})
            self.receipt.append("provider-revoked", {"id": fixture["id"]})
            self.capture(case)
        elif expected != 404:
            _check(case, "provider-state", lambda: self.db.execute("provider-read", fixture) == {"count": 1, "matches": 1})

    def organization(self, case):
        self._guard()
        self.auth()
        self.receipt.append("organization-intent", {"run": self.inputs["run_id"], "user": self.values.get("user")})
        status, body = self.request(case, "POST", "/hive/api/organizations",
                {"name": "Local full " + self.inputs["run_id"], "slug": "lf-" + self.inputs["run_id"]}, self.auth())
        _check(case, "http-status", lambda: status in {200, 201})
        organization = _check(case, "organization-id", lambda: canonical_uuid(body["id"]))
        self.receipt.append("organization-created", {"id": organization, "user": self.values["user"],
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
                _check(case, "membership-principal", lambda: body["issuer"] == self.claims["iss"] and body["user_id"] == self.values["user"])
                return
            _check(case, "projection-pending", lambda: status == 403)
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
    if _retaining(mesh.inputs) or getattr(mesh.receipt, "is_retention", False):
        capability = getattr(mesh.receipt, "retention", None)
        if not isinstance(capability, _Retention):
            return {"status": "BLOCKED", "mode": RETAIN_MODE, "complete": False, "removed": 0,
                    "technical_status": "BLOCKED", "residuals": list(RESIDUAL_CLASSES), "uncertainties": ["receipt"]}
        return _terminalize(capability, mesh.receipt, mesh.http)
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


def binding(driver, args, data, inputs, services, *, _qualify_transport=False):
    if _retaining(inputs):
        for path in (Path(args.inputs), inputs["credentials_path"]):
            private_path(path, driver.runtime.target())
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
    result = {"version": 1, "run_id": inputs["run_id"], "suite": inputs["suite"],
            "source": source, "git": driver.git_revision(), "context": driver.CONTEXT,
            "namespace_uids": {n: data.get("namespaceUids", {}).get(n) for n in driver.NAMESPACES.values()},
            "credential_sha256": hashlib.sha256(inputs["credentials_path"].read_bytes()).hexdigest(),
            "public_certificates_sha256": certificates,
            "input_sha256": hashlib.sha256(Path(args.inputs).read_bytes()).hexdigest()}
    if _retaining(inputs):
        target = driver.runtime.target()
        current = target.check()
        fields = ("task_id", "kube_system_uid", "nodeContainerIds")
        if any(current.get(k) != data.get(k) for k in fields):
            raise Blocked("retention target anchor differs from verified lease")
        if current.get("namespaceUids") != data.get("namespaceUids"):
            raise Blocked("retention namespace lease binding changed")
        for namespace in driver.NAMESPACES.values():
            driver.namespace_owned(data, namespace)
        uids = result["namespace_uids"]
        if len(uids) != 3 or any(type(v) is not str or not v for v in uids.values()):
            raise Blocked("three current namespace UIDs required")
        spec = importlib.util.spec_from_file_location("local_full_e2e_epoch", driver.ROOT / "ops/deploy/local_full_environment.py")
        environment = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(environment)
        anchor = {k: current[k] for k in fields}
        anchor["nodeContainerIds"] = sorted(anchor["nodeContainerIds"])
        anchor["openbao_epoch_sha256"] = environment.coffer_epoch(target)
        if (any(type(anchor[k]) is not str or not anchor[k] for k in ("task_id", "kube_system_uid"))
                or len(anchor["nodeContainerIds"]) != 3 or len(set(anchor["nodeContainerIds"])) != 3
                or any(not isinstance(v, str) or not re.fullmatch(r"[a-f0-9]{64}", v) for v in anchor["nodeContainerIds"])
                or not re.fullmatch(r"[a-f0-9]{64}", anchor["openbao_epoch_sha256"])):
            raise Blocked("complete target/epoch binding required")
        # Cert and transport qualification are read-only, in the same fixed
        # binding seam as PKI. Doubled bindings in unit tests are not live proof.
        if _qualify_transport:
            for identity in IDENTITIES:
                services.pki(identity)
            for service in INTERNAL:
                with services.forward(service):
                    pass
        result.update(version=2, cleanup_policy=dict(inputs["cleanup_policy"]), target_anchor=anchor)
    return result


def _preflight_retention(services, database, inputs, driver):
    registry = services.iam_providers()
    if registry["providers"] != ["github"]:
        raise Blocked("effective IAM GitHub-only profile required before retention claim/consumption")
    if not callable(getattr(driver.runtime.target(), "e2e_forward", None)):
        raise Blocked("fixed E2E transport capability required")
    for identity in IDENTITIES:
        services.pki(identity)
    values = {"run": inputs["run_id"], "email": "local-full-" + inputs["run_id"] + "@example.com",
              "username": "lf" + inputs["run_id"].replace("-", "")}
    database.retention_mode = True
    database.qualify(values)  # FK/fixture only, never either final inverse.
    if database.execute("absence", values) != {"absent": True}:
        raise Blocked("new run email/username/verification required; no adoption")
    return registry


def _terminalize(capability, receipt, services, *, crashed=False):
    """No API/SQL, recovery login or PID adoption. Repair only an identical tail."""
    capability.active = False
    uncertainties = set()
    technical = "PASS"
    try:
        previous = receipt.last("retained") if receipt is not None else None
        terminal_path = capability.path / "terminal.json"
        terminal_present = os.path.lexists(terminal_path)
        capability._lock_check()
        if _authority_path(capability.driver) != capability.path:
            raise Blocked("original retention authority unavailable")
        try:
            capability.current()
        except Exception:
            # A known invocation may close its ORIGINAL slot after source drift,
            # never recapture/adopt the new binding or perform another effect.
            if not capability.consumption_owned:
                raise
            uncertainties.add("receipt")
            technical = "BLOCKED"
        try:
            capability._same_claim("claim.json")
        except Exception:
            if not capability.consumption_owned:
                raise
            uncertainties.add("receipt")
            technical = "BLOCKED"
        consumed = os.path.lexists(capability.path / "consumed.json") or capability.consumption_owned
        if consumed:
            try:
                capability._same_claim("consumed.json")
            except Exception:
                uncertainties.add("receipt")
                technical = "BLOCKED"
        target = capability.driver.runtime.target()
        markers = [target.config.parent / ".e2e-forward", target.config.parent / ".envoy-admin"]
        if getattr(target, "path", None) is not None:
            markers.append(target.path.with_name(target.path.name + ".oauth-forward"))
        if (any(os.path.lexists(p) for p in markers)
                or crashed and consumed and previous is None and not terminal_present
                or getattr(services, "technical_cleanup_failed", False)):
            technical = "BLOCKED"
            uncertainties.add("technical-cleanup")
        if receipt is None:
            uncertainties.add("receipt")
        else:
            if consumed and not receipt.last("account-created"):
                uncertainties.add("signup-ack")
            for resource in ("provider", "organization"):
                if receipt.last(resource + "-intent") and not receipt.last(resource + "-created"):
                    uncertainties.add(resource + "-ack")
        if capability.journal_failed:
            uncertainties.add("case-journal")
        retained = {"status": "RETAINED", "complete": False, "mode": RETAIN_MODE,
            "business_cleanup": "NOT_ATTEMPTED", "technical_cleanup": technical, "effects_possible": consumed,
            "inventory_complete": False, "residual_classes": list(RESIDUAL_CLASSES),
            "uncertainties": [v for v in UNCERTAINTIES if v in uncertainties]}
        durable = capability._read("terminal.json") if terminal_present else None
        identity = {k: capability.claim[k] for k in ("version", "run_id", "consent_id", "binding_sha256", "consent_sha256")}
        if durable is not None:
            if not isinstance(durable, dict) or set(durable) != set(retained) | set(identity):
                raise Blocked("closed retention terminal required")
            retained = {k: durable[k] for k in retained}
            _retained_schema(retained)
            if durable != {**retained, **identity} or type(durable["version"]) is not int:
                raise Blocked("retention terminal binding differs")
        elif previous is not None:
            retained = previous
        if previous is not None and previous != retained:
            raise Blocked("retention terminal journals disagree")
        _retained_schema(retained)
        if receipt is not None and previous is None:
            receipt.append("retained", retained)
        if durable is None:
            _write_private_json(terminal_path, {**retained, **identity}, capability.driver)
        if capability._read("terminal.json") != {**retained, **identity}:
            raise Blocked("retention terminal read-back differs")
        if retained["effects_possible"] != consumed:
            uncertainties.add("receipt")
        observed_uncertainties = [v for v in UNCERTAINTIES if v in uncertainties or v in retained["uncertainties"]]
        if technical != "PASS" or receipt is None or retained["technical_cleanup"] != "PASS" or any(
                u in observed_uncertainties for u in ("case-journal", "receipt", "technical-cleanup")):
            status = "BLOCKED"
        else:
            status = "RETAINED"
        return {"status": status, "complete": False, "mode": RETAIN_MODE, "removed": 0,
                "technical_status": "BLOCKED" if technical != "PASS" else retained["technical_cleanup"], "residuals": list(RESIDUAL_CLASSES),
                "uncertainties": observed_uncertainties, "effects_possible": consumed or retained["effects_possible"],
                "inventory_complete": False, "executed": 0, "launch_status": "NOT_PROVEN"}
    except Exception:
        capability.poisoned = True
        return {"status": "BLOCKED", "complete": False, "mode": RETAIN_MODE, "removed": 0,
                "technical_status": "BLOCKED", "residuals": list(RESIDUAL_CLASSES),
                "uncertainties": ["receipt", "technical-cleanup"], "executed": 0, "launch_status": "NOT_PROVEN"}


def _run_retention(driver, args, data, inputs, services, database, bound, credentials):
    capability = _Retention(driver, inputs, bound, lambda: binding(driver, args, data, inputs, services))
    with capability.operation():
        if args.action == "prepare-probes":
            if any(os.path.lexists(capability.path / n) for n in ("claim.json", "consumed.json", "terminal.json")):
                raise Blocked("retention slot already claimed; no rearm")
            registry = _preflight_retention(services, database, inputs, driver)
            binding(driver, args, data, inputs, services, _qualify_transport=True)
            # Receipt absence is a read-only prerequisite; never claim over an old directory.
            if os.path.lexists(inputs["receipt_path"]):
                raise Blocked("fresh private receipt required before claim")
            capability.prepare()
            receipt = Receipt(inputs["receipt_path"], driver, capability.binding, create=True)
            receipt.append("iam-provider-registry", _iam_provider_evidence(registry))
            receipt.append("retention-prepared", RETENTION_PREPARED)
            args.executed = 0
            args.preparation_report = {"status": "READY_RETAINED", "prepared": True, "mode": RETAIN_MODE,
                "consent_id": inputs["cleanup_policy"]["consent_id"], "executed": 0, "evidence": "NOT RUN",
                **{k: v for k, v in RETENTION_PREPARED.items() if k not in {"prepared", "mode"}}}
            return args.preparation_report
        capability.current()
        capability._same_claim("claim.json")
        if args.action == "cleanup-probes":
            receipt = None
            try:
                if not os.path.lexists(inputs["receipt_path"]) and not os.path.lexists(capability.path / "consumed.json"):
                    receipt = Receipt(inputs["receipt_path"], driver, capability.binding, create=True)
                else:
                    receipt = Receipt(inputs["receipt_path"], driver, capability.binding)
            except Exception:
                pass  # A partial/missing receipt is not permission to replay/replace it.
            outcome = _terminalize(capability, receipt, services, crashed=True)
            args.cleanup_report, args.executed = outcome, 0
            return {"cleanup": outcome, "created_resources": []}
        receipt = Receipt(inputs["receipt_path"], driver, capability.binding)
        assert_run_open(receipt)
        if receipt.last("retention-prepared") != RETENTION_PREPARED:
            raise Blocked("retention preparation ACK required")
        _preflight_retention(services, database, inputs, driver)
        binding(driver, args, data, inputs, services, _qualify_transport=True)
        results = Results("mesh-authn")
        results.receipt = receipt
        if os.path.lexists(capability.path / "consumed.json") or os.path.lexists(capability.path / "terminal.json"):
            raise Blocked("only cleanup-probes may terminalize a consumed/crashed invocation")
        try:
            capability.consume(receipt)
            services.retention = database.retention = capability
            mesh = Mesh(services, database, receipt, credentials, inputs, results)
            receipt.append("execution-started", {"suite": "mesh-authn"})
            mesh.run()
        except Exception:
            results.block_remaining("invocation interrupted; no replay; details redacted")
        finally:
            outcome = (_terminalize(capability, receipt, services) if capability.consumption_owned else
                       {"status": "BLOCKED", "complete": False, "mode": RETAIN_MODE, "removed": 0,
                        "technical_status": "BLOCKED", "residuals": list(RESIDUAL_CLASSES), "uncertainties": ["receipt"]})
        report = {**results.report(), "provider_fixture": "SIMULATED_PROVIDER", "cleanup": outcome}
        report["suite_status"] = "FAIL" if report["failed"] else "BLOCKED" if report["blocked"] else "PASS"
        try:
            capability.current()
            _case_schema(receipt.last("case-observed"))
            launch = not capability.journal_failed
        except Exception:
            launch = False
        report["launch_status"] = "STARTED" if launch else "NOT_PROVEN"
        args.e2e_report, args.executed = report, report["executed"]
        try:
            receipt.append("suite-results", report)
        except Exception:
            outcome.update(status="BLOCKED", uncertainties=list(dict.fromkeys([*outcome["uncertainties"], "receipt"])))
        return report


def run_action(driver, args, data):
    inputs = read_inputs(args.inputs, driver)
    if not _retaining(inputs) and _slot_claimed(driver):
        raise Blocked("claimed retention slot forbids strict mutation or empty cleanup")
    if args.action == "e2e" and args.suite != inputs["suite"]:
        raise Blocked("input suite differs from selected suite")
    for namespace in driver.NAMESPACES.values():
        driver.namespace_owned(data, namespace)
    services = Services(driver, args, data, inputs)
    database = Database(driver, data)
    bound = binding(driver, args, data, inputs, services)
    credentials = _strict_private_json(inputs["credentials_path"])
    if not isinstance(credentials, dict) or set(credentials) != {"password", "internal_token"} or any(
            not isinstance(v, str) or not 8 <= len(v) <= 256 or any(ord(c) < 32 for c in v) for v in credentials.values()):
        raise Blocked("closed private credential schema required")
    if _retaining(inputs):
        return _run_retention(driver, args, data, inputs, services, database, bound, credentials)
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
