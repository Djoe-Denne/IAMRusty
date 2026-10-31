"""Synthetic tests only. Neither comparison input is imported or executed."""

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import unittest


FILE = Path(__file__).with_name('function.py')
SPEC = importlib.util.spec_from_file_location('candidate_ast_diff', FILE)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
compare = MODULE.compare_python_ast


class ComparisonTests(unittest.TestCase):
    def test_format_and_comments_are_ignored(self):
        result = compare('def f():\n    return 1\n', '# comment\n\ndef f( ):\n return 1\n')
        self.assertTrue(result['moduleAstEqual'])
        self.assertEqual(result['counts']['unchanged'], 1)
        self.assertEqual(result['items']['unchanged'][0]['after'][0]['line'], 3)

    def test_changes_additions_and_removals(self):
        result = compare('def f(): return 1\ndef gone(): pass\n',
                         'def f(): return 2\ndef added(): pass\n')
        self.assertEqual(result['counts']['changed'], 1)
        self.assertEqual(result['items']['removed'][0]['name'], 'gone')
        self.assertEqual(result['items']['added'][0]['name'], 'added')

    def test_class_method_async_branch_and_nested_function(self):
        source = ('class C:\n async def method(self):\n'
                  '  def inner(): pass\n'
                  'if True:\n def conditional(): pass\n')
        result = compare(source, source)
        names = {row['name'] for row in result['items']['unchanged']}
        self.assertEqual(names, {'C', 'C.method', 'C.method.inner', 'conditional'})
        method = next(x for x in result['items']['unchanged'] if x['name'] == 'C.method')
        self.assertEqual(method['before'][0]['kind'], 'async-function')

    def test_duplicate_definitions_are_not_silently_overwritten(self):
        before = 'if True:\n def f(): return 1\nelse:\n def f(): return 2\n'
        result = compare(before, 'def f(): return 1\n')
        self.assertEqual(result['counts']['ambiguous'], 1)
        self.assertEqual(len(result['items']['ambiguous'][0]['before']), 2)
        self.assertEqual(result['counts']['unchanged'], 0)

    def test_property_getter_and_setter_are_ambiguous(self):
        source = ('class C:\n @property\n def value(self): return 1\n'
                  ' @value.setter\n def value(self, v): pass\n')
        result = compare(source, source)
        self.assertTrue(result['moduleAstEqual'])
        self.assertEqual(result['items']['ambiguous'][0]['name'], 'C.value')

    def test_duplicate_previews_are_bounded_without_losing_counts(self):
        result = compare('def f(): pass\n' * 12, '')
        row = result['items']['ambiguous'][0]
        self.assertEqual(row['beforeCount'], 12)
        self.assertEqual(len(row['before']), 5)
        self.assertTrue(row['beforeTruncated'])
        self.assertTrue(result['truncated'])

    def test_definitions_in_try_and_match_are_visited(self):
        source = ('try:\n def tried(): pass\nexcept Exception:\n def handled(): pass\n'
                  'match 1:\n case 1:\n  class Matched: pass\n')
        result = compare(source, source)
        self.assertEqual({r['name'] for r in result['items']['unchanged']},
                         {'tried', 'handled', 'Matched'})

    def test_changed_definition_kind_is_reported(self):
        result = compare('def f(): pass\n', 'class f: pass\n')
        self.assertEqual(result['counts']['changed'], 1)

    def test_module_changes_are_visible_without_symbol_changes(self):
        result = compare('VALUE = 1\n', 'VALUE = 2\n')
        self.assertFalse(result['moduleAstEqual'])
        self.assertEqual(sum(result['counts'].values()), 0)

    def test_annotations_decorators_defaults_and_type_comments(self):
        for before, after in [
            ('def f(x: int): pass', 'def f(x: str): pass'),
            ('@a\ndef f(): pass', '@b\ndef f(): pass'),
            ('def f(x=1): pass', 'def f(x=2): pass'),
            ('x = 1 # type: int', 'x = 1 # type: str'),
        ]:
            with self.subTest(before=before):
                self.assertFalse(compare(before, after)['moduleAstEqual'])

    def test_sources_are_not_executed(self):
        result = compare('raise RuntimeError("must not execute")\n',
                         'import module_that_does_not_exist\n')
        self.assertFalse(result['moduleAstEqual'])

    def test_output_limit_keeps_full_counts_and_prioritizes_differences(self):
        result = compare('def a(): pass\ndef z(): return 1\n',
                         'def a(): pass\ndef z(): return 2\n', max_output_items=1)
        self.assertTrue(result['truncated'])
        self.assertEqual(result['counts']['unchanged'], 1)
        self.assertEqual(result['omittedCounts']['unchanged'], 1)
        self.assertEqual(result['items']['changed'][0]['name'], 'z')
        self.assertEqual(sum(len(x) for x in result['items'].values()), 1)

    def test_invalid_sources_use_stable_errors(self):
        for source, code in [(None, 'INVALID_SOURCE'), ('def :', 'INVALID_PYTHON'),
                             ('\ud800', 'INVALID_SOURCE_ENCODING'), ('\0', 'INVALID_PYTHON')]:
            with self.subTest(code=code), self.assertRaisesRegex(MODULE.ComparisonError, '^' + code + '$'):
                compare(source, '')

    def test_limits_and_boolean_options(self):
        for options, code in [
            ({'max_source_bytes': 1}, 'SOURCE_TOO_LARGE'),
            ({'max_ast_nodes': 1}, 'AST_NODE_LIMIT'),
            ({'max_symbols': 1}, 'SYMBOL_LIMIT'),
            ({'max_output_items': True}, 'INVALID_OPTIONS'),
            ({'max_output_items': 1001}, 'INVALID_OPTIONS'),
            ({'max_ast_nodes': 0}, 'INVALID_OPTIONS'),
        ]:
            with self.subTest(options=options), self.assertRaisesRegex(MODULE.ComparisonError, '^' + code + '$'):
                compare('def f(): pass\ndef g(): pass\n', '', **options)

    def test_utf8_byte_limit(self):
        with self.assertRaisesRegex(MODULE.ComparisonError, '^SOURCE_TOO_LARGE$'):
            compare('# éé', '', max_source_bytes=5)


class CliTests(unittest.TestCase):
    def run_cli(self, payload):
        process = subprocess.run([sys.executable, '-B', str(FILE)], input=payload,
                                 capture_output=True, timeout=10)
        self.assertEqual(process.stderr, b'')
        return process.returncode, json.loads(process.stdout)

    def test_json_stdin_success(self):
        code, result = self.run_cli(json.dumps({'before': 'def f(): pass', 'after': 'def f(): pass'}).encode())
        self.assertEqual(code, 0)
        self.assertTrue(result['ok'])
        self.assertTrue(result['moduleAstEqual'])

    def test_cli_rejects_invalid_requests_without_echoing_source(self):
        cases = [b'\xff', b'{', b'[]', b'{}',
                 b'{"before":"", "after":"", "extra":true}',
                 b'{"before":"", "after":"", "options":{"unknown":1}}',
                 b'{"before":"private def :", "after":""}']
        for raw in cases:
            with self.subTest(raw=raw):
                code, result = self.run_cli(raw)
                self.assertEqual(code, 1)
                self.assertFalse(result['ok'])
                self.assertNotIn('private', json.dumps(result))


if __name__ == '__main__':
    unittest.main()
