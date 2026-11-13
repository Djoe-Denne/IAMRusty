"""Pinned Docker launcher for the real OAuth case on disposable hosted E2E only.

The parent owns provisioning, TLS trust, named-check reception and all deltas.
This helper executes the unchanged browser scenario, not a replacement probe.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import socket
import sys

from binaries import ROOT, Refused, require, sha, strict_json
import browser as browser_profile
import browser_container as trial
from runtime import hosted, write_json
from safety import Child, Rejected, CODES, BROWSER_DIAGNOSTICS, child_environment, new_output, private_path

HERE = Path(__file__).resolve().parent
OWNER = "k6-ci-oauth-browser"
ADDITIONAL_FILES = ("ci/campaign_browser.py", "ci/runtime.py", "scenarios/auth/oauth-browser.js",
                    "lib/summary.js", "lib/campaign-target.js")
RESULT_FIELDS = {"qualification", "run_id", "exit", "cleanup", "sandbox", "renderer_observed",
                 "witnesses", "diagnostic", "process_security", "startup"}
STARTUP_CAUSES = {"NO_USABLE_SANDBOX", "NAMESPACE_DENIED", "SUID_HELPER", "CRASHPAD", "LIBRARY_LOAD",
                  "RESOURCE_EXHAUSTION", "PROFILE_LOCK", "FILESYSTEM_SANDBOX_FAILURE", "DISPLAY_SERVER_MISSING",
                  "GRAPHICS_INIT_FAILURE", "UNIX_SOCKET_PATH_TOO_LONG"}
CONTAINER_OUTPUT = "/run/case"
WITNESS_FIELDS = {"pid", "uid", "start_ticks", "seccomp_mode", "filters", "additional_filter",
                  "user_namespace_isolated", "pid_namespace_isolated", "no_new_privileges", "forbidden_flags_absent"}


def record(runtime, event, identity, safe):
    require(re.fullmatch(r"[a-f0-9]{64}", identity), "E2E_BROWSER_CONTAINER_ID")
    row = dict(timestamp=datetime.now(timezone.utc).isoformat(), event=event, kind="container", id=identity,
               safe_to_delete=safe, owner=OWNER, run_id=runtime.state["job_run_id"],
               run_attempt=runtime.state["job_attempt"], campaign=runtime.state["run_id"])
    registry = ROOT / ".tmp" / ("docker-resources-" + datetime.now(timezone.utc).strftime("%Y%m%d") + ".log")
    registry.parent.mkdir(exist_ok=True)
    for path in (registry, runtime.directory / "private/browser-docker-resources.jsonl"):
        with path.open("a", encoding="utf-8") as stream:
            stream.write(json.dumps(row, sort_keys=True) + "\n")
            stream.flush()
            os.fsync(stream.fileno())


def prepare(runtime, browser_directory):
    hosted()
    require("browser_container" not in runtime.state, "E2E_BROWSER_REPLAY")
    security = strict_json(runtime.docker(["info", "--format", "{{json .SecurityOptions}}"]))
    require("name=apparmor" in security and "name=seccomp,profile=builtin" in security, "E2E_BROWSER_DAEMON_SECURITY")
    package = new_output(str(runtime.directory / "private/browser-package"))
    context = trial.prepare_context(package, browser_directory)
    for name in ADDITIONAL_FILES:
        target = context / "ops/tests/k6" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(HERE.parent / name, target)
    profile, _derived, facts = trial.acquire_seccomp(package)
    base = strict_json((HERE / "images.lock.json").read_bytes())["runtime"]
    require(re.fullmatch(r"ubuntu:24\.04@sha256:[a-f0-9]{64}", base), "E2E_BROWSER_RUNTIME_PIN")
    before = sorted(runtime.container_ids())
    runtime.state["browser_container"] = dict(profile=str(profile), profile_sha256=sha(profile),
        profile_facts=facts, uid=os.getuid(), gid=os.getgid(), before=before, cleanup="NOT_RUN")
    runtime.save()
    iid = package / "private/image.iid"
    try:
        runtime.docker(["build", "--platform", "linux/amd64", "--iidfile", str(iid), "--build-arg", "RUNTIME_IMAGE=" + base,
                        "--file", str(context / "Dockerfile"), str(context)], timeout=900)
    finally:
        runtime.observe_images()
    identity = iid.read_text().strip()
    require(re.fullmatch(r"sha256:[a-f0-9]{64}", identity), "E2E_BROWSER_IMAGE_ID")
    runtime.state["browser_container"].update(image_id=identity, base=base)
    runtime.save()


def create_arguments(runtime, output, inputs):
    state = runtime.state["browser_container"]
    profile = Path(state["profile"])
    require(profile.is_file() and not profile.is_symlink() and sha(profile) == state["profile_sha256"], "E2E_BROWSER_PROFILE_CHANGED")
    require(state["uid"] != 0 and re.fullmatch(r"sha256:[a-f0-9]{64}", state["image_id"]), "E2E_BROWSER_IMAGE_ID")
    require(all(path.is_absolute() and "," not in str(path) for path in (output, inputs)), "E2E_BROWSER_MOUNT_PATH")
    args = ["create", "--read-only", "--network", "host", "--cap-drop", "ALL",
        "--security-opt", "no-new-privileges", "--security-opt", "seccomp=" + str(profile),
        "--user", f"{state['uid']}:{state['gid']}", "--pids-limit", "512", "--memory", "1536m",
        "--cpus", "2", "--shm-size", "256m", "--tmpfs", "/tmp:rw,nosuid,noexec,size=64m",
        "--mount", "type=bind,src=" + str(output) + ",dst=" + CONTAINER_OUTPUT,
        "--mount", "type=bind,src=" + str(inputs) + ",dst=/run/campaign-input,readonly",
        "--label", "aiforall.dev/oauth-browser-campaign=" + runtime.state["run_id"],
        "--label", "aiforall.dev/oauth-browser-run=" + runtime.state["job_run_id"],
        "--label", "aiforall.dev/oauth-browser-attempt=" + runtime.state["job_attempt"],
        "--entrypoint", "python3"]
    for key in ("GITHUB_ACTIONS", "GITHUB_JOB", "GITHUB_REF", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"):
        args += ["--env", key + "=" + os.environ[key]]
    return [*args, state["image_id"], "-B", "/opt/AIForAll/ops/tests/k6/ci/campaign_browser.py", "run",
            "--output", CONTAINER_OUTPUT, "--run-id", runtime.state["run_id"]]


def accept_result(raw, run_id, uid):
    value = strict_json(raw)
    require(isinstance(value, dict) and value.keys() == RESULT_FIELDS
            and value["qualification"] == "OAUTH_BROWSER_EXECUTION" and value["run_id"] == run_id, "E2E_BROWSER_RESULT_SCHEMA")
    require(value["exit"] is None or type(value["exit"]) is int and -255 <= value["exit"] <= 255, "E2E_BROWSER_RESULT_EXIT")
    require(all(type(value[key]) is bool for key in ("cleanup", "sandbox", "renderer_observed"))
            and value["diagnostic"] in (None, *CODES, "BROWSER_EXECUTION_FAILED"), "E2E_BROWSER_RESULT_FIELDS")
    startup = value["startup"]
    require(isinstance(startup, dict) and startup.keys() == {"browser", "causes"}
            and startup["browser"] in (None, *BROWSER_DIAGNOSTICS)
            and isinstance(startup["causes"], list) and len(startup["causes"]) <= len(STARTUP_CAUSES)
            and all(isinstance(cause, str) and cause in STARTUP_CAUSES for cause in startup["causes"]), "E2E_BROWSER_STARTUP_SCHEMA")
    require(isinstance(value["witnesses"], list) and len(value["witnesses"]) <= 4, "E2E_BROWSER_WITNESS_SCHEMA")
    identities = set()
    for witness in value["witnesses"]:
        require(isinstance(witness, dict) and witness.keys() == WITNESS_FIELDS
                and all(type(witness[key]) is int and 0 < witness[key] < 2**63 for key in ("pid", "uid", "start_ticks", "filters"))
                and witness["uid"] == uid and type(witness["seccomp_mode"]) is int and witness["seccomp_mode"] == 2
                and all(witness[key] is True for key in WITNESS_FIELDS - {"pid", "uid", "start_ticks", "filters", "seccomp_mode"}),
                "E2E_BROWSER_WITNESS_SCHEMA")
        identity = (witness["pid"], witness["uid"], witness["start_ticks"])
        require(identity not in identities, "E2E_BROWSER_DUPLICATE_WITNESS")
        identities.add(identity)
    security = value["process_security"]
    require(isinstance(security, dict) and security.keys() == {"apparmor", "seccomp_mode", "filters", "no_new_privileges"}
            and security["apparmor"] == "DOCKER_DEFAULT" and type(security["seccomp_mode"]) is int and security["seccomp_mode"] == 2
            and type(security["filters"]) is int and security["filters"] >= 1
            and type(security["no_new_privileges"]) is int and security["no_new_privileges"] == 1, "E2E_BROWSER_PROCESS_SECURITY")
    require(all(w["filters"] > security["filters"] for w in value["witnesses"]), "E2E_BROWSER_FILTER_EVIDENCE")
    require(not value["sandbox"] or value["renderer_observed"] and bool(value["witnesses"]), "E2E_BROWSER_WITNESS_REQUIRED")
    return value


def validate_mounts(mounts, output, inputs):
    # Docker may expose --tmpfs in Mounts as well as in HostConfig.Tmpfs.
    binds = [mount for mount in mounts if mount.get("Type") == "bind"]
    temporary = [mount for mount in mounts if mount.get("Type") == "tmpfs"]
    require(len(binds) == 2 and len(temporary) <= 1 and len(mounts) == len(binds) + len(temporary)
            and all(mount.get("Destination") == "/tmp" and mount.get("Source", "") == "" and mount.get("RW") is True for mount in temporary)
            and {(m["Source"], m["Destination"], m["RW"]) for m in binds}
                == {(str(output), CONTAINER_OUTPUT, True), (str(inputs), "/run/campaign-input", False)}, "E2E_BROWSER_MOUNTS")


def execute(runtime, output):
    hosted()
    state = runtime.state["browser_container"]
    require("container_id" not in state, "E2E_BROWSER_REPLAY")
    inputs = runtime.directory / "private/browser-input"
    require(not inputs.exists(), "E2E_BROWSER_INPUT_REPLAY")
    inputs.mkdir(mode=0o700)
    for source, name in (("options.json", "options.json"), ("target.json", "target.json"), ("pki/ca-bundle.crt", "ca-bundle.crt")):
        shutil.copyfile(runtime.directory / "private" / source, inputs / name)
        (inputs / name).chmod(0o400)
    args = create_arguments(runtime, output, inputs)
    identity = runtime.docker(args).decode().strip()
    record(runtime, "created", identity, "no")
    state["container_id"] = identity
    runtime.save()
    try:
        inspected = runtime.inspect(identity)
        expected = strict_json(Path(state["profile"]).read_bytes())
        state["policy"] = trial.container_policy(inspected, state["uid"], state["gid"], applied=False,
                                                  expected_seccomp=expected, expected_network="host")
        validate_mounts(inspected.get("Mounts", []), output, inputs)
        runtime.save()
        runtime.docker(["start", identity])
        status = runtime.docker(["wait", identity], timeout=160).decode().strip()
        require(re.fullmatch(r"[0-9]{1,3}", status), "E2E_BROWSER_CONTAINER_EXIT")
        path = output / "private/browser-result.json"
        require(path.is_file() and not path.is_symlink() and path.stat().st_size <= 65536, "E2E_BROWSER_RESULT_MISSING")
        result = accept_result(path.read_bytes(), runtime.state["run_id"], state["uid"])
        state["result"] = result
        state["container_exit"] = int(status)
        runtime.save()
        require(int(status) == 0, "E2E_BROWSER_CONTAINER_FAILED")
        return result
    finally:
        clean(runtime)


def browser_receipts(runtime):
    path = runtime.directory / "private/browser-docker-resources.jsonl"
    if not path.exists():
        return []
    require(path.is_file() and not path.is_symlink() and path.stat().st_size <= 65536, "E2E_BROWSER_RECEIPTS")
    return [strict_json(line) for line in path.read_bytes().splitlines() if line.strip()]


def clean(runtime):
    state = runtime.state.get("browser_container")
    if not state:
        return
    hosted()
    require(runtime.state["host"] == socket.gethostname()
            and runtime.state["boot_id"] == Path("/proc/sys/kernel/random/boot_id").read_text().strip(), "E2E_BROWSER_FOREIGN_HOST")
    rows = browser_receipts(runtime)
    created = {row["id"] for row in rows if row.get("event") == "created"}
    identity = state.get("container_id")
    if identity is None and created:
        require(len(created) == 1, "E2E_BROWSER_RECEIPT_CONFLICT")
        identity = next(iter(created))
        state["container_id"] = identity
    if identity:
        require(identity in created and created == {identity} and identity not in state["before"], "E2E_BROWSER_OWNERSHIP")
        for registry in sorted((ROOT / ".tmp").glob("docker-resources-*.log")):
            require(not registry.is_symlink() and registry.stat().st_size <= 4 * 1024**2, "E2E_BROWSER_REGISTRY_LIMIT")
            rows += [strict_json(line) for line in registry.read_bytes().splitlines() if line.strip()]
        for row in rows:
            if row.get("kind") == "container" and row.get("id") == identity:
                require(row.get("owner") == OWNER and row.get("run_id") == runtime.state["job_run_id"]
                        and row.get("run_attempt") == runtime.state["job_attempt"] and row.get("campaign") == runtime.state["run_id"]
                        and row.get("safe_to_delete") in ("yes", "no"), "E2E_BROWSER_RECEIPT_CONFLICT")
        if identity in runtime.container_ids():
            value = runtime.inspect(identity)
            labels = value["Config"].get("Labels", {})
            require(value["Id"] == identity and value["Image"] == state["image_id"]
                    and labels.get("aiforall.dev/oauth-browser-campaign") == runtime.state["run_id"]
                    and labels.get("aiforall.dev/oauth-browser-run") == runtime.state["job_run_id"]
                    and labels.get("aiforall.dev/oauth-browser-attempt") == runtime.state["job_attempt"], "E2E_BROWSER_IDENTITY")
            if value["State"]["Running"]:
                runtime.docker(["stop", "--time", "20", identity], timeout=30)
            require(runtime.inspect(identity)["State"]["Running"] is False, "E2E_BROWSER_STOP_UNPROVEN")
            record(runtime, "stopped", identity, "no")
            record(runtime, "reclassified", identity, "yes")
            runtime.docker(["rm", identity])
            record(runtime, "removed", identity, "yes")
        require(identity not in runtime.container_ids(), "E2E_BROWSER_CLEANUP_INCOMPLETE")
    remaining = runtime.docker(["container", "ls", "--all", "--no-trunc", "--format", "{{.ID}}", "--filter",
        "label=aiforall.dev/oauth-browser-campaign=" + runtime.state["run_id"]]).strip()
    require(not remaining, "E2E_BROWSER_UNRECEIPTED_CONTAINER")
    state["cleanup"] = "PASS"
    runtime.save()


def in_container(output, run_id):
    hosted()
    require(Path("/.dockerenv").is_file() and re.fullmatch(r"[a-f0-9]{32}", run_id), "E2E_BROWSER_CONTAINER_REQUIRED")
    os.umask(0o077)
    output = Path(output)
    private_path(output, directory=True)
    inputs = Path("/run/campaign-input")
    pins = browser_profile.tools(Path("/opt/browser"))
    child = Child()
    security = trial.process_security(Path("/proc/self/attr/current").read_text(), Path("/proc/self/status").read_text())
    result = dict(qualification="OAUTH_BROWSER_EXECUTION", run_id=run_id, diagnostic=None, process_security=security)
    result["startup"] = dict(browser=None, causes=[])
    env = child_environment(output)
    env.update(K6_RUN_ID=run_id, K6_ORIGIN="https://127.0.0.1:18443", K6_CAMPAIGN_FILE=str(inputs / "target.json"),
        K6_SUMMARY_PATH=str(output / "private/summary.json"), SSL_CERT_FILE=str(inputs / "ca-bundle.crt"),
        K6_BROWSER_EXECUTABLE_PATH=pins["chromium"]["path"], K6_EXPECTED_BROWSER_VERSION=pins["chromium"]["version"],
        K6_BROWSER_ARGS="enable-logging,log-level=2,log-file=" + str(output / "private/chromium.log"))
    try:
        require(security["apparmor"] == "DOCKER_DEFAULT" and security["seccomp_mode"] == 2
                and security["no_new_privileges"] == 1 and type(security["filters"]) is int and security["filters"] >= 1,
                "E2E_BROWSER_PROCESS_SECURITY")
        code = child.execute([pins["k6"]["path"], "run", "--quiet", "--no-usage-report", "--address", "127.0.0.1:0",
                              "--config", str(inputs / "options.json"), str(HERE.parent / "scenarios/auth/oauth-browser.js")],
                             output=output, env=env, timeout=140, browser=True)
        require(code == 0 and child.cleanup and child.sandbox and child.renderer_observed, "E2E_BROWSER_CHILD_EVIDENCE")
    except Rejected as error:
        result["diagnostic"] = error.code
        result["startup"]["browser"] = getattr(error, "browser_diagnostic", None)
    except Exception:
        result["diagnostic"] = "BROWSER_EXECUTION_FAILED"
    finally:
        result["startup"]["causes"] = sorted(set(browser_profile.native_diagnostic(child.stderr)["causes"]
            + browser_profile.private_log_diagnostic(output / "private/chromium.log")["causes"]))
        result.update(exit=child.code, cleanup=child.cleanup, sandbox=child.sandbox,
                      renderer_observed=child.renderer_observed, witnesses=child.sandbox_proof)
        write_json(output / "private/browser-result.json", result)
    return 0 if result["diagnostic"] is None else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("run",))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--run-id", required=True)
    args = parser.parse_args()
    try:
        sys.exit(in_container(args.output, args.run_id))
    except Exception:
        print("OAuth browser execution incomplete; no raw diagnostics exported")
        sys.exit(1)
