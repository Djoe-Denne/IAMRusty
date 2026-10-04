"""Lease-gated local backup/restore. Never deletes resources or overwrites targets."""
import argparse
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
    lock = path.with_name(path.name + ".local-full-lock")
    try:
        with lock.open("x", encoding="utf-8") as file:
            json.dump({"action": action, "pid": os.getpid()}, file)
    except FileExistsError as error:
        raise ValueError("local-full operation lock exists; parent must resolve active/stale ownership") from error
    try:
        yield
    finally:
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
    result = {"apiVersion": "v1", "kind": "Pod", "metadata": {"name": name, "namespace": namespace, "labels": {"aiforall.dev/profile": "local-full", "app.kubernetes.io/name": "local-full-storage"}}, "spec": {
        "automountServiceAccountToken": False, "restartPolicy": "Never", "activeDeadlineSeconds": 900,
        "securityContext": {"runAsNonRoot": True, "runAsUser": 65534, "runAsGroup": 65534, "fsGroup": 65534, "seccompProfile": {"type": "RuntimeDefault"}},
        "containers": [{"name": "storage", "image": "postgres:15.12-alpine3.21@sha256:ef9d1517df69c4d27dbb9ddcec14f431a2442628603f4e9daa429b92ae6c3cd1", "command": ["sh", "-c", "trap 'exit 0' TERM INT; sleep 900 & wait"],
                        "resources": {"requests": {"cpu": "10m", "memory": "32Mi"}, "limits": {"cpu": "200m", "memory": "128Mi"}},
                        "securityContext": {"allowPrivilegeEscalation": False, "capabilities": {"drop": ["ALL"]}},
                        "volumeMounts": [{"name": "data", "mountPath": "/data", "readOnly": read_only}]}],
        "volumes": [{"name": "data", "persistentVolumeClaim": {"claimName": claim, "readOnly": read_only}}]}}
    if read_only:
        result["spec"]["securityContext"].pop("fsGroup")
    return result


def postgres_restore_pod(namespace, name, claim, database):
    pod = volume_pod(namespace, name, claim, False)
    pod["spec"]["securityContext"] = {"runAsNonRoot": True, "runAsUser": 70, "runAsGroup": 70, "fsGroup": 70, "seccompProfile": {"type": "RuntimeDefault"}}
    container = pod["spec"]["containers"][0]
    container.pop("command")
    container["env"] = [{"name": "POSTGRES_USER", "value": "postgres"}, {"name": "POSTGRES_PASSWORD", "value": "postgres"}, {"name": "POSTGRES_DB", "value": database}, {"name": "PGDATA", "value": "/data/pgdata"}]
    container["readinessProbe"] = {"exec": {"command": ["pg_isready", "-U", "postgres", "-d", database]}, "periodSeconds": 5}
    container["resources"]["limits"] = {"cpu": "1", "memory": "512Mi"}
    return pod


def backup(args, data):
    namespace = SOURCE if args.volume != "admission" else "aiforall-local-apparatus"
    assert_namespace_owned(namespace, data)
    if args.archive.exists() or args.archive.with_suffix(args.archive.suffix + ".json").exists():
        raise ValueError("backup output already exists; no overwrite")
    if not args.archive.parent.is_dir():
        raise ValueError("archive parent directory must already exist")
    name = {"postgres": "postgres", "zot": "zot", "admission": "apparatus-operator"}[args.volume]
    claim = {"postgres": "postgres-data", "zot": "zot-data", "admission": "admission-store"}[args.volume]
    identity = pvc_identity(namespace, claim)
    deployment_uid = kubectl("get", "deployment", name, "-n", namespace, "-o", "jsonpath={.metadata.uid}")
    owned = data.get("ownedResources", [])
    if not any(o.get("kind") == "Deployment" and o.get("namespace") == namespace and o.get("name") == name and o.get("uid") == deployment_uid for o in owned):
        raise ValueError("source Deployment UID is not explicitly owned")
    if not any(o.get("kind") == "PersistentVolumeClaim" and o.get("namespace") == namespace and o.get("uid") == identity["uid"] for o in owned):
        raise ValueError("source PVC UID is not explicitly owned")
    original_replicas = None
    staging_pod = None
    try:
        if args.volume == "postgres":
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
            kubectl("scale", "deployment/" + name, "-n", namespace, "--replicas=0")
            kubectl("wait", "--for=delete", "pod", "-n", namespace, "-l", "app.kubernetes.io/name=" + name, "--timeout=120s")
            staging_pod = "backup-volume-" + uuid.uuid4().hex[:12]
            create(volume_pod(namespace, staging_pod, claim, True), args, data)
            kubectl("wait", "--for=condition=Ready", "pod/" + staging_pod, "-n", namespace, "--timeout=180s")
            contents = kubectl("exec", "-n", namespace, staging_pod, "--", "tar", "-C", "/data", "-czf", "-", ".", binary=True, timeout=300)
            counts_hash = None
        with args.archive.open("xb") as target:
            target.write(contents)
        metadata = {"volume": args.volume, "database": args.database if args.volume == "postgres" else None, "source": identity,
                    "archive_sha256": hashlib.sha256(contents).hexdigest(), "relation_counts_sha256": counts_hash}
        with args.archive.with_suffix(args.archive.suffix + ".json").open("x", encoding="utf-8") as target:
            json.dump(metadata, target, indent=2)
        print("backup created; contents not printed; PVC retained")
    finally:
        try:
            if staging_pod:
                # Process stop only; retain exact Pod/PVC UID inventory. Failure
                # to stop must not prevent rollback of the source replicas.
                try:
                    kubectl("exec", "-n", namespace, staging_pod, "--", "sh", "-c", "kill -TERM 1")
                except (RuntimeError, subprocess.TimeoutExpired):
                    print("WARNING: owned backup Pod stop unverified; parent must release it before PVC reuse")
        finally:
            if original_replicas is not None:
                kubectl("scale", "deployment/" + name, "-n", namespace, "--replicas=" + str(original_replicas))
                kubectl("rollout", "status", "deployment/" + name, "-n", namespace, "--timeout=180s")


