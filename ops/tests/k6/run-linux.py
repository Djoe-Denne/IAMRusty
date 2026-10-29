#!/usr/bin/env python3
"""Mission1 launcher. The sole runtime/CI entry point; never provisions tools."""
import argparse
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import platform
import re
import subprocess
import sys
import time
import uuid

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
sys.path.insert(0, str(HERE / "lib"))
from safety import (ARTIFACT_LIMIT, Child, Rejected, child_environment, confidential,
                    digest, new_output, private_path, private_read, strict_json)

CHECKS = {"HTTP": ("iam-status", "iam-body"), "Chromium": ("browser-evaluation", "browser-blank")}
CASES = {"HTTP": "iam-health", "Chromium": "browser-prerequisite"}
SCENARIOS = {phase: HERE / "scenarios" / "smoke" / (case + ".js") for phase, case in CASES.items()}


def config_schema(data):
    fields = {"version", "profile", "cluster", "context", "task_id", "target_id", "origin", "lease", "observations"}
    if (not isinstance(data, dict) or set(data) != fields or type(data["version"]) is not int
            or data["version"] != 1 or data["profile"] != "local-full"
            or data["cluster"] != "aiforall-local-full" or data["context"] != "kind-aiforall-local-full"
            or data["origin"] != "http://127.0.0.1:18080"):
        raise Rejected("INPUT_INVALID")
    for key in ("task_id", "target_id"):
        if not isinstance(data[key], str) or not re.fullmatch(r"[a-f0-9]{32}", data[key]):
            raise Rejected("INPUT_INVALID")
    for key in ("lease", "observations"):
        if not isinstance(data[key], str) or not PurePosixPath(data[key]).is_absolute():
            raise Rejected("INPUT_INVALID")
    return data


def configuration(path):
    data = config_schema(strict_json(private_read(path)))
    # Offline only: no import/call of Docker, kubectl or the live target authority.
    lease = strict_json(private_read(data["lease"]))
    private_read(data["observations"])
    if (not isinstance(lease, dict) or lease.get("version") != 2
            or any(lease.get(k) != data[k] for k in ("task_id", "cluster", "context"))):
        raise Rejected("INPUT_INVALID")
    return data


def live_target(config):
    """Preserve the existing authority; never manufacture/adopt a POSIX lease.

    The contract explicitly leaves live origin binding unqualified. A successful
    cluster check is NOT an authorization to send HTTP to a declared loopback.
    No flag or campaign field can lift this gate. Integration needs the parent's
    actual existing guarded observation path (contract lines 103-125).
    """
    try:
        spec = importlib.util.spec_from_file_location("k6_target_authority", REPO / "ops/deploy/local_full_target.py")
        authority = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = authority
        spec.loader.exec_module(authority)
        anchor = authority.LocalClusterTargetAnchor(config["lease"], expected_cluster="aiforall-local-full")
        lease = anchor.check()  # real check; no injected runner, root or privacy
        if lease["task_id"] != config["task_id"]:
            raise Rejected("TARGET_UNAVAILABLE", True)
    except Rejected:
        raise
    except Exception:
        raise Rejected("TARGET_UNAVAILABLE", True) from None
    raise Rejected("TARGET_BINDING_UNQUALIFIED", True)


def verify_summary(data, run_id, phase):
    fields = {"version", "run_id", "phase", "cases", "expected", "started", "completed", "iterations", "requests", "checks"}
    if (not isinstance(data, dict) or set(data) != fields or type(data["version"]) is not int
            or data["version"] != 1 or data["run_id"] != run_id or data["phase"] != phase
            or data["cases"] != [CASES[phase]]):
        raise Rejected("SUMMARY_INVALID")
    for key in ("expected", "started", "completed", "iterations", "requests"):
        expected = 0 if key == "requests" and phase == "Chromium" else 1
        if type(data[key]) is not int or data[key] != expected:
            raise Rejected("SUMMARY_INVALID")
    if not isinstance(data["checks"], dict) or set(data["checks"]) != set(CHECKS[phase]):
        raise Rejected("SUMMARY_INVALID")
    for check in data["checks"].values():
        if (not isinstance(check, dict) or set(check) != {"passes", "fails"}
                or type(check["passes"]) is not int or check["passes"] != 1
                or type(check["fails"]) is not int or check["fails"] != 0):
            raise Rejected("SUMMARY_INVALID")


