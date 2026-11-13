"""Independent cancellation cleanup: IDs from receipts, never labels as ownership."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ci"))
import binaries as b
import cleanup as c

ID, FOREIGN = "a" * 64, "b" * 64


def receipt(**changes):
    return dict(event="created", kind="container", id=ID, owner=c.OWNER,
                run_id="789", run_attempt="1", safe_to_delete="no", **changes)


class ReceiptTests(unittest.TestCase):
    def test_exact_creation_receipt_required(self):
        self.assertEqual(c.owned_receipts([receipt()], "789", "1"), {ID})
        self.assertEqual(c.owned_receipts([receipt()], "789", "2"), set())
        self.assertEqual(c.owned_receipts([{**receipt(), "event": "stopped"}], "789", "1"), set())

    def test_ask_and_foreign_identity_conflict_refused(self):
        for changed in (dict(safe_to_delete="ask"), dict(owner="another-mission"), dict(run_attempt="2")):
            with self.subTest(changed=changed), self.assertRaises(b.Refused):
                c.owned_receipts([receipt(), {**receipt(), **changed, "event": "reclassified"}], "789", "1")


class ClosureTests(unittest.TestCase):
    def close(self, running=True, present=None, candidates=None, wrong_labels=False, stop_works=True):
        self.calls = []
        state = {"running": running}

        def inspect(identity):
            self.calls.append(("inspect", identity))
            return dict(Id=identity, Config={"Labels": {
                "aiforall.dev/k6-packaging-run": "other" if wrong_labels else "789",
                "aiforall.dev/k6-packaging-attempt": "1"}}, State={"Running": state["running"]})

        def stop(identity):
            self.calls.append(("stop", identity))
            state["running"] = not stop_works

        return c.close_probes({ID}, {ID} if present is None else present,
                              {ID} if candidates is None else candidates, inspect, stop,
                              lambda identity: self.calls.append(("remove", identity)),
                              lambda event, identity, safe: self.calls.append((event, identity, safe)), "789", "1")

    def test_interrupted_active_probe_stops_before_id_scoped_removal(self):
        self.assertEqual(self.close(), [ID])
        self.assertLess(self.calls.index(("stop", ID)), self.calls.index(("remove", ID)))
        self.assertLess(self.calls.index(("reclassified", ID, "yes")), self.calls.index(("remove", ID)))
        self.assertTrue(all(call[1] == ID for call in self.calls))

    def test_stopped_probe_is_removed_without_stop(self):
        self.close(running=False)
        self.assertNotIn(("stop", ID), self.calls)
        self.assertIn(("remove", ID), self.calls)

    def test_already_removed_probe_is_idempotently_absent(self):
        self.assertEqual(self.close(present=set(), candidates=set()), [])
        self.assertEqual(self.calls, [])

    def test_label_only_unreceipted_probe_is_not_adopted_but_owned_probe_still_closed(self):
        with self.assertRaises(b.Refused):
            self.close(candidates={ID, FOREIGN}, present={ID, FOREIGN})
        self.assertIn(("remove", ID), self.calls)
        self.assertFalse(any(call[1] == FOREIGN for call in self.calls))

    def test_identity_conflict_does_not_stop_or_remove(self):
        with self.assertRaises(b.Refused):
            self.close(wrong_labels=True)
        self.assertNotIn(("stop", ID), self.calls)
        self.assertNotIn(("remove", ID), self.calls)

    def test_failed_stop_cannot_be_reclassified_or_removed(self):
        with self.assertRaises(b.Refused):
            self.close(stop_works=False)
        self.assertNotIn(("reclassified", ID, "yes"), self.calls)
        self.assertNotIn(("remove", ID), self.calls)


if __name__ == "__main__":
    unittest.main()
