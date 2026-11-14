"""Synthetic, offline tests for the standalone constraint planner."""

import contextlib
import copy
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "clippy-constraint-plan.py"
SPEC = importlib.util.spec_from_file_location("constraint_plan", SCRIPT)
planner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(planner)
FIXTURES = Path(__file__).parent / "fixtures" / "clippy_constraint_plan"


class Fixture:
    def __init__(self, root):
        self.root = root.resolve()
        self.inventory_path = self.root / "inventory.json"
        self.snapshot_path = self.root / "snapshot.json"
        self.evidence_path = self.root / "evidence.json"
        self.constraints_path = self.root / "constraints.json"
        self.source = (FIXTURES / "sample.rs").read_text(encoding="utf-8")
        for name in ("a.rs", "b.rs", "c.rs", "independent.rs", "consumer.rs"):
            self.write(name, self.source)
        self.write(planner.POLICY, "# Campagne 2026-10-08\nExisting policy: doc_markdown\n")
        self.write("capture.json", {"synthetic": True, "scope": "explicitly reviewed fixture"})
        records = [self.record(i, name) for i, name in
                   enumerate(("a.rs", "b.rs", "c.rs", "independent.rs"), 1)]
        self.inventory = {
            "schema_version": 1, "counts": {}, "clippy": records, "sonar": [],
            "matches": [], "ambiguous_exact_matches": [], "unmerged_hints": [],
            "clippy_unmatched_ids": [], "sonar_unmatched_ids": [],
            "limitations": ["Synthetic conservative overlap inventory."],
            "coverage_percent": {"clippy": 0, "sonar": None, "strict_union": 0},
            "source_stats": {"clippy": {"rows": 4}, "sonar": {"raw_unique_keys": 0}},
        }
        self.evidence = {
            "schema_version": 1, "kind": "clippy-code-evidence", "workspace": str(self.root),
            "snapshot_sha256": "", "inventory_sha256": "",
            "producer": {"version": "fixture-1", "catalogue": "capture.json",
                         "catalogue_sha256": planner.digest(self.root / "capture.json")},
            "mappings": [self.mapping(r) for r in records], "relations": [],
            "queries": [{
                "id": "q1", "tool": "find_symbol", "arguments": {"root": str(self.root)},
                "artifact": "capture.json", "artifact_sha256": planner.digest(self.root / "capture.json"),
                "status": "ok",
                "pagination": {"status": "complete", "pages": 1, "returned": 4,
                               "total": 4, "reason": "fixture exhaustive review"},
                "coverage": {k: "verified" for k in planner.COVERAGE},
            }], "limitations": [],
        }
        self.constraints = []
        self.observations = None
        self.snapshot = {
            "schema_version": 1, "kind": "clippy-source-snapshot", "workspace": str(self.root),
            "head": "1" * 40, "captured_at": "2026-11-14T00:00:00Z",
            "files": [], "inputs": [],
            "configuration": {"toolchain": "synthetic", "features": "fixture", "target": "fixture",
                              "lane": "fixture", "submodules": {}, "index": "unknown"},
            "source_binding": "verified", "limitations": [],
        }
        self.captures = {"capture.json", planner.POLICY}
        self.proof_changes = {}
        self.local("C000004", "independent.rs")

    @staticmethod
    def record(i, name):
        return {"id": f"C{i:06d}", "rule": "clippy::doc_markdown", "message": "Synthetic rustdoc finding",
                "primary_spans": [{"file": name, "range": [1, 4, 1, 11]}],
                "provenance": [{"input": "synthetic.jsonl", "row": i, "level": "warning",
                                "package_id": "synthetic", "target": {"kind": ["lib"]}}],
                "unresolved_reason": None}

    def write(self, name, value):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes((value if isinstance(value, str) else
                          json.dumps(value, sort_keys=True, ensure_ascii=False) + "\n").encode("utf-8"))
        return path

    def ref(self, name):
        return {"path": name, "sha256": planner.digest(self.root / name), "locator": "entire artifact"}

    def mapping(self, record):
        name = record["primary_spans"][0]["file"]
        return {
            "diagnostic_key": "clippy:" + record["id"], "status": "exact",
            "symbols": [{"id": "symbol-" + name, "qualified_name": "fixture::example",
                         "kind": "function", "path": name,
                         "range": {"start_line": 1, "start_column": 0,
                                   "end_line": 4, "end_column": 1, "column_unit": "unicode_scalar"},
                         "source_sha256": planner.digest(self.root / name)}],
            "evidence_refs": ["q1"], "reasons": ["verified unique enclosing fixture symbol"],
        }

    def local(self, identifier, name):
        artifact = "local-" + identifier + ".json"
        self.write(artifact, {
            "diagnostic_key": "clippy:" + identifier, "source_sha256": planner.digest(self.root / name),
            **{k: True for k in ("preserves_api", "preserves_behavior", "preserves_documentation_meaning",
                                "literal_source_only", "not_generated", "no_macro_expansion",
                                "no_cross_file_edit")},
        })
        self.captures.add(artifact)
        mapping = next(m for m in self.evidence["mappings"] if m["diagnostic_key"] == "clippy:" + identifier)
        mapping["evidence_refs"] = [
            ref for ref in mapping["evidence_refs"] if not (isinstance(ref, dict) and ref["path"] == artifact)]
        mapping["evidence_refs"].append(self.ref(artifact))

    def hard(self, identifier, files, keys):
        constraint = {
            "id": identifier, "diagnostic_keys": ["clippy:" + k for k in keys], "files": sorted(files),
            "classification": "must_coedit", "rationale": "Synthetic correction changes shared contract",
            "evidence_refs": ["q1"], "policy_ref": planner.POLICY + "#Campagne 2026-10-08",
            "approval": None, "scope_complete": True,
        }
        authority = f"authority-{identifier}.json"
        self.write(authority, {"reviewer": "independent-reviewer", "proposal_author": "proposal-author",
                               "authorized": True, "constraint_ids": [identifier]})
        closure = f"closure-{identifier}.json"
        self.write(closure, {
            "constraint_id": identifier, "reviewer": "independent-reviewer", "files": sorted(files),
            "scope_complete": True, "rationale": constraint["rationale"],
            "policy_ref": constraint["policy_ref"],
            "source_sha256": {f: planner.digest(self.root / f) for f in sorted(files)},
        })
        self.captures.update((authority, closure))
        constraint["approval"] = {"reviewer": "independent-reviewer", "authority_ref": self.ref(authority),
                                  "reviewed_at": "2026-11-14T00:01:00Z"}
        constraint["evidence_refs"].append(self.ref(closure))
        self.constraints.append(constraint)
        return constraint

    def raw_query(self, identifier, tool, payload, arguments, pagination=None):
        name = "raw-" + identifier + ".json"
        self.write(name, {"isError": False, "content": [{"type": "text", "text": payload if
                        isinstance(payload, str) else json.dumps(payload, ensure_ascii=False)}],
                          "structuredContent": None})
        self.captures.add(name)
        query = {"id": identifier, "tool": tool, "arguments": arguments,
                 "artifact": name, "artifact_sha256": planner.digest(self.root / name), "status": "ok",
                 "pagination": pagination or {"status": "unknown", "pages": 1, "returned": 1,
                                               "total": None, "reason": "no advertised totals"},
                 "coverage": {k: "unknown" for k in planner.COVERAGE}}
        self.evidence["queries"].append(query)
        return query

    def range_mapping(self, index=3, source=None, native_start=0, native_end=3):
        mapping = self.evidence["mappings"][index]
        symbol = mapping["symbols"][0]
        if source is not None:
            self.write(symbol["path"], source)
            symbol["source_sha256"] = planner.digest(self.root / symbol["path"])
        symbol["range"] = {"start_line": native_start + 1, "start_column": 0,
                           "end_line": native_end + 2, "end_column": 0, "column_unit": "unicode_scalar"}
        query = self.raw_query("range-q", "serena_1mcp_find_symbol", [{
            "name_path": "example", "kind": "Function", "relative_path": symbol["path"],
            "body_location": {"start_line": native_start, "end_line": native_end},
        }], {"relative_path": symbol["path"], "name_path_pattern": "example", "include_body": False})
        mapping["evidence_refs"].append(query["id"])
        self.proof_changes[mapping["diagnostic_key"]] = lambda p: p["local_discovery"].update(
            method="source_crosschecked_function_range_v1", query_ids=[query["id"]])
        return query

    def observation_fixture(self, destinations=("consumer.rs",), omitted_consumer=False):
        origin = "crates/apparatus-contracts/src/runtime.rs"
        manifest = "crates/apparatus-contracts/Cargo.toml"
        module = "crates/apparatus-contracts/src/lib.rs"
        target = "apparatus_contracts::runtime::validate_runtime_uid"
        self.write(origin, "pub fn validate_runtime_uid(value: &str) -> Result<(), RuntimeErrorCodeV1> {\n"
                           "    Ok(())\n}\n")
        self.write(manifest, '[package]\nname = "apparatus-contracts"\nversion = "0.0.0"\n')
        self.write(module, "pub mod runtime;\n")
        self.captures.update((manifest, module))
        for destination in set(destinations) - {origin}:
            count = destinations.count(destination)
            if destination in ("a.rs", "b.rs", "c.rs", "independent.rs"):
                self.write(destination, self.source.replace(
                    "    1", "    validate_runtime_uid();\n" * count + "    1"))
                for mapping in self.evidence["mappings"]:
                    if mapping["symbols"][0]["path"] == destination:
                        mapping["symbols"][0]["source_sha256"] = planner.digest(self.root / destination)
                        mapping["symbols"][0]["range"]["end_line"] = 4 + count
            else:
                self.write(destination, "pub fn observed() {\n" + "    validate_runtime_uid();\n" * count + "}\n")
        root_args = {"directory": str(self.root)}
        self.raw_query("sig", "function_signature", {
            "target": target, "signature": {"is_async": False, "self_param": None,
                                          "params": [{"name": "value", "ty": "&str", "by_ref": True,
                                                      "mutability": False}],
                                          "return_type": "Result<(), RuntimeErrorCodeV1>", "generics": []},
        }, {**root_args, "target": target})
        self.raw_query("def", "find_definition",
                       "Found 1 definition(s) for 'validate_runtime_uid':\n"
                       + str(self.root / origin) + ":1:8 (validate_runtime_uid, exact=true)",
                       {**root_args, "symbol_name": "validate_runtime_uid", "exact": True})
        self.raw_query("origin", "serena_1mcp_find_symbol", [{
            "name_path": "validate_runtime_uid", "kind": "Function", "relative_path": origin,
            "body_location": {"start_line": 0, "end_line": 2},
        }], {"relative_path": origin, "name_path_pattern": "validate_runtime_uid"})
        extent = {"start_line": 1, "start_column": 0, "end_line": 4, "end_column": 0,
                  "column_unit": "unicode_scalar"}
        self.write("origin-proof.json", {
            "schema_version": 1, "kind": "clippy-origin-binding", "target": target,
            "source_path": origin, "source_sha256": planner.digest(self.root / origin),
            "declaration_range": extent, "crate_manifest_ref": self.ref(manifest),
            "module_root_ref": self.ref(module), "query_ids": ["sig", "def", "origin"],
            "method": "direct_crate_module_function_v1",
        })
        self.captures.add("origin-proof.json")
        usages = []
        for destination in destinations:
            start = (self.root / destination).read_bytes().index(b"validate_runtime_uid")
            # Repeated sites in the same file are distinct witnesses, not copies.
            while any(row["file"] == destination and row["start"] == start for row in usages):
                data = (self.root / destination).read_bytes()
                start = data.index(b"validate_runtime_uid", start + 1)
            row = {"file": destination, "start": start, "end": start + len("validate_runtime_uid"),
                   "category": "Other", "consumer_module": "fixture::observed"}
            if not omitted_consumer:
                row["consumer_function"] = "fixture::observed"
            usages.append(row)
        pagination = {"status": "complete", "pages": 1, "returned": len(usages),
                      "total": len(usages), "reason": "captured indexed fixture scope"}
        arguments = {**root_args, "target": target, "offset": 0, "limit": 50, "summary": False}
        query = self.raw_query("usage", "who_uses", {
            "target": target, "total_match_count": len(usages), "offset": 0, "limit": 50,
            "summary": False, "returned_match_count": len(usages), "usages": usages,
        }, arguments, pagination)
        raw_artifact, raw_hash = query["artifact"], query["artifact_sha256"]
        self.write("usage-sequence.json", {
            "target": target, "pagination": pagination,
            "pages": [{"id": "usage-page", "tool": "who_uses", "arguments": arguments,
                       "artifact": raw_artifact, "artifact_sha256": raw_hash, "status": "ok"}],
        })
        self.captures.add("usage-sequence.json")
        query.update(artifact="usage-sequence.json", artifact_sha256=planner.digest(self.root / "usage-sequence.json"))
        self.observations = {
            "schema_version": 1, "kind": "clippy-file-observations", "workspace": str(self.root),
            "snapshot_sha256": "", "evidence_sha256": "", "inventory_sha256": "", "limitations": [],
            "observations": [{
                "id": f"observed-{i}", "kind": "reference",
                "origin": {"path": origin, "source_sha256": planner.digest(self.root / origin),
                           "symbol_id": None, "range": extent},
                "destination": {"path": path, "source_sha256": planner.digest(self.root / path),
                                "symbol_id": None, "range": None},
                "query_ids": ["usage"], "origin_binding_ref": {**self.ref("origin-proof.json"), "locator": "/"},
                "identity_status": "file_candidate", "endpoint_status": "source_verified",
                "relationship_scope": "symbol_to_file_candidate", "reasons": [],
            } for i, path in enumerate(destinations)],
        }
        return self.observations["observations"]

    def refresh_capture(self, name):
        """Rebind deliberately forged fixture bytes without hiding them as stale."""
        for query in self.evidence["queries"]:
            if query["artifact"] == name:
                query["artifact_sha256"] = planner.digest(self.root / name)
        if name == "raw-usage.json":
            sequence = planner.read_json(self.root / "usage-sequence.json")
            sequence["pages"][0]["artifact_sha256"] = planner.digest(self.root / name)
            self.write("usage-sequence.json", sequence)
            self.refresh_capture("usage-sequence.json")
        if name == "origin-proof.json":
            for observation in self.observations["observations"]:
                observation["origin_binding_ref"] = {**self.ref(name), "locator": "/"}

    def save(self):
        for mapping in self.evidence.get("mappings", []):
            if mapping["status"] != "exact" or len(mapping["symbols"]) != 1:
                continue
            symbol = mapping["symbols"][0]
            proof_name = "proof-" + mapping["diagnostic_key"].replace(":", "-") + ".json"
            proof = {
                "schema_version": 1, "kind": "clippy-localization-proof",
                "diagnostic_key": mapping["diagnostic_key"], "source_path": symbol["path"],
                "source_sha256": symbol["source_sha256"], "symbol_id": symbol["id"],
                "inventory_column_unit": "unicode_scalar",
                "local_discovery": {
                    "method": "source_crosschecked_simple_function_v1",
                    "scope_range": copy.deepcopy(symbol["range"]),
                    "candidate_symbol_ids": [symbol["id"]], "query_ids": ["q1"],
                    "checked_predicates": {k: True for k in planner.LOCAL_PREDICATES},
                    "limitations": [],
                },
            }
            change = self.proof_changes.get(mapping["diagnostic_key"])
            if change:
                change(proof)
            self.write(proof_name, proof)
            self.captures.add(proof_name)
            mapping["evidence_refs"] = [
                ref for ref in mapping["evidence_refs"]
                if not (isinstance(ref, dict) and ref["path"] == proof_name)]
            mapping["evidence_refs"].append({**self.ref(proof_name), "locator": "/"})
        nc, ns, nm = (len(self.inventory["clippy"]), len(self.inventory["sonar"]),
                      len(self.inventory["matches"]))
        self.inventory["counts"] = {
            "clippy_unique": nc, "sonar_unique_sites": ns,
            "sonar_raw_keys": sum(len(r["issue_keys"]) for r in self.inventory["sonar"]),
            "exact_intersection": nm, "matched_clippy": nm, "matched_sonar": nm,
            "clippy_unmatched": nc - nm, "sonar_unmatched": ns - nm, "strict_union": nc + ns - nm,
        }
        raw_clippy = sum(len(r["provenance"]) for r in self.inventory["clippy"])
        raw_sonar = self.inventory["counts"]["sonar_raw_keys"]
        self.inventory["source_stats"] = {
            "clippy": {"rows": raw_clippy + 1, "successful_files": 1,
                       "excluded_non_compiler_rows": 0, "excluded_compiler_messages": 0,
                       "clippy_diagnostic_rows": raw_clippy},
            "sonar": {"pages": 1, "paging_total": raw_sonar, "raw_issue_rows": raw_sonar,
                      "raw_unique_keys": raw_sonar},
        }
        self.inventory["coverage_percent"] = {
            label: round(100 * nm / denominator, 6) if denominator else None
            for label, denominator in (("clippy", nc), ("sonar", ns), ("strict_union", nc + ns - nm))}
        for namespace in ("clippy", "sonar"):
            matched = {m[namespace + "_id"] for m in self.inventory["matches"]}
            self.inventory[namespace + "_unmatched_ids"] = [
                r["id"] for r in self.inventory[namespace] if r["id"] not in matched]
        self.write("inventory.json", self.inventory)
        self.snapshot["files"] = [{"path": p.relative_to(self.root).as_posix(), "sha256": planner.digest(p)}
                                  for p in sorted(self.root.rglob("*.rs"))]
        self.snapshot["inputs"] = [
            {"role": "inventory" if name == "inventory.json" else "capture", **self.ref(name)}
            for name in sorted(self.captures | {"inventory.json"})]
        self.write("snapshot.json", self.snapshot)
        self.evidence["inventory_sha256"] = planner.digest(self.inventory_path)
        self.evidence["snapshot_sha256"] = planner.digest(self.snapshot_path)
        self.write("evidence.json", self.evidence)
        self.write("constraints.json", {
            "schema_version": 1, "kind": "clippy-coedit-constraints", "workspace": str(self.root),
            "snapshot_sha256": planner.digest(self.snapshot_path),
            "inventory_sha256": planner.digest(self.inventory_path), "constraints": self.constraints,
        })
        if self.observations is not None:
            self.observations.update(snapshot_sha256=planner.digest(self.snapshot_path),
                                     evidence_sha256=planner.digest(self.evidence_path),
                                     inventory_sha256=planner.digest(self.inventory_path))
            self.write("observations.json", self.observations)

    def plan(self):
        return planner.build_plan(self.inventory_path, self.evidence_path,
                                  self.snapshot_path, self.constraints_path,
                                  self.root / "observations.json" if self.observations is not None else None)


class PlannerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="constraint-plan-", dir=FIXTURES)
        self.addCleanup(self.temp.cleanup)
        self.f = Fixture(Path(self.temp.name))

    def test_transitive_components_include_consumer_and_independent_local_singleton(self):
        self.f.hard("ab", ["a.rs", "b.rs"], ["C000001", "C000002"])
        self.f.hard("bc", ["b.rs", "c.rs", "consumer.rs"], ["C000002", "C000003"])
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(plan["confirmed_multi_file_components"], 1)
        group, singleton = plan["components"]
        self.assertEqual(group["files"], ["a.rs", "b.rs", "c.rs", "consumer.rs"])
        self.assertEqual(group["outside_diagnostic_files"], ["consumer.rs"])
        self.assertEqual(group["constraint_ids"], ["ab", "bc"])
        self.assertEqual(group["dispatchability"], "candidate")
        self.assertEqual(singleton["files"], ["independent.rs"])
        self.assertEqual(singleton["dispatchability"], "candidate")

    def test_same_rule_and_dependency_do_not_union(self):
        self.f.evidence["relations"].append({
            "id": "call-a-b", "kind": "call", "from_symbol": "symbol-a.rs", "to_symbol": "symbol-b.rs",
            "evidence_refs": ["q1"], "coverage": {k: "verified" for k in planner.COVERAGE}})
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(len(plan["components"]), 4)
        self.assertEqual(plan["confirmed_multi_file_components"], 0)
        for group in plan["components"][:2]:
            self.assertIn("relation:call-a-b", group["blocker_ids"])

    def test_candidate_and_rejected_constraints_never_union(self):
        for classification in ("candidate", "rejected"):
            with self.subTest(classification=classification):
                c = self.f.hard("ab", ["a.rs", "b.rs"], ["C000001", "C000002"])
                c["classification"] = classification
                c["approval"] = None
                self.f.save()
                self.assertEqual(len(self.f.plan()["components"]), 4)
                self.f.constraints.clear()

    def test_unsafe_hard_constraints_fail_closed(self):
        cases = {
            "missing_approval": lambda c: c.update(approval=None),
            "missing_scope": lambda c: c.update(scope_complete=False),
            "missing_closure": lambda c: c.update(evidence_refs=["q1"]),
            "unbound_consumer": lambda c: c["files"].append("unknown.rs"),
            "unknown_policy": lambda c: c.update(policy_ref=planner.POLICY + "#invented"),
            "missing_rationale": lambda c: c.update(rationale=""),
            "old_review": lambda c: c["approval"].update(reviewed_at="2026-11-13T00:00:00Z"),
        }
        for name, mutate in cases.items():
            with self.subTest(name=name):
                self.f.constraints.clear()
                c = self.f.hard("ab", ["a.rs", "b.rs"], ["C000001", "C000002"])
                mutate(c)
                self.f.save()
                with self.assertRaises(planner.Invalid):
                    self.f.plan()

    def test_self_approval_and_missing_delegation_rejected(self):
        for change in ({"proposal_author": "independent-reviewer"}, {"authorized": False}):
            with self.subTest(change=change):
                self.f.constraints.clear()
                c = self.f.hard("ab", ["a.rs", "b.rs"], ["C000001", "C000002"])
                path = self.f.root / "authority-ab.json"
                authority = planner.read_json(path)
                authority.update(change)
                self.f.write(path.name, authority)
                c["approval"]["authority_ref"] = self.f.ref(path.name)
                self.f.save()
                with self.assertRaises(planner.Invalid):
                    self.f.plan()

    def test_dirty_source_retains_unresolved_but_blocks_approved_constraint(self):
        self.f.save()
        self.f.write("a.rs", self.f.source + "// dirty\n")
        plan = self.f.plan()
        self.assertEqual(plan["diagnostics"][0]["mapping_status"], "stale")
        self.assertEqual(plan["components"][0]["dispatchability"], "blocked")
        self.assertEqual(plan["accounting"]["retained_diagnostic_records"], 4)
        self.f.write("a.rs", self.f.source)
        self.f.hard("ab", ["a.rs", "b.rs"], ["C000001", "C000002"])
        self.f.save()
        self.f.write("a.rs", self.f.source + "// dirty\n")
        with self.assertRaises(planner.Invalid):
            self.f.plan()

    def test_stale_capture_or_hash_binding_rejected(self):
        self.f.save()
        self.f.write("capture.json", {"changed": True})
        with self.assertRaisesRegex(planner.Invalid, "stale snapshot input"):
            self.f.plan()
        self.f.save()
        self.f.evidence["snapshot_sha256"] = "0" * 64
        self.f.write("evidence.json", self.f.evidence)
        with self.assertRaisesRegex(planner.Invalid, "binding"):
            self.f.plan()

    def test_outside_root_inventory_is_retained_unbounded_not_guessed(self):
        self.f.inventory["clippy"][0]["primary_spans"][0]["file"] = "../outside.rs"
        self.f.evidence["mappings"][0].update(status="outside_root", symbols=[])
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(plan["diagnostics"][0]["files"], [])
        self.assertEqual(plan["global_independence"], "unproven")
        independent = next(c for c in plan["components"] if c["files"] == ["independent.rs"])
        self.assertEqual(independent["dispatchability"], "candidate")

    def test_outside_root_snapshot_and_case_aliases_rejected(self):
        self.f.save()
        snapshot = planner.read_json(self.f.snapshot_path)
        snapshot["files"].append({"path": "../outside.rs", "sha256": "0" * 64})
        self.f.write("snapshot.json", snapshot)
        with self.assertRaises(planner.Invalid):
            self.f.plan()
        paths = planner.Sources(str(self.f.root))
        paths.path("a.rs")
        with self.assertRaisesRegex(planner.Invalid, "case alias"):
            paths.path("A.rs")
        self.assertEqual(paths.path(r"nested\\repeated\\file.rs")[0], "nested/repeated/file.rs")
        for name in ("./a.rs", r"C:\a.rs", "//server/share", "a/../b.rs", "a.rs:stream"):
            with self.subTest(name=name), self.assertRaises(planner.Invalid):
                paths.path(name)

    def test_same_file_unknown_and_overlapping_blockers_propagate(self):
        self.f.local("C000001", "a.rs")
        record = self.f.record(5, "a.rs")
        record["rule"] = "clippy::unused_async"
        self.f.inventory["clippy"].append(record)
        m = self.f.mapping(record)
        m.update(status="unknown", symbols=[], reasons=["macro expansion coordinates unknown"])
        self.f.evidence["mappings"].append(m)
        for identifier, other in (("ab", "b.rs"), ("ac", "c.rs")):
            c = self.f.hard(identifier, ["a.rs", other], ["C000001"])
            c["classification"] = "candidate"
        self.f.save()
        plan = self.f.plan()
        a = plan["components"][0]
        self.assertEqual(a["dispatchability"], "blocked")
        self.assertEqual(a["diagnostic_keys"], ["clippy:C000001", "clippy:C000005"])
        self.assertEqual(plan["diagnostics"][0]["local_policy"]["classification"], "local_candidate")
        self.assertIn("diagnostic:clippy:C000005", a["blocker_ids"])
        self.assertIn("constraint:ab", a["blocker_ids"])
        self.assertIn("constraint:ac", a["blocker_ids"])
        self.assertEqual(len(plan["components"]), 4)

    def test_unknown_coverage_not_repaired_by_complete_pagination(self):
        for dimension in ("private", "macros", "traits", "features", "tests", "freshness"):
            with self.subTest(dimension=dimension):
                self.f.evidence["queries"][0]["coverage"][dimension] = "unknown"
                self.f.save()
                plan = self.f.plan()
                self.assertEqual(plan["global_independence"], "unproven")
                self.assertEqual(plan["queries"][0]["pagination"]["status"], "complete")
                self.assertTrue(all(d["mapping_status"] == "exact" for d in plan["diagnostics"]))
                self.assertEqual(plan["components"][-1]["dispatchability"], "candidate")
                self.f.evidence["queries"][0]["coverage"][dimension] = "verified"

    def test_exact_local_proof_survives_unknown_pagination_and_external_references(self):
        q = self.f.evidence["queries"][0]
        q["pagination"].update(status="unknown", total=None, reason="symbol API advertises no totals")
        q["coverage"] = {k: "unknown" for k in planner.COVERAGE}
        self.f.save()
        plan = self.f.plan()
        self.assertTrue(all(d["mapping_status"] == "exact" for d in plan["diagnostics"]))
        independent = plan["diagnostics"][-1]
        self.assertEqual(independent["local_policy"]["classification"], "local_candidate")
        self.assertEqual(plan["components"][-1]["dispatchability"], "candidate")
        self.assertEqual(plan["global_independence"], "unproven")
        self.assertEqual(plan["queries"][0]["pagination"]["status"], "unknown")
        self.assertEqual(plan["queries"][0]["coverage"]["private"], "unknown")

    def test_local_proof_true_booleans_do_not_hide_nested_declaration(self):
        source = "/// Literal documentation.\npub fn example() -> usize {\n    fn nested() {}\n    1\n}\n"
        self.f.write("a.rs", source)
        symbol = self.f.evidence["mappings"][0]["symbols"][0]
        symbol["source_sha256"] = planner.digest(self.f.root / "a.rs")
        symbol["range"]["end_line"] = 5
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(plan["diagnostics"][0]["mapping_status"], "unknown")
        self.assertTrue(any("nested/competing" in reason
                            for reason in plan["diagnostics"][0]["mapping_reasons"]))
        self.assertEqual(plan["components"][0]["dispatchability"], "blocked")
        self.assertEqual(plan["components"][-1]["dispatchability"], "candidate")

    def test_source_local_subset_rejects_unsupported_competing_syntax(self):
        for source in (
            self.f.source + "struct Omitted;\n",
            self.f.source.replace("    1", '    "fn fake() {}"'),
            self.f.source.replace("    1", "    /* competing? */ 1"),
            self.f.source.replace("pub fn", "#[inline]\npub fn"),
            self.f.source.replace("    1", "    helper!()"),
            self.f.source.replace("    1", "    let closure = || 1; 1"),
        ):
            with self.subTest(source=source):
                self.f.write("a.rs", source)
                symbol = self.f.evidence["mappings"][0]["symbols"][0]
                symbol["source_sha256"] = planner.digest(self.f.root / "a.rs")
                # The trailing omitted item is outside the claimed extent.
                symbol["range"]["end_line"] = 5 if "#[inline]" in source else 4
                self.f.save()
                self.assertEqual(self.f.plan()["diagnostics"][0]["mapping_status"], "unknown")

    def test_shared_proof_required_fields_and_local_candidates_are_replayed(self):
        cases = {
            "method": lambda p: p["local_discovery"].update(method="invented"),
            "competing": lambda p: p["local_discovery"].update(
                candidate_symbol_ids=[p["symbol_id"], "omitted"]),
            "local_limit": lambda p: p["local_discovery"].update(limitations=["nested body unknown"]),
            "unreferenced_query": lambda p: p["local_discovery"].update(query_ids=["missing"]),
            "false_summary": lambda p: p["local_discovery"]["checked_predicates"].update(
                nested_or_competing_symbols_excluded=False),
            "missing_field": lambda p: p.pop("inventory_column_unit"),
            "wrong_symbol": lambda p: p.update(symbol_id="other"),
            "wrong_source": lambda p: p.update(source_path="b.rs"),
            "wrong_scope": lambda p: p["local_discovery"]["scope_range"].update(start_line=2),
        }
        for name, mutate in cases.items():
            with self.subTest(name=name):
                self.f.proof_changes["clippy:C000001"] = mutate
                self.f.save()
                self.assertEqual(self.f.plan()["diagnostics"][0]["mapping_status"], "unknown")

    def test_overview_alone_and_identifier_span_do_not_prove_localization(self):
        query = copy.deepcopy(self.f.evidence["queries"][0])
        query.update(id="overview", tool="get_symbols_overview")
        self.f.evidence["queries"].append(query)
        self.f.evidence["mappings"][0]["evidence_refs"].append("overview")
        self.f.proof_changes["clippy:C000001"] = lambda proof: proof["local_discovery"].update(
            query_ids=["overview"])
        self.f.save()
        self.assertEqual(self.f.plan()["diagnostics"][0]["mapping_status"], "unknown")
        self.f.proof_changes.clear()
        self.f.evidence["mappings"][0]["symbols"][0]["range"].update(
            start_line=2, start_column=7, end_line=2, end_column=14)
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(plan["diagnostics"][0]["mapping_status"], "unknown")
        self.assertEqual(plan["components"][-1]["dispatchability"], "candidate")

    def test_coordinate_or_policy_context_without_proof_discriminator_is_not_localization(self):
        self.f.save()
        self.f.evidence["mappings"][3]["evidence_refs"] = [
            ref for ref in self.f.evidence["mappings"][3]["evidence_refs"]
            if not (isinstance(ref, dict) and ref["path"].startswith("proof-"))]
        self.f.write("evidence.json", self.f.evidence)
        plan = self.f.plan()
        self.assertEqual(plan["diagnostics"][-1]["mapping_status"], "unknown")
        self.assertEqual(plan["diagnostics"][-1]["local_policy"]["classification"], "needs_review")

    def test_bounded_incomplete_relation_query_does_not_contaminate_independent_components(self):
        for index, name in enumerate(("a.rs", "b.rs", "c.rs"), 1):
            self.f.local(f"C{index:06d}", name)
        query = copy.deepcopy(self.f.evidence["queries"][0])
        query.update(id="refs-q", tool="who_uses", arguments={"file": "independent.rs"}, status="partial")
        query["pagination"].update(status="partial", returned=1, total=10, reason="bounded capture")
        query["coverage"]["private"] = "unknown"
        self.f.evidence["queries"].append(query)
        self.f.evidence["relations"].append({
            "id": "observed-a-b", "kind": "reference", "from_symbol": "symbol-a.rs",
            "to_symbol": "symbol-b.rs", "evidence_refs": ["refs-q"],
            "coverage": {k: "unknown" for k in planner.COVERAGE}})
        self.f.save()
        plan = self.f.plan()
        blocker = next(b for b in plan["blockers"] if b["id"] == "query:refs-q")
        self.assertEqual(blocker["files"], ["a.rs", "b.rs"])
        self.assertTrue(all(d["mapping_status"] == "exact" for d in plan["diagnostics"]))
        self.assertEqual([c["dispatchability"] for c in plan["components"]],
                         ["blocked", "blocked", "candidate", "candidate"])

    def test_unbounded_caveat_denies_global_certification_without_erasing_local_facts(self):
        for index, name in enumerate(("a.rs", "b.rs", "c.rs"), 1):
            self.f.local(f"C{index:06d}", name)
        self.f.save()
        baseline = self.f.plan()
        self.f.evidence["limitations"] = ["External index completeness has not been established."]
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(plan["global_independence"], "unproven")
        self.assertEqual(plan["diagnostics"], baseline["diagnostics"])
        self.assertEqual(plan["components"], baseline["components"])
        self.assertTrue(all(c["dispatchability"] == "candidate" for c in plan["components"]))
        self.assertEqual(plan["blockers"][0]["scope"], "unbounded")

    def test_localization_proof_does_not_relax_hard_constraint_closure(self):
        self.f.hard("ab", ["a.rs", "b.rs"], ["C000001", "C000002"])
        q = self.f.evidence["queries"][0]
        q["pagination"].update(status="unknown", total=None, reason="no totals")
        q["coverage"]["features"] = "unknown"
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(plan["confirmed_multi_file_components"], 0)
        self.assertEqual(len(plan["components"]), 4)
        self.assertTrue(all(d["mapping_status"] == "exact" for d in plan["diagnostics"]))
        self.assertEqual(plan["unqualified_hard_constraints"], [{"id": "ab", "incomplete_query_ids": ["q1"]}])
        self.assertTrue(all("constraint:ab" in c["blocker_ids"] for c in plan["components"][:2]))

    def test_review_c1_mixed_complete_partial_closure_never_unions(self):
        constraint = self.f.hard("ab", ["a.rs", "b.rs"], ["C000001", "C000002"])
        query = copy.deepcopy(self.f.evidence["queries"][0])
        query.update(id="refs-incomplete", tool="who_uses", status="partial")
        query["pagination"].update(status="partial", returned=1, total=10, reason="nine consumers missing")
        query["coverage"]["traits"] = "unknown"
        self.f.evidence["queries"].append(query)
        constraint["evidence_refs"].append("refs-incomplete")
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(plan["confirmed_multi_file_components"], 0)
        self.assertEqual(plan["unqualified_hard_constraints"],
                         [{"id": "ab", "incomplete_query_ids": ["refs-incomplete"]}])
        self.assertEqual([c["files"] for c in plan["components"]],
                         [["a.rs"], ["b.rs"], ["c.rs"], ["independent.rs"]])
        for component in plan["components"][:2]:
            self.assertEqual(component["dispatchability"], "blocked")
            self.assertIn("constraint:ab", component["blocker_ids"])
            self.assertIn("query:refs-incomplete", component["blocker_ids"])
            self.assertEqual(component["constraint_ids"], [])
        self.assertTrue(all(d["mapping_status"] == "exact" for d in plan["diagnostics"]))
        self.assertEqual(plan["components"][-1]["dispatchability"], "candidate")

    def test_review_c2_candidate_blocks_declared_and_diagnostic_participants(self):
        constraint = self.f.hard("ab", ["a.rs", "b.rs"], ["C000004"])
        constraint["classification"] = "candidate"
        self.f.save()
        plan = self.f.plan()
        independent = next(c for c in plan["components"] if c["files"] == ["independent.rs"])
        self.assertEqual(independent["dispatchability"], "blocked")
        self.assertIn("constraint:ab", independent["blocker_ids"])
        blocker = next(b for b in plan["blockers"] if b["id"] == "constraint:ab")
        self.assertEqual(blocker["files"], ["a.rs", "b.rs", "independent.rs"])
        self.assertEqual(plan["confirmed_multi_file_components"], 0)
        self.assertEqual(len(plan["components"]), 4)
        self.assertEqual(plan["diagnostics"][-1]["mapping_status"], "exact")
        self.assertEqual(plan["diagnostics"][-1]["local_policy"]["classification"], "local_candidate")

    def test_error_unsupported_partial_queries_never_become_empty_success(self):
        for status in ("error", "unsupported", "partial"):
            with self.subTest(status=status):
                q = self.f.evidence["queries"][0]
                q["status"] = status
                q["pagination"].update(status="unknown", total=None, reason="explicit failure")
                self.f.save()
                plan = self.f.plan()
                self.assertEqual(plan["queries"][0]["status"], status)
                self.assertEqual(plan["accounting"]["retained_diagnostic_records"], 4)
                self.assertEqual(plan["global_independence"], "unproven")

    def test_inconsistent_semantic_totals_rejected(self):
        self.f.evidence["queries"][0]["pagination"]["total"] = 5
        self.f.save()
        with self.assertRaisesRegex(planner.Invalid, "pagination"):
            self.f.plan()

    def test_uncertain_aliases_retain_both_ids_and_issue_provenance(self):
        self.f.inventory["sonar"].append({
            "id": "S000001", "rule": "clippy::doc_markdown", "message": "different message",
            "file": "a.rs", "component": "synthetic:a.rs", "range": [1, 4, 1, 11], "line": 1,
            "issue_keys": ["issue-one", "issue-two"], "unresolved_reason": None})
        self.f.inventory["unmerged_hints"].append({
            "type": "same_full_range_different_message", "clippy_id": "C000001", "sonar_id": "S000001"})
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(plan["accounting"]["strict_union"], 5)
        self.assertEqual(plan["accounting"]["retained_diagnostic_records"], 5)
        self.assertEqual(plan["inventory"]["sonar"][0]["issue_keys"], ["issue-one", "issue-two"])
        self.assertIsNone(plan["accounting"]["semantic_defect_count"])
        self.assertEqual(len(plan["components"]), 4)
        self.f.inventory["sonar"][0]["message"] = self.f.inventory["clippy"][0]["message"]
        self.f.inventory["matches"].append({"clippy_id": "C000001", "sonar_id": "S000001",
                                            "sonar_issue_keys": ["issue-one", "issue-two"],
                                            "clippy_provenance": self.f.inventory["clippy"][0]["provenance"]})
        self.f.inventory["unmerged_hints"].clear()
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(plan["accounting"]["strict_union"], 4)
        self.assertEqual(plan["accounting"]["retained_diagnostic_records"], 5)

    def test_unicode_units_and_split_codepoints(self):
        source = "aé😀z\n"
        expected = (1, 3)
        self.assertEqual(planner.position(source, 1, 3, "unicode_scalar"), expected)
        self.assertEqual(planner.position(source, 1, 7, "utf8_byte"), expected)
        self.assertEqual(planner.position(source, 1, 4, "utf16"), expected)
        self.assertEqual(planner.position("a\u2028b\n", 1, 3, "unicode_scalar"), (1, 3))
        for unit, column in (("utf8_byte", 2), ("utf16", 3)):
            with self.subTest(unit=unit), self.assertRaises(planner.Invalid):
                planner.position(source, 1, column, unit)
        with self.assertRaises(planner.Invalid):
            planner.source_range({"start_line": 0, "start_column": 0, "end_line": 1,
                                  "end_column": 1, "column_unit": "unicode_scalar"}, source)

    def test_unicode_inventory_containment_in_normalized_symbol_units(self):
        line = "/// é😀 literal documentation."
        self.f.write("a.rs", line + "\npub fn example() -> usize {\n    1\n}\n")
        record = self.f.inventory["clippy"][0]
        record["primary_spans"][0]["range"] = [1, 4, 1, 6]
        symbol = self.f.evidence["mappings"][0]["symbols"][0]
        symbol["source_sha256"] = planner.digest(self.f.root / "a.rs")
        for unit, end in (("utf8_byte", len(line[:6].encode("utf-8"))),
                          ("utf16", len(line[:6].encode("utf-16-le")) // 2),
                          ("unicode_scalar", 6)):
            with self.subTest(unit=unit):
                symbol["range"].update(end_line=4, end_column=1, column_unit=unit)
                record["primary_spans"][0]["range"] = [1, 4, 1, end]
                self.f.proof_changes["clippy:C000001"] = lambda proof: proof.update(inventory_column_unit=unit)
                self.f.save()
                self.assertEqual(self.f.plan()["diagnostics"][0]["mapping_status"], "exact")

    def test_ambiguous_mapping_and_missing_ranges_are_not_exact(self):
        self.f.evidence["mappings"][0]["status"] = "ambiguous"
        self.f.inventory["clippy"][1]["primary_spans"][0]["range"] = None
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(plan["diagnostics"][0]["mapping_status"], "ambiguous")
        self.assertEqual(plan["diagnostics"][1]["mapping_status"], "unknown")
        self.assertTrue(all(c["dispatchability"] == "blocked" for c in plan["components"][:2]))

    def test_unresolved_relation_endpoint_is_not_an_invented_node(self):
        self.f.evidence["relations"].append({
            "id": "unresolved", "kind": "call", "from_symbol": "symbol-a.rs", "to_symbol": "unseen",
            "evidence_refs": ["q1"], "coverage": {k: "unknown" for k in planner.COVERAGE}})
        self.f.save()
        with self.assertRaisesRegex(planner.Invalid, "unresolved relation endpoint"):
            self.f.plan()

    def test_malformed_accounting_and_alias_provenance_rejected(self):
        self.f.save()
        inventory = planner.read_json(self.f.inventory_path)
        inventory["counts"]["strict_union"] = 620
        self.f.write("inventory.json", inventory)
        # Rebind inputs so this reaches accounting rather than a hash mismatch.
        self.f.snapshot["inputs"] = [
            {**r, "sha256": planner.digest(self.f.root / r["path"])}
            for r in self.f.snapshot["inputs"]]
        self.f.write("snapshot.json", self.f.snapshot)
        self.f.evidence.update(inventory_sha256=planner.digest(self.f.inventory_path),
                               snapshot_sha256=planner.digest(self.f.snapshot_path))
        self.f.write("evidence.json", self.f.evidence)
        with self.assertRaisesRegex(planner.Invalid, "accounting"):
            self.f.plan()

    def test_cli_protects_captures_and_sources_not_only_three_inputs(self):
        self.f.save()
        for output_name in ("capture.json", "a.rs"):
            with self.subTest(output=output_name):
                path = self.f.root / output_name
                before = path.read_bytes()
                args = ["--inventory", str(self.f.inventory_path), "--snapshot", str(self.f.snapshot_path),
                        "--evidence", str(self.f.evidence_path), "--output", str(path)]
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    planner.main(args)
                self.assertEqual(path.read_bytes(), before)

    def test_bare_rule_whitelist_never_classifies_local(self):
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(plan["diagnostics"][0]["local_policy"]["classification"], "needs_review")
        for rule in planner.LOCAL_RULES:
            with self.subTest(rule=rule):
                self.f.inventory["clippy"][0]["rule"] = "clippy::" + rule
                self.f.save()
                self.assertEqual(self.f.plan()["diagnostics"][0]["local_policy"]["classification"],
                                 "needs_review")

    def test_deterministic_json_markdown_and_order(self):
        self.f.hard("ab", ["a.rs", "b.rs"], ["C000001", "C000002"])
        self.f.save()
        first = self.f.plan()
        self.assertEqual(json.dumps(first, sort_keys=True), json.dumps(self.f.plan(), sort_keys=True))
        self.assertEqual(planner.markdown(first), planner.markdown(self.f.plan()))
        self.f.evidence["mappings"].reverse()
        self.f.save()
        second = self.f.plan()
        # Input hashes correctly reflect different raw evidence ordering.
        first.pop("input_hashes")
        second.pop("input_hashes")
        self.assertEqual(first, second)
        self.assertIn("Semantic defect count: unknown", planner.markdown(first))
        self.assertIn("private=verified", planner.markdown(first))

    def test_output_collisions_and_atomic_validation(self):
        self.f.save()
        output = self.f.root / "plan.json"
        report = self.f.root / "plan.md"
        args = ["--inventory", str(self.f.inventory_path), "--snapshot", str(self.f.snapshot_path),
                "--evidence", str(self.f.evidence_path), "--output", str(output), "--report", str(report)]
        self.assertEqual(planner.main(args), 0)
        before = output.read_bytes()
        self.assertEqual(planner.main(args), 0)
        self.assertEqual(output.read_bytes(), before)
        self.f.evidence["snapshot_sha256"] = "0" * 64
        self.f.write("evidence.json", self.f.evidence)
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            planner.main(args)
        self.assertEqual(output.read_bytes(), before)
        with self.assertRaisesRegex(planner.Invalid, "collision"):
            planner.publish([(output, "x"), (output, "y")], [])
        with self.assertRaisesRegex(planner.Invalid, "collision"):
            planner.publish([(self.f.inventory_path, "x")], [self.f.inventory_path])
        with mock.patch.object(planner.os, "replace", side_effect=OSError("synthetic replace failure")):
            with self.assertRaises(OSError):
                planner.publish([(output, "new")], [])
        self.assertEqual(output.read_bytes(), before)
        self.assertFalse(list(self.f.root.glob(".plan*.tmp")))

    def test_malformed_shapes_references_and_required_fields_rejected(self):
        baseline = copy.deepcopy(self.f.evidence)
        for field in ("producer", "mappings", "relations", "queries", "limitations"):
            with self.subTest(field=field):
                self.f.evidence = copy.deepcopy(baseline)
                del self.f.evidence[field]
                self.f.save()
                with self.assertRaises(planner.Invalid):
                    self.f.plan()
        self.f.evidence = baseline
        self.f.evidence["mappings"][0]["evidence_refs"] = ["absent-query"]
        self.f.save()
        with self.assertRaisesRegex(planner.Invalid, "unknown query reference"):
            self.f.plan()

    def test_missing_mapping_and_absent_file_retained(self):
        self.f.evidence["mappings"].pop()
        self.f.inventory["clippy"].append({
            **self.f.record(5, "a.rs"), "primary_spans": [], "unresolved_reason": "no_primary_span"})
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(len(plan["diagnostics"]), 5)
        self.assertEqual(plan["global_independence"], "unproven")
        self.assertEqual(plan["mapping_status_counts"]["unknown"], 2)

    def test_source_changes_during_plan_fail_without_output(self):
        self.f.save()
        real_digest = planner.digest
        calls = []

        def changing_digest(path):
            if Path(path).name == "a.rs":
                calls.append(path)
                if len(calls) == 2:
                    return "0" * 64
            return real_digest(path)

        with mock.patch.object(planner, "digest", side_effect=changing_digest):
            with self.assertRaisesRegex(planner.Invalid, "source changed during planning"):
                self.f.plan()

    def test_hardlink_output_input_alias_rejected(self):
        self.f.save()
        alias = self.f.root / "alias.json"
        try:
            os.link(self.f.inventory_path, alias)
        except OSError:
            self.skipTest("hard links unavailable")
        with self.assertRaisesRegex(planner.Invalid, "filesystem alias"):
            planner.publish([(alias, "overwrite")], [self.f.inventory_path])

    def test_range_local_mixed_file_replays_hashed_queries_preserving_crlf(self):
        source = ("use std::fmt;\r\nstruct Other;\r\n/// A literal documentation sentence.\r\n"
                  "pub fn example() -> usize {\r\n    let value = || 1;\r\n"
                  "    format!(\"{:?}\", value());\r\n    value()\r\n}\r\n")
        self.f.range_mapping(source=source, native_start=2, native_end=7)
        self.f.inventory["clippy"][3]["primary_spans"][0]["range"] = [3, 4, 3, 11]
        self.f.local("C000004", "independent.rs")
        self.f.save()
        with mock.patch.object(planner._RANGES, "verify_function_range",
                               wraps=planner._RANGES.verify_function_range) as replay:
            plan = self.f.plan()
        self.assertIn("\r\n", replay.call_args.args[0])
        self.assertEqual(plan["diagnostics"][-1]["mapping_status"], "exact")
        self.assertEqual(plan["diagnostics"][-1]["local_policy"]["classification"], "local_candidate")
        self.assertEqual(plan["queries"][-1]["pagination"]["status"], "unknown")
        self.assertEqual(plan["global_independence"], "unproven")

    def test_exact_header_mapping_does_not_authorize_documentation_policy(self):
        query = self.f.range_mapping()
        self.f.inventory["clippy"][3]["rule"] = "clippy::missing_panics_doc"
        self.f.inventory["clippy"][3]["primary_spans"][0]["range"] = [2, 0, 2, 26]
        self.f.save()
        plan = self.f.plan()
        self.assertEqual(query["status"], "ok")
        self.assertEqual(plan["diagnostics"][-1]["mapping_status"], "exact")
        self.assertEqual(plan["diagnostics"][-1]["local_policy"]["classification"], "needs_review")

    def test_range_local_forged_dual_payload_and_request_mismatch_fail_closed(self):
        query = self.f.range_mapping()
        baseline = planner.read_json(self.f.root / query["artifact"])
        for raw, args in (
            ({**baseline, "structuredContent": {"result": "[]"}}, query["arguments"]),
            (baseline, {"relative_path": "a.rs", "name_path_pattern": "example"}),
            ({**baseline, "isError": True}, query["arguments"]),
        ):
            with self.subTest(raw=raw, args=args):
                query["arguments"] = args
                self.f.write(query["artifact"], raw)
                self.f.refresh_capture(query["artifact"])
                self.f.save()
                self.assertEqual(self.f.plan()["diagnostics"][-1]["mapping_status"], "unknown")

    def test_range_local_shared_high_regressions_are_not_overridden_by_proof_flags(self):
        cases = [
            ("pub fn example() {\n    unsafe extern \"C\" {\n        fn foreign(x: i32);\n    }\n}\n",
             4, [3, 8, 3, 25]),
            ("pub fn example() {\n    #![cfg(feature = \"x\")]\n    1;\n}\n", 3, [1, 0, 1, 18]),
        ]
        for source, end, span in cases:
            with self.subTest(source=source):
                # Reuse the fixture; remove the previous range query explicitly.
                self.f.evidence["queries"] = [q for q in self.f.evidence["queries"] if q["id"] != "range-q"]
                mapping = self.f.evidence["mappings"][0]
                mapping["evidence_refs"] = [r for r in mapping["evidence_refs"] if r != "range-q"]
                self.f.range_mapping(index=0, source=source, native_start=0, native_end=end)
                self.f.inventory["clippy"][0]["primary_spans"][0]["range"] = span
                self.f.save()
                self.assertEqual(self.f.plan()["diagnostics"][0]["mapping_status"], "unknown")
        query = next(q for q in self.f.evidence["queries"] if q["id"] == "range-q")
        self.f.write("a.rs", self.f.source)
        self.f.evidence["mappings"][0]["symbols"][0].update(
            range={"start_line": 1, "start_column": 0, "end_line": 5, "end_column": 0,
                   "column_unit": "unicode_scalar"}, source_sha256=planner.digest(self.f.root / "a.rs"))
        self.f.inventory["clippy"][0]["primary_spans"][0]["range"] = [1, 4, 1, 11]
        raw = planner.read_json(self.f.root / query["artifact"])
        raw["content"][0]["text"] = json.dumps([{"name_path": "Wrong/example", "kind": "Function",
                                                "relative_path": "a.rs",
                                                "body_location": {"start_line": 0, "end_line": 3}}])
        self.f.write(query["artifact"], raw)
        self.f.refresh_capture(query["artifact"])
        self.f.save()
        self.assertEqual(self.f.plan()["diagnostics"][0]["mapping_status"], "unknown")

    def test_work_graph_exists_without_observations_and_reuses_hard_components(self):
        self.f.hard("ab", ["a.rs", "b.rs"], ["C000001", "C000002"])
        self.f.hard("bc", ["b.rs", "c.rs", "consumer.rs"], ["C000002", "C000003"])
        self.f.save()
        graph = self.f.plan()["work_graph"]
        self.assertEqual(len(graph["nodes"]), 5)
        self.assertEqual(len(graph["proposed_groups"]), 2)
        self.assertEqual(graph["coverage"]["hard_edges"], 3)
        self.assertTrue(all(e["class"] == "hard_must_coedit" for e in graph["edges"]))
        self.assertEqual(graph["coverage"]["edge_absence"], "not_observed")
        self.assertEqual(graph["proposed_groups"][0]["files"], ["a.rs", "b.rs", "c.rs", "consumer.rs"])

    def test_observation_site_edges_boundaries_and_outside_consumers_never_union(self):
        origin = "crates/apparatus-contracts/src/runtime.rs"
        self.f.observation_fixture(("consumer.rs", "consumer.rs", "a.rs", origin), omitted_consumer=True)
        self.f.save()
        plan = self.f.plan()
        graph = plan["work_graph"]
        self.assertEqual(graph["coverage"]["files"], 6)
        self.assertEqual(graph["coverage"]["diagnostic_files"], 4)
        self.assertEqual(graph["coverage"]["candidate_edges"], 4)
        self.assertEqual(graph["coverage"]["candidate_cross_file_observations"], 3)
        self.assertEqual(graph["coverage"]["same_file_candidate_observations"], 1)
        self.assertEqual(graph["coverage"]["unique_cross_file_boundaries"], 2)
        self.assertEqual(graph["coverage"]["hard_edges"], 0)
        self.assertEqual(len(graph["proposed_groups"]), 6)
        self.assertEqual(plan["confirmed_multi_file_components"], 0)
        self.assertEqual(graph["unresolved_observations"], [])
        consumer = next(n for n in graph["nodes"] if n["path"] == "consumer.rs")
        self.assertEqual(consumer["diagnostic_keys"], [])
        self.assertEqual(consumer["source_status"], "verified_current")
        group = next(g for g in graph["proposed_groups"] if g["files"] == ["consumer.rs"])
        self.assertEqual(len(group["boundary_edge_ids"]), 2)
        self.assertIn("impact direction", graph["coverage"]["edge_orientation"])
        raw = planner.read_json(self.f.root / "raw-usage.json")
        self.assertNotIn("consumer_function", json.loads(raw["content"][0]["text"])["usages"][0])
        self.assertIn("not an execution DAG", planner.markdown(plan))
        for edge in graph["edges"]:
            self.assertEqual(edge["origin_path"], origin)
            self.assertEqual(edge["evidence_refs"][0]["sha256"], planner.digest(self.f.root / "observations.json"))

    def test_missing_origin_and_forged_verified_symbols_are_unresolved_not_edges(self):
        observation = self.f.observation_fixture()[0]
        for change in (
            {"origin_binding_ref": None},
            {"identity_status": "verified_symbols"},
            {"endpoint_status": "path_only"},
        ):
            with self.subTest(change=change):
                original = copy.deepcopy(observation)
                observation.update(change)
                self.f.save()
                graph = self.f.plan()["work_graph"]
                self.assertEqual(graph["edges"], [])
                self.assertEqual(len(graph["unresolved_observations"]), 1)
                observation.clear()
                observation.update(original)

    def test_origin_proof_chain_rejects_forged_target_or_manifest_override(self):
        self.f.observation_fixture()
        original = planner.read_json(self.f.root / "origin-proof.json")
        for change in ({"target": "another::runtime::validate_runtime_uid"},
                       {"source_path": "consumer.rs"}, {"self_approval": True}):
            with self.subTest(change=change):
                self.f.write("origin-proof.json", {**original, **change})
                self.f.refresh_capture("origin-proof.json")
                self.f.save()
                self.assertEqual(self.f.plan()["work_graph"]["edges"], [])
        self.f.write("origin-proof.json", original)
        self.f.write("crates/apparatus-contracts/Cargo.toml",
                     '[package]\nname="apparatus-contracts"\n[lib]\nname="another"\n')
        original["crate_manifest_ref"] = self.f.ref("crates/apparatus-contracts/Cargo.toml")
        self.f.write("origin-proof.json", original)
        self.f.refresh_capture("origin-proof.json")
        self.f.save()
        self.assertEqual(self.f.plan()["work_graph"]["edges"], [])

    def test_origin_module_cfg_and_reexport_cannot_replace_default_chain(self):
        self.f.observation_fixture()
        for module in ('#[cfg(feature = "x")]\npub mod runtime;\n',
                       'pub use other::runtime;\n', '/* hidden */\npub mod runtime;\n'):
            with self.subTest(module=module):
                self.f.write("crates/apparatus-contracts/src/lib.rs", module)
                proof = planner.read_json(self.f.root / "origin-proof.json")
                proof["module_root_ref"] = self.f.ref("crates/apparatus-contracts/src/lib.rs")
                self.f.write("origin-proof.json", proof)
                self.f.refresh_capture("origin-proof.json")
                self.f.save()
                self.assertEqual(self.f.plan()["work_graph"]["edges"], [])

    def test_definition_coordinates_ambiguity_and_unsafe_paths_do_not_bind_origin(self):
        self.f.observation_fixture()
        raw = planner.read_json(self.f.root / "raw-def.json")
        text = raw["content"][0]["text"]
        actual_path = str(self.f.root / "crates/apparatus-contracts/src/runtime.rs")
        for forged in (text.replace(":1:8 ", ":1:7 "), text.replace("Found 1 ", "Found 2 "),
                       text.replace(actual_path, "\\\\server\\share\\runtime.rs"),
                       text.replace(actual_path, "\\\\?\\UNC\\server\\share\\runtime.rs"),
                       text.replace(actual_path, "\\\\.\\C:\\runtime.rs"),
                       text.replace(actual_path, "crates/../runtime.rs")):
            with self.subTest(forged=forged):
                raw["content"][0]["text"] = forged
                self.f.write("raw-def.json", raw)
                self.f.refresh_capture("raw-def.json")
                self.f.save()
                self.assertEqual(self.f.plan()["work_graph"]["edges"], [])
        raw["content"][0]["text"] = text.replace(actual_path, "\\\\?\\" + actual_path)
        self.f.write("raw-def.json", raw)
        self.f.refresh_capture("raw-def.json")
        self.f.save()
        self.assertEqual(self.f.plan()["work_graph"]["coverage"]["candidate_edges"], 1)

    def test_usage_query_target_and_raw_consumer_shape_are_not_repaired(self):
        self.f.observation_fixture(omitted_consumer=True)
        raw = planner.read_json(self.f.root / "raw-usage.json")
        baseline = json.loads(raw["content"][0]["text"])
        for bad in (None, "", False):
            with self.subTest(consumer_function=bad):
                data = copy.deepcopy(baseline)
                data["usages"][0]["consumer_function"] = bad
                raw["content"][0]["text"] = json.dumps(data)
                self.f.write("raw-usage.json", raw)
                self.f.refresh_capture("raw-usage.json")
                self.f.save()
                self.assertEqual(self.f.plan()["work_graph"]["edges"], [])
        raw["content"][0]["text"] = json.dumps({**baseline, "target": "another::same_spelling"})
        self.f.write("raw-usage.json", raw)
        self.f.refresh_capture("raw-usage.json")
        self.f.save()
        self.assertEqual(self.f.plan()["work_graph"]["edges"], [])

    def test_one_observation_cannot_reuse_another_rows_good_span(self):
        self.f.observation_fixture(("consumer.rs", "consumer.rs"))
        raw = planner.read_json(self.f.root / "raw-usage.json")
        data = json.loads(raw["content"][0]["text"])
        data["usages"][0].update(start=0, end=20)
        raw["content"][0]["text"] = json.dumps(data)
        self.f.write("raw-usage.json", raw)
        self.f.refresh_capture("raw-usage.json")
        self.f.save()
        graph = self.f.plan()["work_graph"]
        self.assertEqual(graph["coverage"]["candidate_edges"], 1)
        self.assertEqual(graph["edges"][0]["id"], "observation:observed-1")
        self.assertEqual(graph["unresolved_observations"][0]["id"], "observed-0")

    def test_observation_order_cardinality_and_raw_page_mismatch_are_not_false_edges(self):
        records = self.f.observation_fixture(("a.rs", "consumer.rs"))
        records.reverse()
        self.f.save()
        self.assertEqual(self.f.plan()["work_graph"]["edges"], [])
        records.reverse()
        records.pop()
        self.f.save()
        self.assertEqual(self.f.plan()["work_graph"]["edges"], [])
        # A gapped page must not be accepted merely because normalized totals say complete.
        sequence = planner.read_json(self.f.root / "usage-sequence.json")
        sequence["pages"][0]["arguments"]["offset"] = 1
        self.f.write("usage-sequence.json", sequence)
        self.f.refresh_capture("usage-sequence.json")
        self.f.save()
        self.assertEqual(self.f.plan()["work_graph"]["edges"], [])

    def test_endpoint_source_drift_and_forged_hash_remain_unresolved(self):
        observation = self.f.observation_fixture()[0]
        original = observation["destination"]["source_sha256"]
        observation["destination"]["source_sha256"] = "0" * 64
        self.f.save()
        self.assertEqual(self.f.plan()["work_graph"]["edges"], [])
        observation["destination"]["source_sha256"] = original
        self.f.save()
        self.f.write("consumer.rs", "// source changed after snapshot\n")
        graph = self.f.plan()["work_graph"]
        self.assertEqual(graph["edges"], [])
        self.assertTrue(graph["unresolved_observations"])
        self.assertFalse(any(n["path"] == "consumer.rs" for n in graph["nodes"]))

    def test_observation_bindings_missing_queries_paths_and_cycles_reject(self):
        self.f.observation_fixture()
        self.f.save()
        path = self.f.root / "observations.json"
        original = planner.read_json(path)
        for mutate in (
            lambda x: x.update(evidence_sha256="0" * 64),
            lambda x: x["observations"][0].update(query_ids=["missing"]),
            lambda x: x["observations"][0]["destination"].update(path="../escape.rs"),
            lambda x: x["observations"][0]["destination"].update(path="A.rs"),
        ):
            with self.subTest(mutate=mutate):
                data = copy.deepcopy(original)
                mutate(data)
                self.f.write("observations.json", data)
                with self.assertRaises(planner.Invalid):
                    self.f.plan()
        self.f.write("observations.json", original)
        snapshot = planner.read_json(self.f.snapshot_path)
        snapshot["inputs"].append({"role": "capture", **self.f.ref("observations.json")})
        self.f.write("snapshot.json", snapshot)
        self.f.evidence["snapshot_sha256"] = planner.digest(self.f.snapshot_path)
        self.f.write("evidence.json", self.f.evidence)
        # Constraints would also need rebinding, so omit that separate optional input.
        with self.assertRaisesRegex(planner.Invalid, "hash cycle"):
            planner.build_plan(self.f.inventory_path, self.f.evidence_path, self.f.snapshot_path,
                               observations_path=path)

    def test_observation_output_collision_and_change_before_publication_preserve_old_output(self):
        self.f.observation_fixture()
        self.f.save()
        observed = self.f.root / "observations.json"
        output = self.f.root / "plan.json"
        args = ["--inventory", str(self.f.inventory_path), "--evidence", str(self.f.evidence_path),
                "--snapshot", str(self.f.snapshot_path), "--observations", str(observed),
                "--output", str(output)]
        self.assertEqual(planner.main(args), 0)
        before = output.read_bytes()
        original = observed.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            planner.main(args[:-1] + [str(observed)])
        self.assertEqual(observed.read_bytes(), original)
        real_fsync = planner.os.fsync

        def mutate_after_stage(fd):
            real_fsync(fd)
            observed.write_bytes(original + b"\n")

        with mock.patch.object(planner.os, "fsync", side_effect=mutate_after_stage):
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                planner.main(args)
        self.assertEqual(output.read_bytes(), before)
        self.assertFalse(list(self.f.root.glob(".plan*.tmp")))

    def test_observation_graph_json_markdown_are_deterministic(self):
        self.f.observation_fixture(("a.rs", "consumer.rs"))
        self.f.save()
        first, second = self.f.plan(), self.f.plan()
        self.assertEqual(json.dumps(first, sort_keys=True), json.dumps(second, sort_keys=True))
        self.assertEqual(planner.markdown(first), planner.markdown(second))
        self.assertIn("Boundary review tasks", planner.markdown(first))
        self.assertIn("Tentative ownership groups", planner.markdown(first))
        self.assertLess(planner.markdown(first).index("Work partition overview"),
                        planner.markdown(first).index("## Components"))


if __name__ == "__main__":
    unittest.main()