def verify_false_check(data, run_id, exit_code):
    # A broken endpoint must not masquerade as the deliberate failing check.
    if (exit_code == 0 or not isinstance(data, dict) or type(data.get("negative_baseline")) is not int
            or data["negative_baseline"] != 1 or data.get("checks", {}).get("iam-body") != {"passes": 0, "fails": 1}
            or any(type(value) is not int for value in data["checks"]["iam-body"].values())):
        raise Rejected("SELFTEST_FAILED")
    normalized = {key: value for key, value in data.items() if key != "negative_baseline"}
    normalized["checks"] = {**data["checks"], "iam-body": {"passes": 1, "fails": 0}}
    verify_summary(normalized, run_id, "HTTP")


def linux_platform():
    if sys.platform != "linux" or platform.machine() not in {"x86_64", "amd64"} or os.getuid() == 0:
        raise Rejected("PLATFORM_UNQUALIFIED", True)
    try:
        release = dict(line.split("=", 1) for line in Path("/etc/os-release").read_text().splitlines() if "=" in line)
        if release.get("ID", "").strip('"') != "ubuntu" or release.get("VERSION_ID", "").strip('"') != "24.04":
            raise Rejected("PLATFORM_UNQUALIFIED", True)
    except OSError:
        raise Rejected("PLATFORM_UNQUALIFIED", True) from None


def pinned_tools(browser=False):
    lock = strict_json((HERE / "config/tools.lock.json").read_bytes())
    value = os.environ.get("K6_TOOLS_DIR")
    if not value:
        raise Rejected("TOOL_UNPINNED", True)
    root = private_path(value, directory=True)
    tools = {}
    for name in ("k6", "chromium") if browser else ("k6",):
        entry = lock[name]
        archive = root / ("k6.tar.gz" if name == "k6" else "chromium.zip")
        binary = root / entry["executable"]
        # Verify archived distribution AND actual ELF, before any --version.
        for path in (archive, binary):
            if path != path.resolve(strict=True) or not path.is_file():
                raise Rejected("TOOL_UNPINNED", True)
            private_path(path)
        with binary.open("rb") as stream:
            elf = stream.read(4)
        if (digest(archive) != entry["archive_sha256"] or digest(binary) != entry["executable_sha256"] or elf != b"\x7fELF"):
            raise Rejected("TOOL_UNPINNED", True)
        tools[name] = binary
    if browser and os.environ.get("K6_BROWSER_EXECUTABLE_PATH") != str(tools["chromium"]):
        raise Rejected("TOOL_UNPINNED", True)
    return tools, lock


def version_checks(tools, lock, output):
    versions = {}
    for name, binary in tools.items():
        child = Child()
        args = [str(binary), "version" if name == "k6" else "--version"]
        if child.execute(args, output=output, env=child_environment(output), timeout=10) != 0:
            raise Rejected("TOOL_VERSION")
        text = child.stdout.decode("utf-8").strip()
        version = lock[name]["version"]
        pattern = r"k6 v" + re.escape(version) + r"(?:\s|$)" if name == "k6" else r"Google Chrome for Testing " + re.escape(version) + r"$"
        if not re.match(pattern, text):
            raise Rejected("TOOL_VERSION")
        versions[name] = version
    return versions


def k6_argv(binary, output, verb):
    return [str(binary), "--quiet", "--no-color", "--address", "", "--log-format", "raw",
            "--log-output", "stderr", "--config", str(output / "private/empty.json"),
            verb, "--include-system-env-vars=false", "--new-machine-readable-summary=false"]


