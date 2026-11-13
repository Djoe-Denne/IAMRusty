"""Independent always() closure of packaging probes, anchored to creation receipts."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re

from binaries import ROOT, Refused, command, require, strict_json
from artifacts import current_ci_source

OWNER = "k6-ci-packaging"


def owned_receipts(records, run_id, attempt):
    created = set()
    for item in records:
        if (item.get("kind") == "container" and item.get("event") == "created"
                and item.get("owner") == OWNER and item.get("run_id") == run_id
                and item.get("run_attempt") == attempt):
            identity = item.get("id", "")
            require(isinstance(identity, str) and re.fullmatch(r"[a-f0-9]{64}", identity), "CLEANUP_RECEIPT_ID")
            created.add(identity)
    for identity in created:
        for item in records:
            if item.get("kind") != "container" or item.get("id") != identity:
                continue
            require(item.get("owner") == OWNER and item.get("run_id") == run_id
                    and item.get("run_attempt") == attempt and item.get("safe_to_delete") != "ask",
                    "CLEANUP_RECEIPT_CONFLICT")
    return created


def close_probes(owned, present, candidates, inspect, stop, remove, append, run_id, attempt):
    """Labels detect an unreceipted create gap; they never grant ownership."""
    removed = []
    for identity in sorted(owned & present):
        value = inspect(identity)
        labels = value.get("Config", {}).get("Labels", {})
        require(value.get("Id") == identity and labels.get("aiforall.dev/k6-packaging-run") == run_id
                and labels.get("aiforall.dev/k6-packaging-attempt") == attempt, "CLEANUP_IDENTITY_CONFLICT")
        if value["State"]["Running"]:
            stop(identity)
        value = inspect(identity)
        require(value.get("Id") == identity and value["State"]["Running"] is False, "CLEANUP_PROBE_STILL_RUNNING")
        append("stopped", identity, "no")
        append("reclassified", identity, "yes")
        remove(identity)
        append("removed", identity, "yes")
        removed.append(identity)
    require(candidates <= owned, "UNRECEIPTED_PROBE_CLEANUP_INCOMPLETE")
    return removed


def cleanup(output, environment=None):
    env = dict(os.environ if environment is None else environment)
    require(env.get("GITHUB_ACTIONS") == "true" and (env.get("GITHUB_JOB") == "package-images"
            or current_ci_source(env, env.get("GITHUB_RUN_ID"), env.get("GITHUB_RUN_ATTEMPT"), env.get("GITHUB_SHA"))),
            "HOSTED_PACKAGING_CLEANUP_ONLY")
    run_id, attempt = env["GITHUB_RUN_ID"], env["GITHUB_RUN_ATTEMPT"]
    require(re.fullmatch(r"[1-9][0-9]*", run_id) and re.fullmatch(r"[1-9][0-9]*", attempt), "CLEANUP_RUN_ID")
    output = Path(output)
    require(output.is_absolute() and not output.is_symlink(), "CLEANUP_OUTPUT")
    output.mkdir(mode=0o700, exist_ok=True)
    report = dict(schema="aiforall.k6-packaging-cleanup/v1", run_id=run_id, run_attempt=attempt,
                  status="INCOMPLETE", owned_running=None)
    current_records = []
    registry = ROOT / ".tmp" / ("docker-resources-" + datetime.now(timezone.utc).strftime("%Y%m%d") + ".log")
    registry.parent.mkdir(exist_ok=True)

    def append(event, identity, safe):
        row = dict(timestamp=datetime.now(timezone.utc).isoformat(), event=event, kind="container", id=identity,
                   safe_to_delete=safe, owner=OWNER, run_id=run_id, run_attempt=attempt)
        with registry.open("a", encoding="utf-8") as stream:
            stream.write(json.dumps(row, sort_keys=True) + "\n")
        current_records.append(row)

    def listing(filtered=False):
        args = ["docker", "container", "ls", "--all", "--no-trunc", "--format", "{{.ID}}"]
        if filtered:
            args += ["--filter", "label=aiforall.dev/k6-packaging-run=" + run_id,
                     "--filter", "label=aiforall.dev/k6-packaging-attempt=" + attempt]
        values = set(command(args).splitlines())
        require(all(re.fullmatch(r"[a-f0-9]{64}", value) for value in values), "CLEANUP_DOCKER_INVENTORY")
        return values

    try:
        records = []
        for path in sorted((ROOT / ".tmp").glob("docker-resources-*.log")):
            require(path.stat().st_size <= 4 * 1024 ** 2 and not path.is_symlink(), "CLEANUP_REGISTRY_LIMIT")
            for line in path.read_bytes().splitlines():
                if line.strip():
                    item = strict_json(line)
                    require(isinstance(item, dict), "CLEANUP_REGISTRY_SCHEMA")
                    records.append(item)
        current_records.extend(item for item in records if item.get("owner") == OWNER
                               and item.get("run_id") == run_id and item.get("run_attempt") == attempt)
        owned = owned_receipts(records, run_id, attempt)
        present, candidates = listing(), listing(True)
        removed = close_probes(owned, present, candidates,
                              lambda identity: strict_json(command(["docker", "inspect", "--format", "{{json .}}", identity])),
                              lambda identity: command(["docker", "stop", "--time", "10", identity]),
                              lambda identity: command(["docker", "rm", identity]), append, run_id, attempt)
        after, late = listing(), listing(True)
        require(not (owned & after) and not late, "CLEANUP_FINAL_INVENTORY_INCOMPLETE")
        report.update(status="PASS", owned_running=0, owned_count=len(owned), removed=removed,
                      already_absent=sorted(owned - present))
    except Refused as error:
        report["reason"] = str(error)
        raise
    finally:
        (output / "cleanup.json").write_text(json.dumps(report, sort_keys=True), encoding="utf-8")
        with (output / "docker-resources.jsonl").open("w", encoding="utf-8") as stream:
            for item in current_records:
                stream.write(json.dumps(item, sort_keys=True) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        cleanup(args.output)
    except Refused as error:
        raise SystemExit("Packaging probe cleanup incomplete: " + str(error)) from None
    except Exception:
        raise SystemExit("Packaging probe cleanup incomplete; final inventory not proven") from None
    print("Independent packaging probe cleanup PASS; no owned containers remain")


if __name__ == "__main__":
    main()
