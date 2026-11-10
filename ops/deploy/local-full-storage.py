"""Lease-gated local backup/restore. Never deletes resources or overwrites targets."""
from contextlib import contextmanager
import hashlib
import json
import os
import re
import subprocess
import tarfile
import uuid
from pathlib import Path, PurePosixPath
from local_full_target import LocalClusterTargetAnchor, AnchorError
from local_full_target import private_file, unique_json
from local_full_helm import CHART, parser
import yaml

STORAGE_SOURCES = yaml.safe_load((CHART / "files/storage-sources.yaml").read_text(encoding="utf-8"))["sources"]

CONTEXT = "kind-aiforall-local-full"
CLUSTER = "aiforall-local-full"
SOURCE = "aiforall-local-full"
TARGET_LEASE_PATH = None


def target():
    if TARGET_LEASE_PATH is None:
        raise AnchorError("explicit leasev2 binding required before any API call")
    return LocalClusterTargetAnchor(TARGET_LEASE_PATH, expected_cluster=CLUSTER)


@contextmanager
def operation_lock(path, action):
    prior = json.loads(path.read_text(encoding="utf-8-sig"), object_pairs_hook=unique_json).get("localFullOperation")
    if prior and prior.get("state") != "complete" and action not in {"local-stop", "local-status"}:
        raise ValueError("previous local-full operation is unresolved; parent arbitration required")
    lock = path.with_name(path.name + ".local-full-lock")
    operation = uuid.uuid4().hex
    try:
        with lock.open("x", encoding="utf-8") as file:
            json.dump({"action": action, "pid": os.getpid(), "operation": operation}, file)
    except FileExistsError as error:
        raise ValueError("local-full operation lock exists; parent must resolve active/stale ownership") from error
    try:
        yield operation
    finally:
        if json.loads(lock.read_text(encoding="utf-8")).get("operation") != operation:
            raise ValueError("operation lock ownership changed; retain the marker and arbitrate")
        lock.unlink()


def kubectl(*args, binary=False, stdin=None, timeout=180):
    output = target().kubectl(list(args), input=stdin, text=not binary, timeout=timeout)
    return output if binary else output.strip()


def lease(path):
    global TARGET_LEASE_PATH
    candidate = LocalClusterTargetAnchor(path, expected_cluster=CLUSTER)
    data = candidate.check()
    TARGET_LEASE_PATH = Path(path)
    return data


def save_lease(path, data):
    temporary = path.with_name(path.name + ".storage-update-" + uuid.uuid4().hex)
    with temporary.open("x", encoding="utf-8") as target:
        json.dump(data, target, indent=2)
    temporary.replace(path)


def assert_namespace_owned(namespace, data):
    if namespace not in {SOURCE, "aiforall-local-apparatus"}:
        raise ValueError("backup source must be a declared local-full namespace")
    uid = kubectl("get", "namespace", namespace, "-o", "jsonpath={.metadata.uid}")
    if data.get("namespaceUids", {}).get(namespace) != uid:
        raise ValueError("source namespace UID not owned by the supplied lease")


def create(obj, args, data):
    result = json.loads(kubectl("create", "-f", "-", "-o", "json", stdin=json.dumps(obj)))
    metadata = result["metadata"]
    data.setdefault("ownedResources", []).append({"kind": obj["kind"], "name": metadata["name"], "namespace": metadata.get("namespace"), "uid": metadata["uid"], "context": CONTEXT, "cluster": CLUSTER})
    if obj["kind"] == "Namespace":
        data.setdefault("namespaceUids", {})[metadata["name"]] = metadata["uid"]
    save_lease(args.lease, data)
    return result


def running_pod(namespace, name):
    raw = kubectl("get", "pods", "-n", namespace, "-l", "app.kubernetes.io/name=" + name,
                  "-o", "jsonpath={range .items[*]}{.metadata.name}{' '}{.status.phase}{'\\n'}{end}")
    pods = [line.split()[0] for line in raw.splitlines() if line.split()[-1] == "Running"]
    if len(pods) != 1:
        raise ValueError("expected exactly one running source pod")
    return pods[0]


def pvc_identity(namespace, name):
    uid = kubectl("get", "pvc", name, "-n", namespace, "-o", "jsonpath={.metadata.uid}")
    pv = kubectl("get", "pvc", name, "-n", namespace, "-o", "jsonpath={.spec.volumeName}")
    if not uid or not pv:
        raise ValueError("persistent source PVC must be bound")
    return {"name": name, "uid": uid, "volume": pv, "namespace": namespace}