def scenario_run(phase, config, output, tools, *, negative=None):
    # Internal arguments are only supplied by selftest; no CLI/env override.
    run_id = uuid.uuid4().hex
    summary = output / "private/summary.json"
    if summary.exists():
        raise Rejected("SUMMARY_INVALID")
    if phase == "HTTP":
        live_target(config)
    child = Child()
    argv = k6_argv(tools["k6"], output, "run")
    for key, value in {"K6_RUN_ID": run_id, "K6_SUMMARY_PATH": str(summary),
                       **({"K6_ORIGIN": config["origin"]} if phase == "HTTP" else {}),
                       **({"K6_INTERNAL_NEGATIVE": negative} if negative else {})}.items():
        argv += ["--env", key + "=" + value]
    argv.append(str(SCENARIOS[phase]))
    env = child_environment(output)
    if phase == "Chromium":
        env["K6_BROWSER_EXECUTABLE_PATH"] = str(tools["chromium"])
    failure = None
    try:
        code = child.execute(argv, output=output, env=env, timeout=30, browser=phase == "Chromium",
                             interrupt_after=0.2 if negative == "interrupt" else None)
        if code != 0 and negative not in {"false-check", "absent-case"}:
            raise Rejected("TOOL_FAILED")
        # k6 creates this in our fresh 0700 directory; seal only this owned file.
        if summary.is_symlink() or not summary.is_file() or summary.stat().st_size > ARTIFACT_LIMIT:
            raise Rejected("SUMMARY_INVALID")
        summary.chmod(0o400)
        raw = private_read(summary)
        confidential(raw)
        parsed = strict_json(raw)
        if negative == "absent-case":
            if (parsed.get("run_id") != run_id or parsed.get("expected") != 1
                    or parsed.get("cases") != ["iam-health"] or parsed.get("iterations") != 1
                    or parsed.get("started") not in {None, 0} or parsed.get("completed") not in {None, 0}
                    or parsed.get("requests") not in {None, 0} or parsed.get("checks") != {}):
                raise Rejected("SELFTEST_FAILED")
            raise Rejected("SUMMARY_INVALID")
        if negative == "false-check":
            verify_false_check(parsed, run_id, code)
            raise Rejected("TOOL_FAILED")
        verify_summary(parsed, run_id, phase)
    except BaseException as error:
        failure = error
    finally:
        if phase == "HTTP":
            # Required even for native failure or interruption; never cached.
            try:
                live_target(config)
            except Rejected as error:
                error.cleanup = "PASS" if child.cleanup else "FAIL"
                raise
    if failure:
        if isinstance(failure, Rejected):
            failure.cleanup = "PASS" if child.cleanup else "FAIL"
        raise failure
    return {"run_id": run_id, "cleanup": "PASS", "sandbox": "PASS" if phase == "Chromium" else "NOT_APPLICABLE",
            **({"sandbox_proof": child.sandbox_proof} if phase == "Chromium" else {})}


def publish(output, result, provenance=None):
    values = {"result.json": result}
    if provenance is not None:
        values["provenance.json"] = provenance
    encoded = {}
    # Validate EVERY byte before writing ANY publishable artifact.
    for name, value in values.items():
        raw = json.dumps(value, sort_keys=True, allow_nan=False, separators=(",", ":")).encode("utf-8")
        if len(raw) > ARTIFACT_LIMIT:
            raise Rejected("OUTPUT_LIMIT")
        confidential(raw)
        encoded[name] = raw
    if sum(map(len, encoded.values())) > 256 * 1024:
        raise Rejected("OUTPUT_LIMIT")
    for name, raw in encoded.items():
        path = output / name
        with path.open("xb") as stream:
            stream.write(raw)
        path.chmod(0o400)


def provenance(tools, lock, versions, config_path=None):
    # Only stable labels and hashes. No URLs, full environment, cmdlines or paths.
    sources = [REPO / "docs/contracts/k6-functional-linux.md", HERE / "run-linux.py",
               HERE / "lib/safety.py", HERE / "lib/summary.js", *SCENARIOS.values(), HERE / "config/tools.lock.json"]
    head = subprocess.run(["/usr/bin/git", "-C", str(REPO), "rev-parse", "HEAD"],
                          capture_output=True, timeout=5, check=True,
                          env={"PATH": "/usr/bin:/bin", "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null"}).stdout.strip().decode("ascii")
    if not re.fullmatch(r"[a-f0-9]{40}", head):
        raise Rejected("INPUT_INVALID")
    observed = {}
    for name, value in {"kernel": platform.release(), "architecture": platform.machine(),
                        "runner_image": os.environ.get("ImageOS", "not-provided"),
                        "runner_image_version": os.environ.get("ImageVersion", "not-provided")}.items():
        observed[name] = value if re.fullmatch(r"[A-Za-z0-9._+-]{1,160}", value) else "not-projectable"
    return {"version": 1, "head": head, "platform": "ubuntu-24.04-linux-amd64", "observed": observed, "versions": versions,
            "tools": {name: digest(path) for name, path in tools.items()},
            "sources": {str(path.relative_to(REPO)): digest(path) for path in sources},
            "config_sha256": digest(Path(config_path)) if config_path else None,
            "chromium_scope": "PREREQUISITE_ONLY", "http_scope": "INCONCLUSIVE"}


