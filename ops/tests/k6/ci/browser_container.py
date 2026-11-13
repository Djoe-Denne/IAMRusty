"""Baseline non-root Docker sandbox trial; never an application E2E verdict."""
import argparse
import copy
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import sys
import urllib.request

from binaries import ROOT, Refused, require, sha, strict_json
import browser as browser_profile
from browser_audit import native_arguments
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))
from safety import Child, Rejected, child_environment, new_output, private_path

HERE = Path(__file__).resolve().parent
OWNER = "k6-browser-container-trial"
FILES = ("ci/binaries.py", "ci/browser.py", "ci/browser_container.py", "ci/browser-sandbox-probe.js",
         "ci/browser_audit.py", "config/tools.lock.json", "lib/safety.py")
NAMESPACE_RULE = {"comment": "Approved Chromium sandbox namespaces, one disposable non-root container only",
                  "names": ["clone", "setns", "unshare"], "action": "SCMP_ACT_ALLOW", "args": []}


def derive_seccomp(baseline):
    require(isinstance(baseline, dict) and baseline.keys() == {"defaultAction", "defaultErrnoRet", "archMap", "syscalls"}
            and baseline["defaultAction"] == "SCMP_ACT_ERRNO" and baseline["defaultErrnoRet"] == 1
            and isinstance(baseline["syscalls"], list), "TRIAL_SECCOMP_BASELINE")
    require(not any(rule.get("action") == "SCMP_ACT_ALLOW" and not rule.get("args") and not rule.get("includes")
                    and not rule.get("excludes") and set(rule.get("names", [])) & set(NAMESPACE_RULE["names"])
                    for rule in baseline["syscalls"]), "TRIAL_SECCOMP_ALREADY_BROAD")
    result = copy.deepcopy(baseline)
    result["syscalls"].append(copy.deepcopy(NAMESPACE_RULE))
    return result


def acquire_seccomp(root):
    pin = strict_json((HERE / "seccomp.lock.json").read_bytes())
    require(pin.get("schema") == "aiforall.chromium-seccomp/v1" and pin.get("approved_namespace_syscalls") == NAMESPACE_RULE["names"]
            and re.fullmatch(r"[a-f0-9]{40}", pin.get("revision", "")) and re.fullmatch(r"[a-f0-9]{64}", pin.get("sha256", ""))
            and pin["source"] == "https://raw.githubusercontent.com/moby/profiles/" + pin["revision"] + "/seccomp/default.json",
            "TRIAL_SECCOMP_SOURCE_PIN")
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    with opener.open(pin["source"], timeout=30) as response:
        require(response.url == pin["source"], "TRIAL_SECCOMP_REDIRECT")
        raw = response.read(65537)
    require(len(raw) <= 65536 and hashlib.sha256(raw).hexdigest() == pin["sha256"], "TRIAL_SECCOMP_SOURCE_HASH")
    baseline = strict_json(raw)
    derived = derive_seccomp(baseline)
    path = root / "private/chromium.seccomp.json"
    require(not path.exists() and not path.is_symlink(), "TRIAL_SECCOMP_COLLISION")
    write(path, derived)
    path.chmod(0o400)
    return path, derived, {"reference_revision": pin["revision"], "reference_sha256": pin["sha256"],
        "derived_sha256": sha(path), "added_syscalls": NAMESPACE_RULE["names"],
        "basis": "pinned_moby_reference_not_asserted_identical_to_daemon_builtin"}


def write(path, value):
    temporary = path.with_suffix(".pending")
    temporary.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")
    temporary.chmod(0o600)
    temporary.replace(path)


def hosted():
    require(sys.platform == "linux" and os.getuid() != 0 and os.environ.get("GITHUB_ACTIONS") == "true"
            and os.environ.get("GITHUB_JOB") == "trial" and os.environ.get("GITHUB_REF") == "refs/heads/master",
            "HOSTED_CONTAINER_TRIAL_ONLY")


