"""Discriminants for the shared, bounded Rust source-range validator."""

from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from clippy_source_ranges import verify_function_range


def fixture(source, name="f", start=1, end=None, parent=None, kind="Function"):
    end = end or len(source.splitlines())
    symbol = {"qualified_name": "serena::file.rs::" + name, "kind": kind, "path": "file.rs",
              "range": {"start_line": start, "start_column": 0, "end_line": end + 1,
                        "end_column": 0, "column_unit": "unicode_scalar"}}
    witnesses = [{"query_id": "q1", "name_path": name, "kind": kind, "relative_path": "file.rs",
                  "body_location": {"start_line": start - 1, "end_line": end - 1},
                  "parent_name_path": parent}]
    return symbol, witnesses


class RangeTests(unittest.TestCase):
    def test_contract_lint_attributes_are_non_rewriting_only_after_complete_parse(self):
        for attribute in ("#![allow(missing_docs)]", "#[warn(clippy::doc_markdown, rustdoc::broken_intra_doc_links,)]",
                          "#[deny(dead_code)]", "#[forbid(unsafe_code)]"):
            source = attribute + "\nfn f() {\n}\n"
            symbol, witnesses = fixture(source, start=2 if attribute.startswith("#!") else 1)
            with self.subTest(attribute=attribute):
                self.assertEqual("verified", verify_function_range(
                    source, symbol, [2, 0, 2, 8], "unicode_scalar", witnesses)["status"])
        for attribute in ("#[allow()]", "#[allow(dead_code, reason = \"why\")]",
                          "#[allow(other::lint)]", "#[allow(/*comment*/dead_code)]",
                          "#[allow(r#dead_code)]", "#[expect(dead_code)]",
                          "#[allow(dead_code(extra))]", "#[allow(dead_code), cfg(x)]",
                          "#[cfg_attr(x, allow(dead_code))]", "#[rewrite]",
                          "#[doc = include_str!(\"generated.md\")]"):
            source = attribute + "\nfn f() {\n}\n"
            symbol, witnesses = fixture(source)
            with self.subTest(attribute=attribute):
                self.assertNotEqual("verified", verify_function_range(
                    source, symbol, [2, 0, 2, 8], "unicode_scalar", witnesses)["status"])
        source = '#![allow(missing_docs)]\nfn f() {\n    #![cfg(feature = "x")]\n}\n'
        symbol, witnesses = fixture(source, start=2)
        self.assertEqual("unsupported", verify_function_range(
            source, symbol, [2, 0, 2, 8], "unicode_scalar", witnesses)["status"])

    def test_review_c1_foreign_bodyless_item_excludes_outer_but_not_header(self):
        source = 'fn f() {\n    unsafe extern "C" {\n        fn foreign(x: i32);\n    }\n}\n'
        symbol, witnesses = fixture(source)
        self.assertNotEqual("verified", verify_function_range(
            source, symbol, [3, 8, 3, 25], "unicode_scalar", witnesses)["status"])
        self.assertEqual("verified", verify_function_range(
            source, symbol, [1, 0, 1, 8], "unicode_scalar", witnesses)["status"])
        foreign, foreign_witness = fixture(source, name="foreign", start=3, end=3, parent="f")
        self.assertNotEqual("verified", verify_function_range(
            source, foreign, [3, 8, 3, 25], "unicode_scalar", foreign_witness)["status"])

    def test_review_c2_selected_inner_cfg_and_unrelated_scope_controls(self):
        source = 'fn f() {\n    #![cfg(feature = "x")]\n    1;\n}\n'
        symbol, witnesses = fixture(source)
        self.assertEqual("unsupported", verify_function_range(
            source, symbol, [1, 0, 1, 8], "unicode_scalar", witnesses)["status"])
        unrelated = ('mod other {\n    #![cfg(feature = "x")]\n}\n'
                     'fn f() {\n    { #![allow(dead_code)] let x = 1; }\n}\n')
        symbol, witnesses = fixture(unrelated, start=4, end=6)
        self.assertEqual("verified", verify_function_range(
            unrelated, symbol, [4, 0, 4, 8], "unicode_scalar", witnesses)["status"])

    def test_review_c3_ordered_and_complete_parent_chain(self):
        trait = "impl Trait for Foo {\n    fn f(&self) {\n    }\n}\n"
        symbol, witnesses = fixture(trait, name="impl Foo for Trait/f", start=2, end=3,
                                    parent="impl Foo for Trait", kind="Method")
        witnesses.append({"query_id": "q2", "name_path": "impl Foo for Trait", "kind": "Object",
                          "relative_path": "file.rs", "parent_name_path": None,
                          "body_location": {"start_line": 0, "end_line": 3}})
        with self.subTest(variant="reversed_trait_target"):
            self.assertNotEqual("verified", verify_function_range(
                trait, symbol, [2, 4, 2, 15], "unicode_scalar", witnesses)["status"])
        symbol["qualified_name"] = "serena::file.rs::impl Trait for Foo/f"
        witnesses[0].update(name_path="impl Trait for Foo/f", parent_name_path="impl Trait for Foo")
        witnesses[1]["name_path"] = "impl Trait for Foo"
        self.assertEqual("verified", verify_function_range(
            trait, symbol, [2, 4, 2, 15], "unicode_scalar", witnesses)["status"])
        top = "fn f() {\n}\n"
        symbol, witnesses = fixture(top, name="Wrong/f", parent="Wrong")
        with self.subTest(variant="imaginary_top_level_parent"):
            self.assertNotEqual("verified", verify_function_range(
                top, symbol, [1, 0, 1, 8], "unicode_scalar", witnesses)["status"])
        nested = "mod actual {\n    impl A {\n        fn f(&self) {\n        }\n    }\n}\n"
        symbol, witnesses = fixture(nested, name="wrong/impl A/f", start=3, end=4,
                                    parent="wrong/impl A", kind="Method")
        witnesses.append({"query_id": "q2", "name_path": "wrong/impl A", "kind": "Object",
                          "relative_path": "file.rs", "parent_name_path": "wrong",
                          "body_location": {"start_line": 1, "end_line": 4}})
        with self.subTest(variant="wrong_module_ancestor"):
            self.assertNotEqual("verified", verify_function_range(
                nested, symbol, [3, 8, 3, 19], "unicode_scalar", witnesses)["status"])
        witnesses.append({"query_id": "q3", "name_path": "wrong", "kind": "Module",
                          "relative_path": "file.rs", "parent_name_path": None,
                          "body_location": {"start_line": 0, "end_line": 5}})
        with self.subTest(variant="forged_module_name_at_matching_range"):
            self.assertNotEqual("verified", verify_function_range(
                nested, symbol, [3, 8, 3, 19], "unicode_scalar", witnesses)["status"])
        witnesses.pop()
        symbol["qualified_name"] = "serena::file.rs::actual/impl A/f"
        witnesses[0].update(name_path="actual/impl A/f", parent_name_path="actual/impl A")
        witnesses[1].update(name_path="actual/impl A", parent_name_path="actual")
        witnesses.append({"query_id": "q3", "name_path": "actual", "kind": "Module",
                          "relative_path": "file.rs", "parent_name_path": None,
                          "body_location": {"start_line": 0, "end_line": 5}})
        self.assertEqual("verified", verify_function_range(
            nested, symbol, [3, 8, 3, 19], "unicode_scalar", witnesses)["status"])

    def test_inner_attribute_scope_ancestry_and_nested_controls(self):
        source = 'mod actual {\n    #![cfg(feature = "x")]\n    fn f() {\n    }\n}\n'
        symbol, witnesses = fixture(source, name="actual/f", start=3, end=4, parent="actual")
        witnesses.append({"query_id": "q2", "name_path": "actual", "kind": "Module",
                          "relative_path": "file.rs", "parent_name_path": None,
                          "body_location": {"start_line": 0, "end_line": 4}})
        self.assertEqual("unsupported", verify_function_range(
            source, symbol, [3, 4, 3, 12], "unicode_scalar", witnesses)["status"])
        safe = source.replace('cfg(feature = "x")', "allow(missing_docs)")
        self.assertEqual("verified", verify_function_range(
            safe, symbol, [3, 4, 3, 12], "unicode_scalar", witnesses)["status"])
        nested = ('fn f() {\n    mod other {\n        #![cfg(feature = "x")]\n    }\n'
                  '    fn inner() {\n        #![cfg(feature = "x")]\n    }\n}\n')
        symbol, witnesses = fixture(nested)
        self.assertEqual("verified", verify_function_range(
            nested, symbol, [1, 0, 1, 8], "unicode_scalar", witnesses)["status"])

    def test_mixed_imports_strings_closures_and_opaque_macros_header(self):
        source = ('use dep::Thing;\nstruct Unrelated;\npub fn f() {\n'
                  '    let s = r###"} fn fake() { /*"###;\n'
                  '    let c = || { "{" };\n    assert!(format!("{}", s).len() > 0);\n}\n')
        symbol, witnesses = fixture(source, start=3)
        result = verify_function_range(source, symbol, [3, 0, 3, 12], "unicode_scalar", witnesses)
        self.assertEqual("verified", result["status"], result["reasons"])
        self.assertFalse(result["checked_predicates"]["source_binding_verified"])
        self.assertFalse(result["checked_predicates"]["source_hash_stable_before_after"])

    def test_macro_and_closure_interiors_not_outer_item(self):
        source = 'fn f() {\n    assert!({ fn fake() {} });\n    let c = || { 1 };\n}\n'
        symbol, witnesses = fixture(source)
        for location in ([2, 15, 2, 25], [3, 17, 3, 18]):
            result = verify_function_range(source, symbol, location, "unicode_scalar", witnesses)
            self.assertNotEqual("verified", result["status"])

    def test_nested_source_item_is_not_outer(self):
        source = "fn f() {\n    fn inner() {\n        1;\n    }\n}\n"
        symbol, witnesses = fixture(source)
        result = verify_function_range(source, symbol, [2, 4, 2, 12], "unicode_scalar", witnesses)
        self.assertNotEqual("verified", result["status"])

    def test_nested_comments_bytes_chars_lifetimes_and_generics(self):
        source = ("fn f<'a, T: Fn() -> u32>(x: &'a T) {\n"
                  "    /* } /* fn fake() {} */ ) */\n"
                  "    let c = b'}'; let d = '\\u{7d}'; let s = b\"\\\"}\";\n"
                  "    let r = br##\"}] fn fake() {\"##;\n}\n")
        symbol, witnesses = fixture(source)
        result = verify_function_range(source, symbol, [1, 0, 1, 35], "unicode_scalar", witnesses)
        self.assertEqual("verified", result["status"], result["reasons"])

    def test_cfg_attribute_affects_selected_only(self):
        source = "#[derive(Debug)]\nstruct Other;\n#[cfg(feature = \"x\")]\nfn f() {\n}\n"
        symbol, witnesses = fixture(source, start=3)
        result = verify_function_range(source, symbol, [4, 0, 4, 8], "unicode_scalar", witnesses)
        self.assertEqual("unsupported", result["status"])

    def test_unterminated_literal_and_wrong_native_end_rejected(self):
        for source in ('fn f() {\n    let s = "unterminated;\n}\n',
                       'fn f() {\n    let s = r##"unterminated"#;\n}\n',
                       'fn f() {\n    /* unterminated\n}\n'):
            symbol, witnesses = fixture(source)
            self.assertNotEqual("verified", verify_function_range(
                source, symbol, [1, 0, 1, 8], "unicode_scalar", witnesses)["status"])
        source = "fn f() {\n    1;\n}\n"
        symbol, witnesses = fixture(source)
        witnesses[0]["body_location"]["end_line"] = 1
        self.assertNotEqual("verified", verify_function_range(
            source, symbol, [1, 0, 1, 8], "unicode_scalar", witnesses)["status"])

    def test_crlf_and_unicode_columns(self):
        source = "/// é😀\r\nfn f() {\r\n    1;\r\n}\r\n"
        symbol, witnesses = fixture(source)
        for unit, end in (("unicode_scalar", 6), ("utf8_byte", 10), ("utf16", 7)):
            self.assertEqual("verified", verify_function_range(
                source, symbol, [1, 4, 1, end], unit, witnesses)["status"])
        self.assertEqual("unsupported", verify_function_range(
            source, symbol, [1, 4, 1, 7], "utf8_byte", witnesses)["status"])

    def test_identical_method_names_require_source_bound_parent(self):
        source = ("impl A {\n    fn f(&self) {\n    }\n}\n"
                  "impl B {\n    fn f(&self) {\n    }\n}\n")
        symbol, witnesses = fixture(source, name="B/f", start=6, end=7, parent="B", kind="Method")
        result = verify_function_range(source, symbol, [6, 4, 6, 15], "unicode_scalar", witnesses)
        self.assertNotEqual("verified", result["status"])
        witnesses.append({"query_id": "q2", "name_path": "B", "kind": "Object",
                          "relative_path": "file.rs", "parent_name_path": None,
                          "body_location": {"start_line": 4, "end_line": 7}})
        self.assertEqual("verified", verify_function_range(
            source, symbol, [6, 4, 6, 15], "unicode_scalar", witnesses)["status"])
        witnesses[-1]["body_location"] = {"start_line": 0, "end_line": 3}
        self.assertNotEqual("verified", verify_function_range(
            source, symbol, [6, 4, 6, 15], "unicode_scalar", witnesses)["status"])

    def test_real_first_gate_source(self):
        root = Path(__file__).resolve().parents[2]
        path = "services/Manifesto/tests/common/migration_roundtrip.rs"
        source = (root / path).read_bytes().decode("utf-8")
        symbol, witnesses = fixture(source, name="assert_initial_schema_roundtrip", start=5, end=61)
        symbol["path"] = path
        symbol["qualified_name"] = "serena::" + path + "::assert_initial_schema_roundtrip"
        witnesses[0]["relative_path"] = path
        result = verify_function_range(source, symbol, [5, 0, 5, 95], "unicode_scalar", witnesses)
        self.assertEqual("verified", result["status"], result["reasons"])

    def test_c_literals_suffixes_and_raw_lifetimes_are_opaque(self):
        source = ("fn f<'r#valid>(x: &'r#valid str) {\n"
                  '    let a = c"} fn fake() {";\n'
                  '    let b = cr###"}"## fn fake() {"###;\n'
                  "    let c = '}'suffix;\n}\n")
        symbol, witnesses = fixture(source)
        result = verify_function_range(source, symbol, [1, 0, 1, 30], "unicode_scalar", witnesses)
        self.assertEqual("verified", result["status"], result["reasons"])

    def test_nested_docs_and_expression_closure_do_not_map_to_outer(self):
        source = "fn f() {\n    /// Nested.\n    fn inner() {}\n    let g = |x, y| x + y;\n}\n"
        symbol, witnesses = fixture(source)
        for span in ([2, 8, 2, 12], [4, 21, 4, 26]):
            self.assertNotEqual("verified", verify_function_range(
                source, symbol, span, "unicode_scalar", witnesses)["status"])
        self.assertEqual("verified", verify_function_range(
            source, symbol, [1, 0, 1, 8], "unicode_scalar", witnesses)["status"])

    def test_opaque_fake_closure_does_not_poison_later_source(self):
        source = "fn f() {\n    opaque!(|| fake);\n    ordinary();\n}\n"
        symbol, witnesses = fixture(source)
        self.assertEqual("verified", verify_function_range(
            source, symbol, [3, 4, 3, 12], "unicode_scalar", witnesses)["status"])

    def test_inner_cfg_and_parent_attributes_not_ignored(self):
        source = '#![cfg(feature = "x")]\nuse dep::Thing;\nfn f() {\n}\n'
        symbol, witnesses = fixture(source, start=3)
        self.assertEqual("unsupported", verify_function_range(
            source, symbol, [3, 0, 3, 8], "unicode_scalar", witnesses)["status"])

    def test_witness_name_and_duplicate_query_correspondence(self):
        source = "fn f() {\n    1;\n}\n"
        symbol, witnesses = fixture(source)
        witnesses.append(dict(witnesses[0], query_id="q2"))
        self.assertEqual("verified", verify_function_range(
            source, symbol, [1, 0, 1, 8], "unicode_scalar", witnesses)["status"])
        symbol["qualified_name"] = "serena::file.rs::Wrong/f"
        self.assertNotEqual("verified", verify_function_range(
            source, symbol, [1, 0, 1, 8], "unicode_scalar", witnesses)["status"])

    def test_unrelated_late_lexical_failure_does_not_poison_selected(self):
        source = 'fn f() {\n    1;\n}\nfn broken() { "unterminated\n'
        symbol, witnesses = fixture(source, end=3)
        self.assertEqual("verified", verify_function_range(
            source, symbol, [1, 0, 1, 8], "unicode_scalar", witnesses)["status"])

    def test_generated_header_rejected_but_unrelated_type_doc_not_global(self):
        source = "// @generated\nuse dep::Thing;\nfn f() {\n}\n"
        symbol, witnesses = fixture(source, start=3)
        self.assertEqual("unsupported", verify_function_range(
            source, symbol, [3, 0, 3, 8], "unicode_scalar", witnesses)["status"])
        source = "use dep::Thing;\n/// @generated\nstruct Other;\nfn f() {\n}\n"
        symbol, witnesses = fixture(source, start=4)
        self.assertEqual("verified", verify_function_range(
            source, symbol, [4, 0, 4, 8], "unicode_scalar", witnesses)["status"])

    def test_async_qualifier_is_not_an_expression_closure(self):
        source = "pub async fn f() {\n    1;\n}\nuse dep::Thing;\n"
        symbol, witnesses = fixture(source, end=3)
        self.assertEqual("verified", verify_function_range(
            source, symbol, [1, 0, 1, 18], "unicode_scalar", witnesses)["status"])

    def test_generic_trait_impl_positive_parent_and_wrong_identity(self):
        source = ("use dep::Trait;\nimpl<'a, T> Trait for Foo<'a, T> {\n"
                  "    pub fn f(&self, x: &'a str) {\n        let s = r#\"{\"#;\n    }\n}\n")
        symbol, witnesses = fixture(source, name="impl Trait for Foo/f", start=3, end=5,
                                    parent="impl Trait for Foo", kind="Method")
        witnesses.append({"query_id": "q2", "name_path": "impl Trait for Foo", "kind": "Object",
                          "relative_path": "file.rs", "parent_name_path": None,
                          "body_location": {"start_line": 1, "end_line": 5}})
        result = verify_function_range(source, symbol, [3, 4, 3, 15], "unicode_scalar", witnesses)
        self.assertEqual("verified", result["status"], result["reasons"])
        witnesses[0]["parent_name_path"] = "impl Wrong for Foo"
        witnesses[1]["name_path"] = "impl Wrong for Foo"
        self.assertNotEqual("verified", verify_function_range(
            source, symbol, [3, 4, 3, 15], "unicode_scalar", witnesses)["status"])


if __name__ == "__main__":
    unittest.main()