def negative_main(name, config_path, output_path):
    """Private selftest driver, deliberately NOT a run CLI/environment override."""
    output = None
    try:
        linux_platform()
        if name not in {"false-check", "absent-case", "missing-summary", "interrupt", "missing-input", "refused-target"}:
            raise Rejected("INPUT_INVALID")
        config = configuration(config_path)
        output = new_output(output_path)
        live_target(config)
        tools, lock = pinned_tools()
        version_checks(tools, lock, output)
        scenario_run("HTTP", config, output, tools, negative=name)
        return 0
    except Rejected as error:
        # A closed result from the actual negative launcher, with nonzero exit.
        print(json.dumps({"version": 1, "diagnostic": error.code, "non_success": True, "cleanup": error.cleanup}))
        return 2 if error.inconclusive else 1
    except BaseException:
        print('{"version":1,"diagnostic":"INTERNAL_ERROR","non_success":true}')
        return 1


def selftest(config, output, tools, config_path):
    # Fails closed before any child if no live binding. Never substitute a mock.
    live_target(config)
    expected = {"false-check": "TOOL_FAILED", "absent-case": "SUMMARY_INVALID",
                "missing-summary": "SUMMARY_INVALID", "interrupt": "INTERRUPTED"}
    outcomes = {}
    expected.update({"missing-input": "INPUT_INVALID", "refused-target": "INPUT_INVALID"})
    # A fixed child driver calls the same launcher; no shell, mock server or
    # user-selectable script. Each negative has a real observable nonzero exit.
    driver = ("import importlib.util,sys;from pathlib import Path;"
              "p=Path(sys.argv[1]);s=importlib.util.spec_from_file_location('negative_runner',p);"
              "m=importlib.util.module_from_spec(s);s.loader.exec_module(m);"
              "sys.exit(m.negative_main(sys.argv[2],sys.argv[3],sys.argv[4]))")
    for name, code in expected.items():
        child_config = str(config_path)
        if name in {"missing-input", "refused-target"}:
            value = {} if name == "missing-input" else {**config, "origin": "http://192.0.2.1"}
            path = output / "private" / (name + ".json")
            path.write_text(json.dumps(value), encoding="utf-8")
            path.chmod(0o400)
            child_config = str(path)
        supervisor_output = new_output(str(output / "private" / (name + "-supervisor")))
        env = child_environment(supervisor_output)
        # This Python driver is also a target guard parent, not the k6 child.
        # Keep the existing POSIX authority rooted at the same HOME as its parent.
        # Its own k6 subprocess still gets a fresh private HOME.
        env["HOME"] = str(Path.home())
        env["K6_TOOLS_DIR"] = os.environ["K6_TOOLS_DIR"]
        child = Child()
        exit_code = child.execute([sys.executable, "-B", "-c", driver, str(HERE / "run-linux.py"),
                                   name, child_config, str(output / "private" / (name + "-launch"))],
                                  output=supervisor_output, env=env, timeout=45)
        projection = strict_json(child.stdout)
        outcomes[name] = (exit_code != 0 and child.cleanup and projection == {
            "version": 1, "diagnostic": code, "non_success": True,
            "cleanup": "NOT_STARTED" if name in {"missing-input", "refused-target"} else "PASS"})
    if set(outcomes) != set(expected) or not all(outcomes.values()):
        raise Rejected("SELFTEST_FAILED")
    return outcomes


class ClosedParser(argparse.ArgumentParser):
    def error(self, message):
        # argparse ordinarily echoes arbitrary rejected input to stderr.
        raise Rejected("INPUT_INVALID")