def command(args, timeout=45, any_exit=False):
    result = subprocess.run(args, capture_output=True, timeout=timeout)
    require(len(result.stdout) <= 4 * 1024 ** 2 and len(result.stderr) <= 4 * 1024 ** 2, "TRIAL_COMMAND_OUTPUT_LIMIT")
    require(any_exit or result.returncode == 0, "TRIAL_COMMAND_FAILED")
    return result


def docker(args, **kwargs):
    return command(["docker", *args], **kwargs)


def image_ids():
    return set(docker(["image", "ls", "--no-trunc", "--quiet"]).stdout.decode().splitlines())


def container_ids():
    return set(docker(["container", "ls", "--all", "--no-trunc", "--format", "{{.ID}}"]).stdout.decode().splitlines())


def record(root, state, event, kind, identity, safe):
    require(re.fullmatch(r"(?:sha256:)?[a-f0-9]{64}", identity), "TRIAL_RESOURCE_ID")
    row = dict(timestamp=datetime.now(timezone.utc).isoformat(), owner=OWNER, run_id=state["run_id"],
               run_attempt=state["attempt"], event=event, kind=kind, id=identity, safe_to_delete=safe)
    registry = ROOT / ".tmp" / ("docker-resources-" + datetime.now(timezone.utc).strftime("%Y%m%d") + ".log")
    registry.parent.mkdir(exist_ok=True)
    for path in (registry, root / "private/docker-resources.jsonl"):
        with path.open("a", encoding="utf-8") as stream:
            stream.write(json.dumps(row, sort_keys=True) + "\n")
            stream.flush()
            os.fsync(stream.fileno())


def container_policy(value, uid, gid, applied=True, expected_seccomp=None):
    config, host = value["Config"], value["HostConfig"]
    require(config["User"] == f"{uid}:{gid}" and uid != 0, "TRIAL_REQUIRES_NONROOT")
    require(host["Privileged"] is False and host["ReadonlyRootfs"] is True
            and host["NetworkMode"] == "none" and host.get("PidMode", "") == ""
            and not host.get("CapAdd") and host["CapDrop"] == ["ALL"], "TRIAL_CONTAINER_POLICY")
    options = host["SecurityOpt"]
    require(isinstance(options, list) and all(isinstance(option, str) for option in options)
            and sum(option in ("no-new-privileges", "no-new-privileges=true") for option in options) == 1,
            "TRIAL_SECURITY_OVERRIDE")
    if expected_seccomp is None:
        require(len(options) == 1, "TRIAL_SECURITY_OVERRIDE")
    else:
        profiles = [option.removeprefix("seccomp=") for option in options if option.startswith("seccomp=")]
        require(len(options) == 2 and len(profiles) == 1 and strict_json(profiles[0]) == expected_seccomp,
                "TRIAL_SECCOMP_APPLIED_MISMATCH")
    require(value.get("AppArmorProfile") in (("docker-default",) if applied else ("", "docker-default")),
            "TRIAL_DEFAULT_APPARMOR_UNPROVEN")
    return {"nonroot": True, "readonly_root": True, "network": "none", "capabilities": "ALL_DROPPED",
            "apparmor": "docker-default" if applied else "docker_default_requested",
            "seccomp": "pinned_moby_plus_chromium_namespaces" if expected_seccomp is not None else "docker_default_requested",
            "no_new_privileges": True, "privileged": False}


def process_security(profile_text, status_text):
    fields = dict(line.split(":", 1) for line in status_text.splitlines() if ":" in line)
    result = {"apparmor": "DOCKER_DEFAULT" if profile_text.strip() in ("docker-default", "docker-default (enforce)") else "OTHER"}
    for name, field in (("seccomp_mode", "Seccomp"), ("filters", "Seccomp_filters"), ("no_new_privileges", "NoNewPrivs")):
        value = fields.get(field, "").strip()
        result[name] = int(value) if re.fullmatch(r"[0-9]{1,9}", value) else None
    return result


