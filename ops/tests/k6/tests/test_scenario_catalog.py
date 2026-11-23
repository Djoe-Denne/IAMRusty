"""STORY-020 registration contract; synthetic files, no application target."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


K6_ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "scenario_catalog_evidence", K6_ROOT / "ci/evidence.py")
evidence = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(evidence)


class CatalogTests(unittest.TestCase):
    """E1/E3: explicit ordered selection and fail-closed registration."""

    def setUp(self):
        self.loader = getattr(evidence, "load_catalog", None)
        self.assertTrue(
            callable(self.loader),
            "STORY-020 requires ci/evidence.py::load_catalog(path)")
        canonical = K6_ROOT / "scenarios/catalog.json"
        self.assertTrue(canonical.is_file(), "one authored catalogue is required")
        self.document = json.loads(canonical.read_text(encoding="utf-8"))
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.parent = Path(self.directory.name)
        self.path = self.parent / "catalog.json"
        # Preserve the real relative layout, including specialized cases.
        for case in self.document["cases"]:
            relative = Path(case["script"])
            self.assertFalse(relative.is_absolute(), "canonical script must be relative")
            self.assertNotIn("..", relative.parts, "canonical script must not escape")
            destination = self.parent / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((canonical.parent / relative).read_bytes())
        self.write(self.document)

    def write(self, document):
        self.path.write_text(json.dumps(document), encoding="utf-8")

    def reject(self, document):
        self.write(document)
        with self.assertRaises(evidence.EvidenceError):
            self.loader(self.path)

    def test_catalogue_returns_records_in_declared_order(self):
        result = self.loader(self.path)
        expected = {case["id"]: case for case in self.document["cases"]}
        self.assertEqual(list(result), [case["id"] for case in self.document["cases"]])
        self.assertEqual(result, expected)

    def test_ordinary_registration_needs_no_second_python_list(self):
        ordinary = copy.deepcopy(next(
            case for case in self.document["cases"] if case["id"] == "iam-health"))
        ordinary.update(id="story020-ordinary", script="smoke/story020-ordinary.js")
        (self.parent / ordinary["script"]).write_text(
            "export default function () {}\n", encoding="utf-8")
        self.document["cases"].insert(1, ordinary)
        self.write(self.document)
        result = self.loader(self.path)
        self.assertEqual(list(result), [case["id"] for case in self.document["cases"]])
        self.assertEqual(result["story020-ordinary"], ordinary)

    def test_unknown_fields_and_missing_required_fields_are_refused(self):
        for location in ("document", "case"):
            fields = self.document if location == "document" else self.document["cases"][0]
            for field in (*fields, "undeclared_hook"):
                with self.subTest(location=location, field=field):
                    changed = copy.deepcopy(self.document)
                    record = changed if location == "document" else changed["cases"][0]
                    if field == "undeclared_hook":
                        record[field] = "never execute this"
                    else:
                        del record[field]
                    self.reject(changed)

    def test_duplicate_ids_and_json_members_are_refused(self):
        duplicate = copy.deepcopy(self.document)
        duplicate["cases"].append(copy.deepcopy(duplicate["cases"][0]))
        self.reject(duplicate)
        raw = json.dumps(self.document)
        case = self.document["cases"][0]
        check = next(iter(case["checks"]))
        for token in ('"version": 1', '"id": ' + json.dumps(case["id"]),
                      json.dumps(check) + ": " + json.dumps(case["checks"][check])):
            self.assertIn(token, raw, "duplicate fixture must alter an existing member")
            mutated = raw.replace(token, token + ", " + token, 1)
            with self.subTest(token=token):
                self.path.write_text(mutated, encoding="utf-8")
                with self.assertRaises(evidence.EvidenceError):
                    self.loader(self.path)

    def test_invalid_check_sets_and_numeric_types_are_refused(self):
        for field, values in (
            ("checks", ({}, [], {"unnamed": "unknown"}, {"": "functional"})),
            ("timeout_seconds", (True, 0, -1, 1.5)),
            ("requests", (True, -1, 1.5)),
            ("phase", ("shell", "")),
            ("lifecycle", ("arbitrary-command", "")),
            ("id", ("", "../escape", "case/escape")),
        ):
            for value in values:
                with self.subTest(field=field, value=value):
                    changed = copy.deepcopy(self.document)
                    changed["cases"][0][field] = value
                    self.reject(changed)
        for version in (True, 0, 2, 1.5):
            with self.subTest(version=version):
                changed = copy.deepcopy(self.document)
                changed["version"] = version
                self.reject(changed)

    def test_script_paths_cannot_escape_or_name_missing_files(self):
        outside = self.parent.parent / (self.parent.name + "-outside.js")
        outside.write_text("export default function () {}\n", encoding="utf-8")
        self.addCleanup(outside.unlink)
        for script in ("../" + outside.name, str(outside.resolve()),
                       "smoke/missing.js"):
            with self.subTest(script=script):
                changed = copy.deepcopy(self.document)
                changed["cases"][0]["script"] = script
                self.reject(changed)

    def test_malformed_nonfinite_and_nonobject_documents_are_refused(self):
        for raw in (b"", b"{", b"[]", b"{}trailing", b"\xff",
                    b'{"version": NaN, "cases": []}',
                    b'{"version": Infinity, "cases": []}'):
            with self.subTest(raw=raw):
                self.path.write_bytes(raw)
                with self.assertRaises(evidence.EvidenceError):
                    self.loader(self.path)

    def test_bounded_size_and_collection_cardinality(self):
        self.path.write_bytes(b" " * (evidence.SUMMARY_LIMIT + 1))
        with self.assertRaises(evidence.EvidenceError):
            self.loader(self.path)
        for cases in ([], {}, self.document["cases"] * 129):
            self.reject({**self.document, "cases": cases})
        for field, value in (("timeout_seconds", 601), ("requests", 65536),
                             ("checks", {"a" * 81: "functional"}), ("id", "a" * 65)):
            changed = copy.deepcopy(self.document)
            changed["cases"][0][field] = value
            self.reject(changed)

    def test_special_lifecycles_and_san_contiguity_cannot_be_redeclared(self):
        for field, value in (("lifecycle", "standard"), ("phase", "Chromium"),
                             ("script", "smoke/iam-health.js")):
            changed = copy.deepcopy(self.document)
            changed["cases"][1][field] = value
            self.reject(changed)
        for transform in ("missing-san", "missing-oauth", "reordered", "split"):
            changed = copy.deepcopy(self.document)
            cases = changed["cases"]
            if transform == "missing-san":
                del cases[2]
            elif transform == "missing-oauth":
                cases.remove(next(record for record in cases if record["id"] == "auth-oauth-browser"))
            elif transform == "reordered":
                cases[1], cases[2] = cases[2], cases[1]
            else:
                cases.insert(2, cases.pop(0))
            self.reject(changed)

    def test_script_symlinks_are_refused_even_when_the_target_is_in_scope(self):
        linked = self.parent / "smoke/linked.js"
        try:
            linked.symlink_to(self.parent / "smoke/iam-health.js")
        except OSError:
            self.skipTest("Windows account cannot create symlinks")
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["script"] = "smoke/linked.js"
        self.reject(changed)


if __name__ == "__main__":
    unittest.main()