def arguments(argv):
    parser = ClosedParser(description="Mission1 Linux prerequisites; HTTP gate remains fail-closed.")
    commands = parser.add_subparsers(dest="command", required=True, parser_class=ClosedParser)
    for name in ("validate", "run", "selftest", "browser", "tools"):
        command = commands.add_parser(name)
        if name in {"validate", "run", "selftest"}:
            command.add_argument("--config", required=True)
        if name != "validate":
            command.add_argument("--output", required=True)
    return parser.parse_args(argv)


def main(argv=None):
    output = None
    phase = None
    mode = None
    result = {"version": 1, "HTTP": "INCONCLUSIVE", "Chromium": "INCONCLUSIVE",
              "cleanup": "NOT_STARTED", "confidentiality": "PASS", "run_id": uuid.uuid4().hex,
              "started_unix_ms": int(time.time() * 1000), "cleanup_scope": "LAUNCHER_CHILDREN_ONLY",
              "parent_runtime_cleanup": "INCONCLUSIVE"}
    try:
        args = arguments(argv)
        mode = args.command
        phase = {"run": "HTTP", "browser": "Chromium"}.get(mode)
        linux_platform()
        config = configuration(args.config) if args.command in {"validate", "run", "selftest"} else None
        if args.command == "validate":
            result["diagnostic"] = "OFFLINE_ADMISSIBLE_NOT_AUTHORIZED"
        else:
            output = new_output(args.output)
            if args.command in {"run", "selftest"}:
                live_target(config)
            tools, lock = pinned_tools(browser=args.command in {"browser", "tools"})
            versions = version_checks(tools, lock, output)
            if args.command == "tools":
                for phase, path in SCENARIOS.items():
                    child = Child()
                    if child.execute(k6_argv(tools["k6"], output, "inspect") + [str(path)],
                                     output=output, env=child_environment(output), timeout=10) != 0:
                        raise Rejected("TOOL_FAILED")
                    # Inspect parses options but never runs an iteration or a target guard.
                    inspected = strict_json(child.stdout)
                    scenarios = inspected.get("scenarios", inspected.get("options", {}).get("scenarios", {}))
                    selected = scenarios.get(CASES[phase], {})
                    if (set(scenarios) != {CASES[phase]} or selected.get("executor") != "per-vu-iterations"
                            or selected.get("vus") != 1 or selected.get("iterations") != 1
                            or not selected.get("maxDuration")):
                        raise Rejected("INPUT_INVALID")
                result["tools"] = "PASS"
                result["diagnostic"] = "PREREQUISITES_ONLY_HTTP_NOT_RUN"
                result["cleanup"] = "PASS"
            elif args.command == "selftest":
                result["selftest"] = selftest(config, output, tools, args.config)
                result["cleanup"] = "PASS"
            else:
                phase = "Chromium" if args.command == "browser" else "HTTP"
                result.update(scenario_run(phase, config, output, tools))
                result[phase] = "PASS"
            result["finished_unix_ms"] = int(time.time() * 1000)
            publish(output, result, provenance(tools, lock, versions, getattr(args, "config", None)))
        raw = json.dumps(result, sort_keys=True).encode()
        confidential(raw)
        print(raw.decode())
        return 0
    except BaseException as error:
        if isinstance(error, SystemExit) and error.code == 0:
            return 0
        failure = error if isinstance(error, Rejected) else Rejected("INTERNAL_ERROR")
        result["diagnostic"] = failure.code
        result["cleanup"] = failure.cleanup
        if phase:
            result[phase] = "INCONCLUSIVE" if failure.inconclusive else "FAIL"
        if mode in {"selftest", "tools"}:
            result[mode] = "INCONCLUSIVE" if failure.inconclusive else "FAIL"
        result["confidentiality"] = "FAIL" if failure.code == "PRIVACY_FAILED" else "NOT_QUALIFIED"
        # No file publication on a failed confidentiality or incomplete cleanup.
        # Final one-line diagnostic is constructed from closed labels, not errors.
        raw = json.dumps(result, sort_keys=True).encode()
        confidential(raw)
        print(raw.decode())
        return 2 if failure.inconclusive else 1


if __name__ == "__main__":
    sys.exit(main())