def create_arguments(state, root):
    mount = str(root / "private/container-data")
    require("," not in mount, "TRIAL_MOUNT_PATH")
    args = ["create", "--read-only", "--network", "none", "--cap-drop", "ALL",
            "--security-opt", "no-new-privileges", "--user", f"{state['uid']}:{state['gid']}",
            "--pids-limit", "512", "--memory", "1536m", "--cpus", "2", "--shm-size", "256m",
            "--tmpfs", "/tmp:rw,nosuid,noexec,size=64m", "--mount", "type=bind,src=" + mount + ",dst=/run/trial",
            "--label", "aiforall.dev/browser-trial-run=" + state["run_id"],
            "--label", "aiforall.dev/browser-trial-attempt=" + state["attempt"]]
    for key in ("GITHUB_ACTIONS", "GITHUB_JOB", "GITHUB_REF", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"):
        args += ["--env", key + "=" + os.environ[key]]
    require(state.get("policy", "baseline") in ("baseline", "chromium"), "TRIAL_POLICY_MODE")
    if state.get("policy") == "chromium":
        path = Path(state["seccomp_path"])
        require(path.is_absolute() and not path.is_symlink() and sha(path) == state["seccomp_sha256"], "TRIAL_SECCOMP_CHANGED")
        args += ["--security-opt", "seccomp=" + str(path)]
    return [*args, state["image_id"]]


def validate_receipt(records, state, identity):
    rows = [row for row in records if row.get("kind") == "container" and row.get("id") == identity]
    require(any(row.get("event") == "created" for row in rows), "TRIAL_CREATION_RECEIPT_REQUIRED")
    require(all(row.get("owner") == OWNER and row.get("run_id") == state["run_id"]
                and row.get("run_attempt") == state["attempt"] and row.get("safe_to_delete") in ("yes", "no")
                for row in rows), "TRIAL_RECEIPT_CONFLICT")


def probe_progress(path):
    path = Path(path)
    if not path.exists():
        return {"state": "ABSENT"}
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 1024:
        return {"state": "WITHHELD"}
    try:
        value = strict_json(path.read_bytes())
        require(isinstance(value, dict) and value.keys() == {"started", "context", "version", "page", "evaluation"}
                and all(type(count) is int and count in (0, 1) for count in value.values()), "TRIAL_STAGE_SCHEMA")
        return {"state": "AVAILABLE", "counts": value}
    except (ValueError, OSError):
        return {"state": "WITHHELD"}


def diagnose_native(root, pins):
    """Second short launch only on failed probe, same policies, no navigation.

    Direct stderr retains RAW_CHECK messages that k6 discards. It cannot grant
    a successful trial verdict, irrespective of whether this launch succeeds.
    """
    output = new_output(str(root / "native-diagnostic"))
    child = Child()
    result = {"qualification": "DIAGNOSTIC_ONLY_NOT_A_PROBE_PASS"}
    try:
        child.execute(native_arguments(pins["chromium"]["path"], output), output=output,
                      env=child_environment(output), timeout=8, browser=True)
    except Rejected as error:
        result["supervisor"] = error.code
    finally:
        result.update(native_exit=child.code, cleanup=child.cleanup,
                      stderr=browser_profile.native_diagnostic(child.stderr),
                      logfile=browser_profile.private_log_diagnostic(output / "private/chromium.log"))
    return result


def in_container(directory):
    hosted()
    require(Path("/.dockerenv").is_file(), "CONTAINER_TRIAL_REQUIRED")
    os.umask(0o077)
    root = Path(directory)
    private_path(root, directory=True)
    (root / "public").mkdir(mode=0o700)
    output = new_output(str(root / "case"))
    pins = browser_profile.tools(Path("/opt/browser"))
    env = child_environment(output)
    env.update(K6_BROWSER_EXECUTABLE_PATH=pins["chromium"]["path"],
               K6_EXPECTED_BROWSER_VERSION=pins["chromium"]["version"],
               K6_SUMMARY_PATH=str(output / "private/progress.json"),
               K6_BROWSER_ARGS="enable-logging,log-level=2,log-file=" + str(output / "private/chromium.log"))
    child = Child()
    report = {"qualification": "NOT_E2E_SANDBOX_TRIAL", "verdict": "SANDBOX_NOT_OBSERVED"}
    try:
        policy = process_security(Path("/proc/self/attr/current").read_text(), Path("/proc/self/status").read_text())
        report["process_security"] = policy
        require(policy["apparmor"] == "DOCKER_DEFAULT" and policy["seccomp_mode"] == 2
                and policy["no_new_privileges"] == 1 and type(policy["filters"]) is int and policy["filters"] >= 1,
                "TRIAL_PROCESS_POLICY_UNPROVEN")
        code = child.execute([pins["k6"]["path"], "run", "--quiet", "--no-usage-report", "--address", "127.0.0.1:0",
                              "--config", str(output / "private/empty.json"), str(HERE / "browser-sandbox-probe.js")],
                             output=output, env=env, timeout=25, browser=True)
        require(code == 0 and child.sandbox is True and child.renderer_observed is True and child.cleanup is True,
                "TRIAL_SANDBOX_NOT_PROVEN")
        report["verdict"] = "SANDBOX_WITNESS_OBSERVED"
    except Rejected as error:
        report["diagnostic"] = error.code
        report["browser_diagnostic"] = getattr(error, "browser_diagnostic", None)
    except Refused as error:
        report["diagnostic"] = str(error)
    finally:
        report.update(native_exit=child.code, sandbox=child.sandbox, renderer_observed=child.renderer_observed,
                      child_cleanup=child.cleanup, chromium=browser_profile.private_log_diagnostic(output / "private/chromium.log"),
                      witnesses=child.sandbox_proof, k6_version=pins["k6"]["version"], chromium_version=pins["chromium"]["version"])
        report["probe_progress"] = probe_progress(output / "private/progress.json")
        if report["verdict"] != "SANDBOX_WITNESS_OBSERVED" and child.cleanup:
            try:
                report["native_diagnostic"] = diagnose_native(root, pins)
            except Exception:
                report["native_diagnostic"] = {"status": "UNAVAILABLE"}
            if report["native_diagnostic"].get("cleanup") is not True:
                report["child_cleanup"] = False
        write(root / "public/result.json", report)
    return 0 if report["verdict"] == "SANDBOX_WITNESS_OBSERVED" else 1


def prepare_context(root, browser_directory):
    browser_directory = Path(browser_directory).resolve()
    pins = browser_profile.tools(browser_directory)
    context = root / "private/context"
    context.mkdir(mode=0o700)
    (context / "browser").mkdir()
    for name in ("k6.tar.gz", "chromium.zip", *(Path(pins[role]["path"]).relative_to(browser_directory).parts[0]
                                               for role in ("k6", "chromium"))):
        source, target = browser_directory / name, context / "browser" / name
        if source.is_dir():
            shutil.copytree(source, target)
        else:
            shutil.copyfile(source, target)
    for name in FILES:
        target = context / "ops/tests/k6" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(HERE.parent / name, target)
    shutil.copyfile(HERE / "Dockerfile.browser-trial", context / "Dockerfile")
    shutil.copyfile(HERE / "Dockerfile.browser-trial.dockerignore", context / ".dockerignore")
    return context


def clean(directory):
    hosted()
    root = Path(directory).resolve()
    path = root / "private/state.json"
    if not path.exists():
        print("No Docker trial ownership acquired")
        return 0
    private_path(root, directory=True)
    state = strict_json(path.read_bytes())
    require(state["run_id"] == os.environ["GITHUB_RUN_ID"] and state["attempt"] == os.environ["GITHUB_RUN_ATTEMPT"]
            and state["host"] == socket.gethostname() and state["boot"] == Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "TRIAL_CLEANUP_OWNER")
    result = {"status": "INCOMPLETE", "owned_running": None, "images": "retained_on_disposable_runner"}
    try:
        identity = state.get("container_id")
        records_path = root / "private/docker-resources.jsonl"
        records = [strict_json(line) for line in records_path.read_bytes().splitlines()] if records_path.exists() else []
        for registry in sorted((ROOT / ".tmp").glob("docker-resources-*.log")):
            require(registry.stat().st_size <= 4 * 1024 ** 2 and not registry.is_symlink(), "TRIAL_REGISTRY_LIMIT")
            records += [strict_json(line) for line in registry.read_bytes().splitlines() if line.strip()]
        if identity:
            validate_receipt(records, state, identity)
            require(identity not in state["before_containers"], "TRIAL_FOREIGN_CONTAINER")
            if identity in container_ids():
                value = strict_json(docker(["inspect", "--format", "{{json .}}", identity]).stdout)
                labels = value["Config"]["Labels"]
                require(value["Id"] == identity and value["Image"] == state["image_id"]
                        and labels.get("aiforall.dev/browser-trial-run") == state["run_id"]
                        and labels.get("aiforall.dev/browser-trial-attempt") == state["attempt"], "TRIAL_CONTAINER_IDENTITY")
                if value["State"]["Running"]:
                    docker(["stop", "--time", "20", identity])
                value = strict_json(docker(["inspect", "--format", "{{json .State}}", identity]).stdout)
                require(value["Running"] is False, "TRIAL_CONTAINER_STOP_UNPROVEN")
                record(root, state, "stopped", "container", identity, "no")
                record(root, state, "reclassified", "container", identity, "yes")
                docker(["rm", identity])
                record(root, state, "removed", "container", identity, "yes")
        candidates = docker(["container", "ls", "--all", "--no-trunc", "--format", "{{.ID}}", "--filter",
                            "label=aiforall.dev/browser-trial-run=" + state["run_id"], "--filter",
                            "label=aiforall.dev/browser-trial-attempt=" + state["attempt"]]).stdout.strip()
        require(not candidates and (not identity or identity not in container_ids()), "TRIAL_CLEANUP_INCOMPLETE")
        result.update(status="PASS", owned_running=0, container_id=identity)
    finally:
        write(root / "public/cleanup.json", result)
        records = root / "private/docker-resources.jsonl"
        if records.exists():
            shutil.copyfile(records, root / "public/docker-resources.jsonl")
    return 0


def host(directory, browser_directory, policy="baseline"):
    hosted()
    require(policy in ("baseline", "chromium"), "TRIAL_POLICY_MODE")
    os.umask(0o077)
    root = new_output(str(Path(directory).resolve()))
    (root / "public").mkdir(mode=0o700)
    (root / "private/container-data").mkdir(mode=0o700)
    state = {"run_id": os.environ["GITHUB_RUN_ID"], "attempt": os.environ["GITHUB_RUN_ATTEMPT"],
             "host": socket.gethostname(), "boot": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
             "uid": os.getuid(), "gid": os.getgid(), "before_containers": sorted(container_ids()), "policy": policy}
    write(root / "private/state.json", state)
    report = {"qualification": "NOT_E2E_SANDBOX_TRIAL", "harness_sha": os.environ["GITHUB_SHA"], "verdict": "INCONCLUSIVE"}
    try:
        security = strict_json(docker(["info", "--format", "{{json .SecurityOptions}}"]).stdout)
        require("name=apparmor" in security and "name=seccomp,profile=builtin" in security, "TRIAL_DEFAULT_DAEMON_SECURITY_UNPROVEN")
        report["daemon_security"] = {"apparmor": True, "seccomp_builtin": True}
        version = docker(["version", "--format", "{{.Server.Version}}"]).stdout.decode().strip()
        report["docker_version"] = version if re.fullmatch(r"[A-Za-z0-9.+-]{1,64}", version) else "WITHHELD"
        expected_seccomp = None
        if policy == "chromium":
            path, expected_seccomp, facts = acquire_seccomp(root)
            state.update(seccomp_path=str(path), seccomp_sha256=facts["derived_sha256"])
            write(root / "private/state.json", state)
            report["seccomp_profile"] = facts
        context = prepare_context(root, browser_directory)
        base = strict_json((HERE / "images.lock.json").read_bytes())["runtime"]
        require(re.fullmatch(r"ubuntu:24\.04@sha256:[a-f0-9]{64}", base), "TRIAL_RUNTIME_PIN")
        before_images = image_ids()
        iid = root / "private/image.iid"
        try:
            docker(["build", "--platform", "linux/amd64", "--iidfile", str(iid), "--build-arg", "RUNTIME_IMAGE=" + base,
                    "--file", str(context / "Dockerfile"), str(context)], timeout=900)
        finally:
            for identity in sorted(image_ids() - before_images):
                record(root, state, "created", "image", identity, "no")
        state["image_id"] = iid.read_text().strip()
        require(re.fullmatch(r"sha256:[a-f0-9]{64}", state["image_id"]), "TRIAL_IMAGE_ID")
        identity = docker(create_arguments(state, root)).stdout.decode().strip()
        require(re.fullmatch(r"[a-f0-9]{64}", identity), "TRIAL_CREATE_ID")
        record(root, state, "created", "container", identity, "no")
        state["container_id"] = identity
        write(root / "private/state.json", state)
        value = strict_json(docker(["inspect", "--format", "{{json .}}", identity]).stdout)
        # Created metadata may not yet expose the applied AppArmor profile.
        # Validate requests now; require the actual profile from inside the
        # started container before observing Chromium or accepting any result.
        report.update(policy=container_policy(value, state["uid"], state["gid"], applied=False, expected_seccomp=expected_seccomp),
                      image_id=state["image_id"], base=base)
        result = docker(["start", "--attach", identity], timeout=80, any_exit=True)
        source = root / "private/container-data/public/result.json"
        require(source.is_file() and not source.is_symlink() and source.stat().st_size <= 65536, "TRIAL_RESULT_MISSING")
        observed = strict_json(source.read_bytes())
        require(observed.get("qualification") == "NOT_E2E_SANDBOX_TRIAL", "TRIAL_RESULT_SCHEMA")
        report["observation"] = observed
        report["verdict"] = "SANDBOX_NOT_OBSERVED"
        applied = observed.get("process_security", {})
        require(applied.get("apparmor") == "DOCKER_DEFAULT" and applied.get("seccomp_mode") == 2
                and applied.get("no_new_privileges") == 1, "TRIAL_PROCESS_POLICY_UNPROVEN")
        report["policy"]["apparmor"] = "docker-default (in-process verified)"
        if (result.returncode == 0 and observed.get("verdict") == "SANDBOX_WITNESS_OBSERVED"
                and observed.get("sandbox") is True and observed.get("renderer_observed") is True
                and observed.get("child_cleanup") is True):
            report["verdict"] = "SANDBOX_WITNESS_OBSERVED"
    except Refused as error:
        report["diagnostic"] = str(error)
        report["verdict"] = "INCONCLUSIVE"
    except Exception:
        report["diagnostic"] = "TRIAL_UNCLASSIFIED"
        report["verdict"] = "INCONCLUSIVE"
    finally:
        try:
            clean(root)
        except Exception:
            report["verdict"] = "INCONCLUSIVE"
            report["cleanup"] = "INCOMPLETE"
        write(root / "public/result.json", report)
    return 0 if report["verdict"] == "SANDBOX_WITNESS_OBSERVED" else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("host", "run", "clean"))
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--browser-directory", type=Path)
    parser.add_argument("--policy", choices=("baseline", "chromium"), default="baseline")
    args = parser.parse_args()
    try:
        if args.action == "clean":
            return clean(args.directory)
        return in_container(args.directory) if args.action == "run" else host(args.directory, args.browser_directory, args.policy)
    except Exception:
        print("Docker sandbox trial incomplete; no E2E qualification or fallback policy")
        return 1


if __name__ == "__main__":
    sys.exit(main())