def fingerprint(namespace, pod, database):
    # Relation counts only: no row contents, credentials or backup bytes logged.
    sql = "SELECT format('SELECT %L AS relation, count(*) FROM %I.%I;', schemaname||'.'||tablename, schemaname, tablename) FROM pg_tables WHERE schemaname NOT IN ('pg_catalog','information_schema') ORDER BY schemaname,tablename;"
    statements = kubectl("exec", "-n", namespace, pod, "--", "psql", "-U", "postgres", "-d", database, "-At", "-c", sql)
    counts = kubectl("exec", "-i", "-n", namespace, pod, "--", "psql", "-U", "postgres", "-d", database, "-At", "-v", "ON_ERROR_STOP=1", stdin=statements)
    return hashlib.sha256(counts.encode()).hexdigest()


def volume_pod(namespace, name, claim, read_only):
    from local_full_helm import storage
    return storage("volume", namespace, name, claim, read_only=read_only)[0]


def postgres_restore_pod(namespace, name, claim, database):
    from local_full_helm import storage
    return storage("postgres", namespace, name, claim, database=database)[0]


def guarded_replicas(namespace, name, uid, replicas, expected):
    current = json.loads(kubectl("get", "deployment", name, "-n", namespace, "-o", "json"))
    if current["metadata"]["uid"] != uid or current["spec"].get("replicas", 1) != expected:
        raise ValueError("backup source UID/replicas changed; no adoption or replica rollback")
    if replicas == expected:
        return
    patch = [{"op": "test", "path": "/metadata/uid", "value": uid},
             {"op": "test", "path": "/metadata/resourceVersion", "value": current["metadata"]["resourceVersion"]},
             {"op": "test", "path": "/spec/replicas", "value": expected},
             {"op": "replace", "path": "/spec/replicas", "value": replicas}]
    ack = json.loads(kubectl("patch", "deployment/" + name, "-n", namespace, "--type=json", "-p", json.dumps(patch), "-o", "json"))
    if ack["metadata"]["uid"] != uid or ack["spec"]["replicas"] != replicas:
        raise ValueError("backup replica mutation ACK differs; source retained for arbitration")


