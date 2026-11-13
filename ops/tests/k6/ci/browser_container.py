"""Baseline non-root Docker sandbox trial; never an application E2E verdict."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import sys

from binaries import ROOT, Refused, require, sha, strict_json
import browser as browser_profile
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))
from safety import Child, Rejected, child_environment, new_output, private_path

HERE = Path(__file__).resolve().parent
OWNER = "k6-browser-container-trial"
FILES = ("ci/binaries.py", "ci/browser.py", "ci/browser_container.py", "ci/browser-sandbox-probe.js",
         "config/tools.lock.json", "lib/safety.py")


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


def container_policy(value, uid, gid):
    config, host = value["Config"], value["HostConfig"]
    require(config["User"] == f"{uid}:{gid}" and uid != 0, "TRIAL_REQUIRES_NONROOT")
    require(host["Privileged"] is False and host["ReadonlyRootfs"] is True
            and host["NetworkMode"] == "none" and host.get("PidMode", "") == ""
            and not host.get("CapAdd") and host["CapDrop"] == ["ALL"], "TRIAL_CONTAINER_POLICY")
    require(host["SecurityOpt"] in (["no-new-privileges"], ["no-new-privileges=true"]), "TRIAL_SECURITY_OVERRIDE")
    require(value.get("AppArmorProfile") == "docker-default", "TRIAL_DEFAULT_APPARMOR_UNPROVEN")
    return {"nonroot": True, "readonly_root": True, "network": "none", "capabilities": "ALL_DROPPED",
            "apparmor": "docker-default", "seccomp": "docker_default_requested",
            "no_new_privileges": True, "privileged": False}


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
    return [*args, state["image_id"]]


def validate_receipt(records, state, identity):
    rows = [row for row in records if row.get("kind") == "container" and row.get("id") == identity]
    require(any(row.get("event") == "created" for row in rows), "TRIAL_CREATION_RECEIPT_REQUIRED")
    require(all(row.get("owner") == OWNER and row.get("run_id") == state["run_id"]
                and row.get("run_attempt") == state["attempt"] and row.get("safe_to_delete") in ("yes", "no")
                for row in rows), "TRIAL_RECEIPT_CONFLICT")


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
               K6_BROWSER_ARGS="enable-logging,log-level=2,log-file=" + str(output / "private/chromium.log"))
    child = Child()
    report = {"qualification": "NOT_E2E_SANDBOX_TRIAL", "verdict": "SANDBOX_NOT_OBSERVED"}
    try:
        code = child.execute([pins["k6"]["path"], "run", "--quiet", "--no-usage-report", "--address", "127.0.0.1:0",
                              "--config", str(output / "private/empty.json"), str(HERE / "browser-sandbox-probe.js")],
                             output=output, env=env, timeout=25, browser=True)
        require(code == 0 and child.sandbox is True and child.renderer_observed is True and child.cleanup is True,
                "TRIAL_SANDBOX_NOT_PROVEN")
        report["verdict"] = "SANDBOX_WITNESS_OBSERVED"
    except Rejected as error:
        report["diagnostic"] = error.code
    except Refused as error:
        report["diagnostic"] = str(error)
    finally:
        report.update(native_exit=child.code, sandbox=child.sandbox, renderer_observed=child.renderer_observed,
                      child_cleanup=child.cleanup, chromium=browser_profile.private_log_diagnostic(output / "private/chromium.log"),
                      witnesses=child.sandbox_proof, k6_version=pins["k6"]["version"], chromium_version=pins["chromium"]["version"])
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


def host(directory, browser_directory):
    hosted()
    os.umask(0o077)
    root = new_output(str(Path(directory).resolve()))
    (root / "public").mkdir(mode=0o700)
    (root / "private/container-data").mkdir(mode=0o700)
    state = {"run_id": os.environ["GITHUB_RUN_ID"], "attempt": os.environ["GITHUB_RUN_ATTEMPT"],
             "host": socket.gethostname(), "boot": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
             "uid": os.getuid(), "gid": os.getgid(), "before_containers": sorted(container_ids())}
    write(root / "private/state.json", state)
    report = {"qualification": "NOT_E2E_SANDBOX_TRIAL", "harness_sha": os.environ["GITHUB_SHA"], "verdict": "INCONCLUSIVE"}
    try:
        security = strict_json(docker(["info", "--format", "{{json .SecurityOptions}}"]).stdout)
        require("name=apparmor" in security and "name=seccomp,profile=builtin" in security, "TRIAL_DEFAULT_DAEMON_SECURITY_UNPROVEN")
        report["daemon_security"] = {"apparmor": True, "seccomp_builtin": True}
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
        report.update(policy=container_policy(value, state["uid"], state["gid"]), image_id=state["image_id"], base=base)
        result = docker(["start", "--attach", identity], timeout=50, any_exit=True)
        source = root / "private/container-data/public/result.json"
        require(source.is_file() and not source.is_symlink() and source.stat().st_size <= 65536, "TRIAL_RESULT_MISSING")
        observed = strict_json(source.read_bytes())
        require(observed.get("qualification") == "NOT_E2E_SANDBOX_TRIAL", "TRIAL_RESULT_SCHEMA")
        report["observation"] = observed
        report["verdict"] = "SANDBOX_NOT_OBSERVED"
        if (result.returncode == 0 and observed.get("verdict") == "SANDBOX_WITNESS_OBSERVED"
                and observed.get("sandbox") is True and observed.get("renderer_observed") is True
                and observed.get("child_cleanup") is True):
            report["verdict"] = "SANDBOX_WITNESS_OBSERVED"
    except Refused as error:
        report["diagnostic"] = str(error)
    except Exception:
        report["diagnostic"] = "TRIAL_UNCLASSIFIED"
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
    args = parser.parse_args()
    try:
        if args.action == "clean":
            return clean(args.directory)
        return in_container(args.directory) if args.action == "run" else host(args.directory, args.browser_directory)
    except Exception:
        print("Docker sandbox trial incomplete; no E2E qualification or fallback policy")
        return 1


if __name__ == "__main__":
    sys.exit(main())