def restore(args, data):
    if not args.confirm_restore:
        raise ValueError("--confirm-restore required: creates an isolated target, never overwrites a source")
    if not re.fullmatch(r"aiforall-local-restore-[a-z0-9][a-z0-9-]{0,31}", args.target_namespace or ""):
        raise ValueError("target namespace must be a new aiforall-local-restore-* namespace")
    metadata = json.loads(args.archive.with_suffix(args.archive.suffix + ".json").read_text())
    assert_namespace_owned(metadata["source"]["namespace"], data)
    contents = args.archive.read_bytes()
    if hashlib.sha256(contents).hexdigest() != metadata["archive_sha256"]:
        raise ValueError("archive checksum mismatch")
    if metadata["source"]["namespace"] == args.target_namespace:
        raise ValueError("source/target namespace collision")
    if metadata["volume"] == "postgres" and (not args.target_database or args.target_database == metadata["database"]):
        raise ValueError("restore database must be distinct from the source")
    if kubectl("get", "namespace", args.target_namespace, "--ignore-not-found", "-o", "name"):
        raise ValueError("target namespace already exists; refuse even an apparently empty target")
    if metadata["volume"] != "postgres":
        with tarfile.open(args.archive) as archive:
            members = archive.getmembers()
            if len(members) > 100000 or sum(m.size for m in members) > 5 * 1024 ** 3:
                raise ValueError("archive exceeds bounded local restore budget")
            for member in members:
                path = PurePosixPath(member.name)
                if path.is_absolute() or ".." in path.parts or not (member.isfile() or member.isdir()):
                    raise ValueError("unsafe archive member")
    namespace = args.target_namespace
    create({"apiVersion": "v1", "kind": "Namespace", "metadata": {"name": namespace, "labels": {"aiforall.dev/profile": "local-full-restore", "pod-security.kubernetes.io/enforce": "baseline"}}}, args, data)
    create({"apiVersion": "networking.k8s.io/v1", "kind": "NetworkPolicy", "metadata": {"name": "default-deny", "namespace": namespace}, "spec": {"podSelector": {}, "policyTypes": ["Ingress", "Egress"]}}, args, data)
    create({"apiVersion": "v1", "kind": "ResourceQuota", "metadata": {"name": "restore-budget", "namespace": namespace}, "spec": {"hard": {"pods": "2", "persistentvolumeclaims": "1", "requests.storage": "2Gi", "requests.cpu": "1", "requests.memory": "512Mi", "limits.cpu": "2", "limits.memory": "1Gi"}}}, args, data)
    claim = "restored-" + metadata["volume"] + "-" + uuid.uuid4().hex[:12]
    if claim == metadata["source"]["name"]:
        raise ValueError("PVC name collision")
    create({"apiVersion": "v1", "kind": "PersistentVolumeClaim", "metadata": {"name": claim, "namespace": namespace}, "spec": {"accessModes": ["ReadWriteOnce"], "storageClassName": "standard", "resources": {"requests": {"storage": "2Gi"}}}}, args, data)
    pod = "restore-" + uuid.uuid4().hex[:12]
    obj = postgres_restore_pod(namespace, pod, claim, args.target_database) if metadata["volume"] == "postgres" else volume_pod(namespace, pod, claim, False)
    create(obj, args, data)
    kubectl("wait", "--for=condition=Ready", "pod/" + pod, "-n", namespace, "--timeout=240s")
    target_identity = pvc_identity(namespace, claim)
    if target_identity["volume"] == metadata["source"]["volume"] or target_identity["uid"] == metadata["source"]["uid"]:
        raise ValueError("restore target reuses the source backing volume")
    remote = "/tmp/local-full-restore.archive"
    # Stream bytes into a fixed new target path, never log contents.
    kubectl("exec", "-i", "-n", namespace, pod, "--", "sh", "-ec", "test ! -e " + remote + "; cat > " + remote, binary=True, stdin=contents, timeout=300)
    if metadata["volume"] == "postgres":
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
    print(json.dumps({"result": "restored-distinct-target", "namespace": namespace, "pod": pod, "pvc": claim, "source_not_targeted": True}))


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["backup", "restore"])
    parser.add_argument("--lease", type=Path, required=True)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--volume", choices=["postgres", "zot", "admission"], default="postgres")
    parser.add_argument("--database", default="manifesto_dev")
    parser.add_argument("--target-namespace")
    parser.add_argument("--target-database")
    parser.add_argument("--confirm-quiesce", action="store_true")
    parser.add_argument("--confirm-restore", action="store_true")
    args = parser.parse_args()
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