def backup(args, data):
    from local_registry import pki
    pki.checked_path(args.archive)
    pki.private_directory(args.archive.parent)
    source = STORAGE_SOURCES[args.volume]
    namespace = source["namespace"]
    assert_namespace_owned(namespace, data)
    if args.archive.exists() or args.archive.with_suffix(args.archive.suffix + ".json").exists():
        raise ValueError("backup output already exists; no overwrite")
    if not args.archive.parent.is_dir():
        raise ValueError("archive parent directory must already exist")
    name, claim = source["deployment"], source["claim"]
    identity = pvc_identity(namespace, claim)
    deployment_uid = kubectl("get", "deployment", name, "-n", namespace, "-o", "jsonpath={.metadata.uid}")
    owned = data.get("ownedResources", [])
    if not any(o.get("kind") == "Deployment" and o.get("namespace") == namespace and o.get("name") == name and o.get("uid") == deployment_uid
               and o.get("context") == CONTEXT and o.get("cluster") == CLUSTER for o in owned):
        raise ValueError("source Deployment UID is not explicitly owned")
    if not any(o.get("kind") == "PersistentVolumeClaim" and o.get("namespace") == namespace and o.get("name") == claim and o.get("uid") == identity["uid"]
               and o.get("context") == CONTEXT and o.get("cluster") == CLUSTER for o in owned):
        raise ValueError("source PVC UID is not explicitly owned")
    original_replicas = None
    staging_pod = None
    staging_uid = None
    source_quiesced = False
    stop_verified = True
    data["localFullOperation"] = {"id": uuid.uuid4().hex, "phase": "backup", "state": "in-progress",
                                 "sourceUid": deployment_uid, "sourcePvcUid": identity["uid"]}
    save_lease(args.lease, data)
    try:
        if source["format"] == "postgres":
            pod = running_pod(namespace, name)
            before = fingerprint(namespace, pod, args.database)
            # Custom binary output goes through Python bytes, not PowerShell redirection.
            contents = kubectl("exec", "-n", namespace, pod, "--", "pg_dump", "-U", "postgres", "-d", args.database, "-Fc", binary=True, timeout=300)
            after = fingerprint(namespace, pod, args.database)
            if before != after:
                raise ValueError("database changed during backup; quiesce application writers and retry with a NEW archive")
            counts_hash = before
        else:
            if not args.confirm_quiesce:
                raise ValueError("volume backup interrupts the owned workload; --confirm-quiesce required")
            original_replicas = int(kubectl("get", "deployment", name, "-n", namespace, "-o", "jsonpath={.spec.replicas}"))
            data["localFullOperation"]["originalReplicas"] = original_replicas
            save_lease(args.lease, data)
            guarded_replicas(namespace, name, deployment_uid, 0, original_replicas)
            source_quiesced = True
            kubectl("wait", "--for=delete", "pod", "-n", namespace, "-l", "app.kubernetes.io/name=" + name, "--timeout=120s")
            staging_pod = "backup-volume-" + uuid.uuid4().hex[:12]
            acknowledged = create(volume_pod(namespace, staging_pod, claim, True), args, data)
            staging_uid = acknowledged["metadata"]["uid"]
            kubectl("wait", "--for=condition=Ready", "pod/" + staging_pod, "-n", namespace, "--timeout=180s")
            contents = kubectl("exec", "-n", namespace, staging_pod, "--", "tar", "-C", "/data", "-czf", "-", ".", binary=True, timeout=300)
            counts_hash = None
        with args.archive.open("xb") as target:
            target.write(contents)
        metadata = {"volume": args.volume, "format": source["format"], "database": args.database if source["format"] == "postgres" else None, "source": identity,
                    "archive_sha256": hashlib.sha256(contents).hexdigest(), "relation_counts_sha256": counts_hash}
        with args.archive.with_suffix(args.archive.suffix + ".json").open("x", encoding="utf-8") as target:
            json.dump(metadata, target, indent=2)
    finally:
        try:
            if staging_pod:
                # No stop by name after a lost ACK, and no PVC reuse until the
                # exact acknowledged helper has completed its process stop.
                try:
                    if not staging_uid or kubectl("get", "pod", staging_pod, "-n", namespace, "-o", "jsonpath={.metadata.uid}") != staging_uid:
                        raise ValueError("backup helper UID ACK lost or changed")
                    kubectl("exec", "-n", namespace, staging_pod, "--", "sh", "-c", "kill -TERM 1")
                    kubectl("wait", "--for=jsonpath={.status.phase}=Succeeded", "pod/" + staging_pod, "-n", namespace, "--timeout=60s")
                except (ValueError, RuntimeError, subprocess.TimeoutExpired):
                    stop_verified = False
                    print("WARNING: owned backup Pod stop unverified; parent must release it before PVC reuse")
        finally:
            if source_quiesced and stop_verified:
                guarded_replicas(namespace, name, deployment_uid, original_replicas, 0)
                kubectl("rollout", "status", "deployment/" + name, "-n", namespace, "--timeout=180s")
    if not stop_verified:
        raise ValueError("backup bytes retained but helper stop is unverified; operation receipt remains incomplete")
    data["localFullOperation"]["state"] = "complete"
    save_lease(args.lease, data)
    print("backup created; contents not printed; PVC retained")


