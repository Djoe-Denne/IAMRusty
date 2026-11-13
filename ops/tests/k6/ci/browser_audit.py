"""Read-only Ubuntu sandbox diagnosis. Never qualifies E2E or changes policy."""
import argparse
import json
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import socket
import subprocess
import sys
import threading
import time

from binaries import Refused, require, strict_json
import browser as browser_profile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))
from safety import Child, Rejected, child_environment, new_output, private_path, same

SETTINGS = {
    "unprivileged_userns_clone": "/proc/sys/kernel/unprivileged_userns_clone",
    "apparmor_restrict_unprivileged_userns": "/proc/sys/kernel/apparmor_restrict_unprivileged_userns",
    "apparmor_restrict_unprivileged_unconfined": "/proc/sys/kernel/apparmor_restrict_unprivileged_unconfined",
    "max_user_namespaces": "/proc/sys/user/max_user_namespaces",
}


def hosted():
    require(sys.platform == "linux" and os.getuid() != 0 and os.environ.get("GITHUB_ACTIONS") == "true"
            and os.environ.get("GITHUB_JOB") == "diagnose" and os.environ.get("GITHUB_REF") == "refs/heads/master",
            "HOSTED_DIAGNOSTIC_ONLY")


def profile(value):
    value = value.strip()
    if value == "unconfined":
        return "UNCONFINED"
    if value in ("unprivileged_userns", "unprivileged_userns (enforce)"):
        return "UNPRIVILEGED_USERNS"
    return "OTHER"


def read_profile(path):
    try:
        return profile(Path(path).read_text(encoding="utf-8"))
    except (OSError, UnicodeError):
        return "UNREADABLE"


def settings():
    result = {}
    for name, path in SETTINGS.items():
        try:
            value = Path(path).read_text(encoding="ascii").strip()
            result[name] = int(value) if re.fullmatch(r"[0-9]{1,12}", value) else "WITHHELD"
        except (OSError, UnicodeError):
            result[name] = "UNAVAILABLE"
    result["parent_profile"] = read_profile("/proc/self/attr/current")
    release = platform.release()
    result["kernel"] = release if re.fullmatch(r"[A-Za-z0-9._+-]{1,100}", release) else "WITHHELD"
    for name in ("ImageOS", "ImageVersion"):
        value = os.environ.get(name, "")
        result[name] = value if re.fullmatch(r"[A-Za-z0-9._+-]{1,80}", value) else "UNAVAILABLE"
    result["apparmor_enabled"] = Path("/sys/module/apparmor/parameters/enabled").read_text().strip() == "Y" if Path("/sys/module/apparmor/parameters/enabled").exists() else None
    return result


def audit_denials(raw, identities, boot_id, start_us, end_us):
    """PID ownership, UID if present, boot and launch window; no raw projection."""
    owned = {item["pid"]: item for item in identities}
    found = []
    require(len(raw) <= 2 * 1024 ** 2, "AUDIT_OUTPUT_LIMIT")
    for line in raw.splitlines():
        if not line.strip():
            continue
        item = strict_json(line)
        if item.get("_BOOT_ID") != boot_id.replace("-", ""):
            continue
        stamp = item.get("__REALTIME_TIMESTAMP", "")
        if not isinstance(stamp, str) or not stamp.isdigit() or not start_us <= int(stamp) <= end_us:
            continue
        message = item.get("MESSAGE", "")
        if not isinstance(message, str):
            continue
        # Never use process name alone as ownership. PIDs come only from Child's
        # birth-time-bound identity receipts during this one Chrome launch.
        pid = re.search(r'\bpid=([0-9]+)\b', message)
        if not pid or int(pid[1]) not in owned or not re.search(r'\bcomm="chrome"', message):
            continue
        uid = re.search(r'\b(?:fsuid|uid)=([0-9]+)\b', message)
        if uid and int(uid[1]) != owned[int(pid[1])]["uid"]:
            continue
        if not re.search(r'\bapparmor="DENIED"', message):
            continue
        if re.search(r'\boperation="userns_create"', message):
            found.append("USERNS_CREATE_DENIED")
        elif (re.search(r'\boperation="capable"', message)
              and re.search(r'\bprofile="unprivileged_userns"', message)):
            found.append("USERNS_CAPABILITY_DENIED")
    return {"status": "CONFIRMED_APPARMOR_USERNS_DENIAL" if found else "INCONCLUSIVE",
            "categories": sorted(set(found)), "matched_events": len(found)}


def native_arguments(executable, output):
    return [str(executable), "--headless", "--no-startup-window", "--remote-debugging-port=0",
            "--remote-debugging-address=127.0.0.1", "--no-first-run", "--no-default-browser-check",
            "--disable-background-networking", "--disable-component-update", "--disable-sync", "--disable-extensions",
            "--user-data-dir=" + str(output / "private/chrome-profile"), "--enable-logging", "--log-level=2",
            "--log-file=" + str(output / "private/chromium.log")]


def save(path, value):
    path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")
    path.chmod(0o600)


