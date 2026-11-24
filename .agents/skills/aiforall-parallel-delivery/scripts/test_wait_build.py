"""Isolated wake-up tests: no OpenCode sessions, API traffic, or runtimes."""

import subprocess
import unittest
from unittest.mock import Mock, patch

from wait_build import WakeError, native_api, wait_for_wake


SESSION = "ses_testBuild"
SINCE = 1000
SECONDS = 180
GET = f"/api/session/{SESSION}"
WAIT = f"/api/experimental/session/{SESSION}/wait"


def state(busy, idle, **kwargs):
    return {"id": SESSION, "agent": "build", "time": {"busy": busy, "idle": idle}, **kwargs}


class WakeTests(unittest.TestCase):
    def check_wake(self, responses, expected):
        api = Mock(side_effect=responses)
        delay = Mock()
        self.assertEqual(wait_for_wake(api, SESSION, SINCE, SECONDS, now=lambda: 0, delay=delay), expected)
        return api, delay

    def test_arm_after_fast_completion(self):
        api, delay = self.check_wake([state(1001, 1002)], "build-ended")
        api.assert_called_once_with("get", GET, 15)
        delay.assert_not_called()

    def test_native_wait_returns_after_current_turn(self):
        api, delay = self.check_wake([state(1001, 900), None, state(1001, 1100)], "build-ended")
        self.assertEqual([c.args for c in api.call_args_list], [
            ("get", GET, 15), ("post", WAIT, SECONDS), ("get", GET, 15),
        ])
        delay.assert_not_called()

    def test_old_idle_is_not_completion_for_a_queued_prompt(self):
        stale = state(700, 800)
        api, delay = self.check_wake([stale, None, stale], "deadline")
        self.assertEqual(api.call_count, 3)
        delay.assert_called_once_with(SECONDS)

    def test_timeout_is_a_wake_up_without_interrupting_worker(self):
        api, delay = self.check_wake(
            [state(1001, 900), subprocess.TimeoutExpired("wait", SECONDS)], "deadline",
        )
        self.assertEqual([c.args[:2] for c in api.call_args_list], [("get", GET), ("post", WAIT)])
        delay.assert_not_called()

    def test_wrong_owner_or_child_is_rejected_before_wait(self):
        for item in (state(1001, None, parentID="ses_parent"), {"id": "ses_other", "agent": "build"},
                     {"id": SESSION, "agent": "architecte"}):
            with self.subTest(item=item):
                api = Mock(return_value=item)
                with self.assertRaises(WakeError):
                    wait_for_wake(api, SESSION, SINCE, SECONDS, now=lambda: 0)
                api.assert_called_once_with("get", GET, 15)

    def test_wait_client_failure_is_not_reported_as_idle(self):
        api = Mock(side_effect=[state(1001, None), WakeError("unavailable")])
        with self.assertRaises(WakeError):
            wait_for_wake(api, SESSION, SINCE, SECONDS, now=lambda: 0)

    def test_session_check_timeout_is_a_monitor_error(self):
        api = Mock(side_effect=subprocess.TimeoutExpired("get", 15))
        with self.assertRaisesRegex(WakeError, "session check timed out"):
            wait_for_wake(api, SESSION, SINCE, SECONDS, now=lambda: 0)
        api.assert_called_once_with("get", GET, 15)

    @patch("wait_build.subprocess.run")
    def test_cli_uses_exact_authenticated_api_routes(self, run):
        run.side_effect = [
            subprocess.CompletedProcess([], 0, b'{"data":{"id":"ses_testBuild"}}'),
            subprocess.CompletedProcess([], 0, b""),
        ]
        self.assertEqual(native_api("opencode", "get", GET, 15), {"id": SESSION})
        self.assertIsNone(native_api("opencode", "post", WAIT, SECONDS))
        self.assertEqual([c.args[0] for c in run.call_args_list], [
            ["opencode", "api", "get", GET], ["opencode", "api", "post", WAIT],
        ])


if __name__ == "__main__":
    unittest.main()
