"""Emit one bounded wake-up for a verified independent OpenCode Build."""

import argparse
import json
import re
import subprocess
import time


class WakeError(Exception):
    """The wake-up could not check its target."""


def native_api(cli, method, path, timeout):
    result = subprocess.run(
        [cli, "api", method, path], stdin=subprocess.DEVNULL,
        capture_output=True, timeout=timeout, check=False,
    )
    if result.returncode:
        raise WakeError("native OpenCode request failed")
    if method == "post":
        # The native wait endpoint returns 204 and does not deliver a verdict.
        return None
    try:
        return json.loads(result.stdout)["data"]
    except (ValueError, KeyError, TypeError):
        raise WakeError("unexpected session response") from None


def dispatch_ended(state, since_ms):
    """Ignore an idle stamp from before this dispatch or a newer running turn."""
    stamps = state.get("time")
    if not isinstance(stamps, dict):
        raise WakeError("missing session timestamps")
    busy, idle = stamps.get("busy"), stamps.get("idle")
    return type(busy) is int and type(idle) is int and since_ms <= busy <= idle


def wait_for_wake(api, session, since_ms, seconds, *, now=time.monotonic, delay=time.sleep):
    deadline = now() + seconds

    def remaining():
        return deadline - now()

    def read_state():
        timeout = remaining()
        if timeout <= 0:
            raise subprocess.TimeoutExpired("OpenCode wake", seconds)
        try:
            state = api("get", f"/api/session/{session}", min(timeout, 15))
        except subprocess.TimeoutExpired:
            raise WakeError("native session check timed out") from None
        if (not isinstance(state, dict) or state.get("id") != session
                or state.get("agent") != "build" or state.get("parentID") is not None):
            raise WakeError("target is not the recorded root Build")
        return state

    try:
        if dispatch_ended(read_state(), since_ms):
            return "build-ended"
        timeout = remaining()
        if timeout <= 0:
            return "deadline"
        api("post", f"/api/experimental/session/{session}/wait", timeout)
        if dispatch_ended(read_state(), since_ms):
            return "build-ended"
    except subprocess.TimeoutExpired:
        # A deadline ends the waiter, not the Build session.
        return "deadline"

    # Native wait can return before the accepted prompt starts. Do not spin
    # or announce completion from an earlier idle turn. Wake at the deadline.
    delay(max(0, remaining()))
    return "deadline"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", default="opencode", help="OpenCode CLI executable")
    parser.add_argument("--session", required=True)
    parser.add_argument("--since-ms", type=int, required=True,
                        help="UTC dispatch cutoff in epoch milliseconds")
    parser.add_argument("--seconds", type=int, default=180,
                        help="bounded wake interval in seconds")
    args = parser.parse_args()
    if not re.fullmatch(r"ses_[A-Za-z0-9]+", args.session):
        parser.error("expected a native session ID")
    if args.since_ms <= 0 or not 1 <= args.seconds <= 1800:
        parser.error("invalid dispatch cutoff or wake interval (1–1800 seconds)")
    reason = None
    try:
        event = wait_for_wake(
            lambda method, path, timeout: native_api(args.cli, method, path, timeout),
            args.session, args.since_ms, args.seconds,
        )
    except (WakeError, OSError) as err:
        event = "monitor-error"
        reason = str(err) if isinstance(err, WakeError) else "OpenCode CLI unavailable"
    print(json.dumps({
        "session": args.session, "since_ms": args.since_ms, "event": event, "reason": reason,
    }))
    return 1 if event == "monitor-error" else 0


if __name__ == "__main__":
    raise SystemExit(main())
