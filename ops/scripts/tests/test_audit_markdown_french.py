"""Offline regression tests; no model requests or runtime startup."""

import importlib.util
import io
import json
import pathlib
import unittest
from unittest.mock import patch


SCRIPT = pathlib.Path(__file__).resolve().parents[1] / "audit-markdown-french.py"
SPEC = importlib.util.spec_from_file_location("audit_markdown_french", SCRIPT)
audit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(audit)


class MarkdownFrenchAuditTests(unittest.TestCase):
    def test_supported_encodings_and_invalid_bytes(self):
        for encoding in ("utf-8", "utf-8-sig", "utf-16"):
            with self.subTest(encoding=encoding):
                self.assertEqual(audit.decode("français".encode(encoding)), "français")
        with self.assertRaises(UnicodeDecodeError):
            audit.decode(b"\x80")

    def test_empty_and_exact_chunk_boundary(self):
        self.assertEqual(list(audit.chunks("")), [])
        self.assertEqual(len(list(audit.chunks("x" * 12000))), 1)
        self.assertEqual(len(list(audit.chunks("x" * 12001))), 2)

    def test_full_coverage_overlap_and_line_numbers(self):
        text = "x" * 11999 + "\nBonjour le monde\n" + "z" * 13000
        parts = list(audit.chunks(text))
        reconstructed = parts[0]["text"] + "".join(p["text"][400:] for p in parts[1:])
        self.assertEqual(reconstructed, text)
        self.assertTrue(any("Bonjour le monde" in p["text"] for p in parts))
        self.assertEqual(parts[0]["first_line"], 1)
        self.assertEqual(parts[-1]["last_line"], 3)

    def decision(self, answers, cost=0):
        connection = {"url": "http://127.0.0.1:12345", "headers": {}}
        response = json.dumps({"data": {"answers": answers, "cost": cost}}).encode()
        with patch.object(audit.urllib.request, "urlopen", return_value=io.BytesIO(response)):
            return audit.classify(connection, [{"text": "Bonjour"}, {"text": "Hello"}])

    def test_answers_are_matched_by_name_not_order(self):
        self.assertEqual(self.decision([
            {"type": "predicate", "name": "sample_1", "probability": 0.02},
            {"type": "predicate", "name": "sample_0", "probability": 0.98},
        ]), [0.98, 0.02])

    def test_incomplete_duplicate_and_invalid_decisions_fail(self):
        valid = {"type": "predicate", "name": "sample_0", "probability": 0.98}
        for answers in (
            [valid], [valid, valid],
            [valid, {"type": "predicate", "name": "sample_1", "probability": True}],
            [valid, {"type": "predicate", "name": "sample_1", "probability": 1.5}],
        ):
            with self.subTest(answers=answers), self.assertRaises(ValueError):
                self.decision(answers)

    def test_unexpected_paid_result_fails(self):
        with self.assertRaises(ValueError):
            self.decision([
                {"type": "predicate", "name": "sample_0", "probability": 0.98},
                {"type": "predicate", "name": "sample_1", "probability": 0.02},
            ], cost=0.01)


if __name__ == "__main__":
    unittest.main()
