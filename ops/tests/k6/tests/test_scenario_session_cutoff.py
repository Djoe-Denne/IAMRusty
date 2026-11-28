import sys
import unittest
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ci"))
from binaries import Refused
from session_cutoff import Observation, cutoff, fresh_gateway_totals, replica_only

SECOND = 1_000_000_000


class CutoffTests(unittest.TestCase):
    def test_same_second_counter_line_before_the_measurement_is_rejected(self):
        threshold = datetime.fromisoformat("2026-11-28T09:32:14.550000Z")
        old = ("2026-11-28T09:32:14.540000321Z INFO session admission totals "
               "replica_decisions=11 authority_decisions=1 unavailable=0\n")
        recent = ("2026-11-28T09:32:14.560000123Z INFO session admission totals "
                  "replica_decisions=12 authority_decisions=1 unavailable=0\n")
        self.assertIsNone(fresh_gateway_totals(old, threshold))
        self.assertEqual(fresh_gateway_totals(old + recent, threshold), (12, 1, 0))

    def test_catchup_waits_for_a_recent_verified_gateway_proof(self):
        threshold = datetime.fromisoformat("2026-11-28T09:32:14.550000Z")
        line = ("2026-11-28T09:32:14.560000123Z INFO session admission totals "
                "replica_decisions=12 authority_decisions=1 unavailable=0 proof_age_ms=")
        self.assertIsNone(fresh_gateway_totals(line + "None", threshold, require_proof=True))
        self.assertIsNone(fresh_gateway_totals(line + "Some(2000)", threshold, require_proof=True))
        self.assertEqual(fresh_gateway_totals(line + "Some(999)", threshold, require_proof=True), (12, 1, 0))

    def test_an_earlier_admission_can_complete_after_the_bound(self):
        samples = [Observation(2 * SECOND, 3 * SECOND, 403),
                   Observation(4 * SECOND, 6 * SECOND, 200),
                   Observation(5 * SECOND, 6 * SECOND, 403)]
        self.assertEqual(cutoff(0, samples), 3000)

    def test_success_of_fresh_request_at_the_bound_fails(self):
        samples = [Observation(SECOND, 2 * SECOND, 403),
                   Observation(5 * SECOND, 6 * SECOND, 200)]
        with self.assertRaisesRegex(Refused, "E2E_RESET_LATE_ADMISSION"):
            cutoff(0, samples)

    def test_denial_completed_after_the_bound_does_not_prove_it(self):
        samples = [Observation(4 * SECOND, 5 * SECOND + 1, 503),
                   Observation(5 * SECOND, 6 * SECOND, 503)]
        with self.assertRaisesRegex(Refused, "E2E_RESET_DENIAL_BOUND_UNPROVEN"):
            cutoff(0, samples)

    def test_only_early_denials_do_not_prove_enforcement_at_the_bound(self):
        with self.assertRaisesRegex(Refused, "E2E_RESET_POST_BOUND_UNPROVEN"):
            cutoff(0, [Observation(SECOND, 2 * SECOND, 403)])

    def test_replica_and_independent_authority_counters_must_both_agree(self):
        replica_only((4, 1, 0), (5, 1, 0), 3, 3, 1)
        with self.assertRaisesRegex(Refused, "E2E_REPLICA_DECISION_SOURCE"):
            replica_only((4, 1, 0), (4, 2, 0), 3, 3, 1)
        with self.assertRaisesRegex(Refused, "E2E_REPLICA_AUTHORITY_CALL"):
            replica_only((4, 1, 0), (5, 1, 0), 3, 4, 1)


if __name__ == "__main__":
    unittest.main()