def restore(args, data):
    if not args.confirm_restore:
        raise ValueError("--confirm-restore required: creates an isolated target, never overwrites a source")
    if not re.fullmatch(r"aiforall-local-restore-[a-z0-9][a-z0-9-]{0,31}", args.target_namespace or ""):
        raise ValueError("target namespace must be a new aiforall-local-restore-* namespace")
    from local_registry import pki
    for path in (args.archive, args.archive.with_suffix(args.archive.suffix + ".json")):
        pki.checked_path(path)
        if not private_file(path):
            raise ValueError("private original backup/archive metadata required")
    metadata = json.loads(args.archive.with_suffix(args.archive.suffix + ".json").read_text())
    if metadata.get("volume") not in STORAGE_SOURCES:
        raise ValueError("unknown declared backup source")
    format = STORAGE_SOURCES[metadata["volume"]]["format"]
    if metadata.get("format", format) != format:
        raise ValueError("archive format differs from its declared source")
    assert_namespace_owned(metadata["source"]["namespace"], data)
    if not any(e.get("kind") == "PersistentVolumeClaim" and e.get("namespace") == metadata["source"]["namespace"]
               and e.get("name") == metadata["source"]["name"] and e.get("uid") == metadata["source"]["uid"]
               and e.get("context") == CONTEXT and e.get("cluster") == CLUSTER for e in data.get("ownedResources", [])):
        raise ValueError("archive source PVC UID is not acknowledged by the original lease")
    contents = args.archive.read_bytes()
    if hashlib.sha256(contents).hexdigest() != metadata["archive_sha256"]:
        raise ValueError("archive checksum mismatch")
    if metadata["source"]["namespace"] == args.target_namespace:
        raise ValueError("source/target namespace collision")
    if format == "postgres" and (not args.target_database or args.target_database == metadata["database"]):
        raise ValueError("restore database must be distinct from the source")
    if kubectl("get", "namespace", args.target_namespace, "--ignore-not-found", "-o", "name"):
        raise ValueError("target namespace already exists; refuse even an apparently empty target")
    if format != "postgres":
        with tarfile.open(args.archive) as archive:
            members = archive.getmembers()
            if len(members) > 100000 or sum(m.size for m in members) > 5 * 1024 ** 3:
                raise ValueError("archive exceeds bounded local restore budget")
            for member in members:
                path = PurePosixPath(member.name)
                if path.is_absolute() or ".." in path.parts or not (member.isfile() or member.isdir()):
                    raise ValueError("unsafe archive member")
    namespace = args.target_namespace
    data["localFullOperation"] = {"id": uuid.uuid4().hex, "phase": "restore", "state": "in-progress",
                                 "sourcePvcUid": metadata["source"]["uid"], "targetNamespace": namespace}
    save_lease(args.lease, data)
    claim = "restored-" + metadata["volume"] + "-" + uuid.uuid4().hex[:12]
    if claim == metadata["source"]["name"]:
        raise ValueError("PVC name collision")
    from local_full_helm import storage
    for obj in storage("foundation", namespace, "restore-foundation", claim):
        create(obj, args, data)
    pod = "restore-" + uuid.uuid4().hex[:12]
    obj = postgres_restore_pod(namespace, pod, claim, args.target_database) if format == "postgres" else volume_pod(namespace, pod, claim, False)
    create(obj, args, data)
    kubectl("wait", "--for=condition=Ready", "pod/" + pod, "-n", namespace, "--timeout=240s")
    target_identity = pvc_identity(namespace, claim)
    if target_identity["volume"] == metadata["source"]["volume"] or target_identity["uid"] == metadata["source"]["uid"]:
        raise ValueError("restore target reuses the source backing volume")
    remote = "/tmp/local-full-restore.archive"
    # Stream bytes into a fixed new target path, never log contents.
    kubectl("exec", "-i", "-n", namespace, pod, "--", "sh", "-ec", "test ! -e " + remote + "; cat > " + remote, binary=True, stdin=contents, timeout=300)
    if format == "postgres":
        count = kubectl("exec", "-n", namespace, pod, "--", "psql", "-U", "postgres", "-d", args.target_database, "-At", "-c", "SELECT count(*) FROM pg_tables WHERE schemaname NOT IN ('pg_catalog','information_schema');")
        if count != "0":
            raise ValueError("restore database is not empty")
        kubectl("exec", "-n", namespace, pod, "--", "pg_restore", "-U", "postgres", "--exit-on-error", "--no-owner", "--no-privileges", "-d", args.target_database, remote, timeout=300)
        if fingerprint(namespace, pod, args.target_database) != metadata["relation_counts_sha256"]:
            raise ValueError("restored relation counts differ from the backup")
    else:
        kubectl("exec", "-n", namespace, pod, "--", "sh", "-ec", "test -z \"$(find /data -mindepth 1 -maxdepth 1 -print -quit)\"; tar -xzf " + remote + " -C /data --no-same-owner", timeout=300)
    # Target is deliberately not wired over any live workload. Parent must
    # release this Pod after assertions; preserve PVC/namespace for inspection.
    data["localFullOperation"]["state"] = "complete"
    save_lease(args.lease, data)
    print(json.dumps({"result": "restored-distinct-target", "namespace": namespace, "pod": pod, "pvc": claim, "source_not_targeted": True}))


def main():
    os.umask(0o077)
    args = parser("storage", choices={"--volume": list(STORAGE_SOURCES)}).parse_args()
    for name in [args.database, args.target_database]:
        if name is not None and not re.fullmatch(r"[a-z][a-z0-9_]{0,62}", name):
            raise ValueError("invalid explicit database identifier")
    with operation_lock(args.lease, args.action):
        data = lease(args.lease)
        if args.action == "backup":
            backup(args, data)
        else:
            restore(args, data)


if __name__ == "__main__":
    main()
