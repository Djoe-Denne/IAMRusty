"""Isolated lifecycle tests; no real Cargo, Docker, GitHub, or shared lock."""

from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import sys
import tempfile
import threading
import unittest
from unittest.mock import Mock

from cargo_slot import BUSY, UNCERTAIN, run_with_slot


class CargoSlotTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.marker = Path(tmp.name) / "shared.lock"
        self.command = ["qualification", "test"]

    def test_busy_owner_blocks_another_writer_until_child_returns(self):
        entered = threading.Event()
        release = threading.Event()

        def hold(command):
            self.assertEqual(command, self.command)
            entered.set()
            self.assertTrue(release.wait(5))
            return 0

        denied = Mock()
        with ThreadPoolExecutor(max_workers=1) as executor:
            owner = executor.submit(run_with_slot, self.marker, self.command, active=lambda: False, launch=hold)
            try:
                self.assertTrue(entered.wait(5))
                self.assertEqual(run_with_slot(self.marker, self.command, active=lambda: False, launch=denied), BUSY)
                self.assertTrue(self.marker.is_file())
                denied.assert_not_called()
            finally:
                release.set()
            self.assertEqual(owner.result(timeout=5), 0)
        self.assertFalse(self.marker.exists())
        self.assertEqual(run_with_slot(self.marker, self.command, active=lambda: False, launch=lambda _: 0), 0)

    def test_failure_exit_code_is_preserved_and_marker_released(self):
        command = [sys.executable, "-c", "raise SystemExit(101)"]
        self.assertEqual(run_with_slot(self.marker, command, active=lambda: False), 101)
        self.assertFalse(self.marker.exists())

    def test_unavailable_inventory_retains_marker(self):
        launch = Mock()
        active = Mock(side_effect=RuntimeError("inventory unavailable"))
        self.assertEqual(run_with_slot(self.marker, self.command, active=active, launch=launch), UNCERTAIN)
        launch.assert_not_called()
        self.assertTrue(self.marker.is_file())

    def test_external_compiler_blocks_admission(self):
        launch = Mock()
        self.assertEqual(run_with_slot(self.marker, self.command, active=lambda: True, launch=launch), BUSY)
        launch.assert_not_called()
        self.assertFalse(self.marker.exists())

    def test_incomplete_teardown_and_interruption_retain_marker(self):
        active = Mock(side_effect=(False, True))
        self.assertEqual(run_with_slot(self.marker, self.command, active=active, launch=lambda _: 0), UNCERTAIN)
        self.assertTrue(self.marker.is_file())
        self.marker.unlink()
        stopped = Mock(side_effect=KeyboardInterrupt)
        self.assertEqual(run_with_slot(self.marker, self.command, active=lambda: False, launch=stopped), UNCERTAIN)
        self.assertTrue(self.marker.is_file())

    def test_uncertain_or_changed_marker_is_not_removed(self):
        self.marker.write_text("prior owner")
        launch = Mock()
        self.assertEqual(run_with_slot(self.marker, self.command, active=lambda: False, launch=launch), BUSY)
        launch.assert_not_called()
        self.assertEqual(self.marker.read_text(), "prior owner")
        self.marker.unlink()

        def replace(_):
            self.marker.write_text("new owner")
            return 0

        self.assertEqual(run_with_slot(self.marker, self.command, active=lambda: False, launch=replace), UNCERTAIN)
        self.assertEqual(self.marker.read_text(), "new owner")


if __name__ == "__main__":
    unittest.main()