def run(directory, browser_directory):
    hosted()
    os.umask(0o077)
    output = new_output(str(Path(directory).resolve()))
    (output / "public").mkdir(mode=0o700)
    pins = browser_profile.tools(browser_directory)
    boot = Path("/proc/sys/kernel/random/boot_id").read_text().strip()
    save(output / "private/owner.json", {"host": socket.gethostname(), "boot_id": boot,
        "run_id": os.environ["GITHUB_RUN_ID"], "attempt": os.environ["GITHUB_RUN_ATTEMPT"]})
    report = {"qualification": "NOT_E2E_DIAGNOSTIC_ONLY", "settings": settings(),
              "audit": {"status": "INCONCLUSIVE", "categories": [], "matched_events": 0}}
    child = Child()
    profiles = set()
    stop = threading.Event()

    def observe():
        while not stop.wait(0.01):
            for identity in list(child.owned.values()):
                try:
                    if same(identity):
                        value = read_profile("/proc/" + str(identity["pid"]) + "/attr/current")
                        if same(identity):
                            profiles.add(value)
                except Rejected:
                    profiles.add("UNREADABLE")

    observer = threading.Thread(target=observe, daemon=True)
    start_us = time.time_ns() // 1000
    observer.start()
    try:
        try:
            child.execute(native_arguments(pins["chromium"]["path"], output), output=output,
                          env=child_environment(output), timeout=8, browser=True)
        except Rejected as error:
            report["supervisor"] = error.code
        end_us = time.time_ns() // 1000
        report.update(chromium=browser_profile.private_log_diagnostic(output / "private/chromium.log"),
                      native_exit=child.code, renderer_observed=child.renderer_observed,
                      sandbox_witness=child.sandbox, cleanup=child.cleanup)
        identities = [{key: value[key] for key in ("pid", "uid", "start")} for value in child.owned.values()]
        report["received_process_count"] = len(identities)
        journal = shutil.which("journalctl")
        sudo = shutil.which("sudo")
        if journal and sudo:
            result = subprocess.run([sudo, "--non-interactive", journal, "--dmesg", "--boot=0", "--no-pager",
                "--output=json", "--grep=apparmor=", "--lines=2000", "--since=@" + str(start_us // 1000000),
                "--until=@" + str(end_us // 1000000 + 1)], capture_output=True, timeout=15,
                env={"PATH": "/usr/bin:/bin", "LANG": "C.UTF-8"})
            if result.returncode == 0:
                report["audit"] = audit_denials(result.stdout, identities, boot, start_us, end_us)
            else:
                report["audit"]["access"] = "UNAVAILABLE"
    except Exception:
        report["audit"]["access"] = "UNAVAILABLE_OR_UNPARSEABLE"
    finally:
        stop.set()
        observer.join(timeout=2)
        report["observed_profiles"] = sorted(profiles)
        report["cleanup"] = child.cleanup
        save(output / "public/diagnostic.json", report)
    print("Read-only Chromium diagnosis collected; never an E2E qualification")
    return 0 if child.cleanup else 1


def clean(directory):
    hosted()
    output = Path(directory).resolve()
    if not (output / "private/owner.json").exists():
        print("No diagnostic process ownership acquired")
        return 0
    private_path(output, directory=True)
    owner = strict_json((output / "private/owner.json").read_bytes())
    require(owner == {"host": socket.gethostname(), "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
                      "run_id": os.environ["GITHUB_RUN_ID"], "attempt": os.environ["GITHUB_RUN_ATTEMPT"]}, "DIAGNOSTIC_OWNER")
    journal = output / "private/process-identities.jsonl"
    identities = []
    if journal.exists():
        require(journal.stat().st_size <= 32768 and not journal.is_symlink(), "DIAGNOSTIC_RECEIPTS")
        identities = [strict_json(line) for line in journal.read_bytes().splitlines() if line.strip()]
    for identity in identities:
        require(set(identity) == {"pid", "uid", "start"} and all(type(value) is int for value in identity.values())
                and identity["pid"] > 1 and identity["pid"] != os.getpid() and identity["uid"] == os.getuid()
                and identity["start"] > 0, "DIAGNOSTIC_RECEIPTS")
        current = same(identity)
        if current and not current["zombie"]:
            os.kill(identity["pid"], signal.SIGTERM)
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        alive = [value for identity in identities if (value := same(identity)) and not value["zombie"]]
        if not alive:
            save(output / "public/cleanup.json", {"status": "PASS", "owned_running": 0, "received": len(identities)})
            return 0
        time.sleep(0.05)
    save(output / "public/cleanup.json", {"status": "FAIL", "owned_running": None})
    return 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("run", "clean"))
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--browser-directory", type=Path)
    args = parser.parse_args()
    try:
        return clean(args.directory) if args.action == "clean" else run(args.directory, args.browser_directory)
    except Exception:
        print("Read-only Chromium diagnosis incomplete; no policy changes made")
        return 1


if __name__ == "__main__":
    sys.exit(main())
