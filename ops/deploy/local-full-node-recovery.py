"""Consent-gated reversible interruption of one EXACT leased Kind worker ID."""
import argparse
import importlib.util
import json
import subprocess
from pathlib import Path

spec = importlib.util.spec_from_file_location("storage_runtime", Path(__file__).with_name("local-full-storage.py"))
runtime = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runtime)


def docker(*args):
    if args[0] in {"start", "stop"}:
        runtime.target().check()
    result = subprocess.run(["docker", *args], capture_output=True, text=True, timeout=90)
    if result.returncode:
        raise RuntimeError("owned node Docker operation failed; no forced fallback")
    return result.stdout.strip()


def run(args):
    data = runtime.lease(args.lease)
    if args.node_id not in data["nodeContainerIds"]:
        raise ValueError("node ID is not explicitly leased; name/image is not ownership")
    info = docker("inspect", "--format", '{{.Id}} {{.Name}} {{.State.Running}} {{index .Config.Labels "io.x-k8s.kind.cluster"}} {{index .Config.Labels "io.x-k8s.kind.role"}}', args.node_id).split()
    if len(info) != 5 or info[0] != args.node_id:
        raise ValueError("full exact container ID required")
    if info[3] != runtime.CLUSTER or info[4] != "worker":
        raise ValueError("only an owned aiforall-local-full WORKER may be interrupted")
    node = info[1].lstrip("/")
    if node not in {"aiforall-local-full-worker", "aiforall-local-full-worker2"}:
        raise ValueError("unexpected worker/node mapping")
    running = info[2] == "true"
    if args.action == "plan":
        print(json.dumps({"context": runtime.CONTEXT, "cluster": runtime.CLUSTER, "node": node, "id": args.node_id, "running": running,
                          "loss": "volatile process memory; local-path PVC data stays in this stopped node container",
                          "no_delete_or_drain": True}))
        return
    if not args.confirm_node_interruption:
        raise ValueError("--confirm-node-interruption required; coordinate the parent fixture lease first")
    failures = data.setdefault("nodeFailures", {})
    if args.action == "interrupt":
        if not running or args.node_id in failures:
            raise ValueError("node stopped or prior interruption unresolved; do not adopt/restart it")
        unschedulable = runtime.kubectl("get", "node", node, "-o", "jsonpath={.spec.unschedulable}")
        if unschedulable == "true":
            raise ValueError("worker already cordoned; refuse to alter prior scheduling ownership")
        failures[args.node_id] = {"node": node, "context": runtime.CONTEXT, "cluster": runtime.CLUSTER, "prior_running": True, "phase": "planned"}
        runtime.save_lease(args.lease, data)
        # Cordon is reversible; no drain or Pod/namespace deletion is performed.
        runtime.kubectl("cordon", node)
        failures[args.node_id]["phase"] = "cordoned"
        runtime.save_lease(args.lease, data)
        docker("stop", "--time", "20", args.node_id)
        failures[args.node_id]["phase"] = "stopped"
        data.setdefault("resourceStates", []).append({"id": args.node_id, "context": runtime.CONTEXT, "cluster": runtime.CLUSTER, "priorState": "running", "state": "stopped", "action": "interrupted-owned-worker"})
        runtime.save_lease(args.lease, data)
        print("owned worker stopped gracefully; parent executes bounded readiness/persistence assertions then recover")
    else:
        record = failures.get(args.node_id)
        if not record or not record.get("prior_running") or record["node"] != node or record.get("context") != runtime.CONTEXT or record.get("cluster") != runtime.CLUSTER:
            raise ValueError("no owned interruption record; unknown stopped runtime is never restarted")
        if not running:
            docker("start", args.node_id)
        runtime.kubectl("wait", "--for=condition=Ready", "node/" + node, "--timeout=300s", timeout=330)
        runtime.kubectl("uncordon", node)
        failures.pop(args.node_id)
        data.setdefault("resourceStates", []).append({"id": args.node_id, "context": runtime.CONTEXT, "cluster": runtime.CLUSTER, "priorState": "stopped" if not running else "running", "state": "running", "action": "recovered-owned-worker"})
        runtime.save_lease(args.lease, data)
        print("same owned worker recovered; PVC state retained; no delete/recreate")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["plan", "interrupt", "recover"])
    parser.add_argument("--lease", type=Path, required=True)
    parser.add_argument("--node-id", required=True)
    parser.add_argument("--confirm-node-interruption", action="store_true")
    args = parser.parse_args()
    with runtime.operation_lock(args.lease, args.action):
        run(args)


if __name__ == "__main__":
    main()
