"""Candidate: compare Python ASTs without importing or executing either source."""

import ast
import hashlib
import json
import sys


class ComparisonError(ValueError):
    """A stable error code, never source text or parser diagnostics."""


def _limit(value, maximum):
    if type(value) is not int or not 1 <= value <= maximum:
        raise ComparisonError('INVALID_OPTIONS')
    return value


def _parse(source, max_source_bytes, max_ast_nodes):
    if not isinstance(source, str):
        raise ComparisonError('INVALID_SOURCE')
    if len(source) > max_source_bytes:
        raise ComparisonError('SOURCE_TOO_LARGE')
    try:
        size = len(source.encode('utf-8'))
    except UnicodeEncodeError:
        raise ComparisonError('INVALID_SOURCE_ENCODING') from None
    if size > max_source_bytes:
        raise ComparisonError('SOURCE_TOO_LARGE')
    try:
        tree = ast.parse(source, mode='exec', type_comments=True)
    except (SyntaxError, ValueError):
        raise ComparisonError('INVALID_PYTHON') from None
    except (MemoryError, RecursionError):
        raise ComparisonError('AST_RESOURCE_LIMIT') from None
    count = 0
    for _ in ast.walk(tree):
        count += 1
        if count > max_ast_nodes:
            raise ComparisonError('AST_NODE_LIMIT')
    return tree


def _fingerprint(node):
    try:
        text = ast.dump(node, annotate_fields=True, include_attributes=False)
    except (RecursionError, MemoryError):
        raise ComparisonError('AST_RESOURCE_LIMIT') from None
    return hashlib.sha256(text.encode('utf-8')).hexdigest()


def _definitions(tree, max_symbols):
    groups = {}
    count = 0
    stack = [(tree, ())]
    kinds = {ast.FunctionDef: 'function', ast.AsyncFunctionDef: 'async-function',
             ast.ClassDef: 'class'}
    while stack:
        node, scope = stack.pop()
        if type(node) in kinds:
            count += 1
            if count > max_symbols:
                raise ComparisonError('SYMBOL_LIMIT')
            scope = (*scope, node.name)
            name = '.'.join(scope)
            groups.setdefault(name, []).append({
                'kind': kinds[type(node)], 'line': node.lineno,
                'astSha256': _fingerprint(node),
            })
        # Traverse branches, nested functions and classes, preserving source order.
        stack.extend((child, scope) for child in reversed(list(ast.iter_child_nodes(node))))
    return groups, count


def compare_python_ast(before, after, *, max_source_bytes=1048576,
                       max_ast_nodes=100000, max_symbols=10000,
                       max_output_items=100):
    """Return structural differences only; duplicates are explicitly ambiguous.

    Inputs are source strings already authorized by the caller. No file, Git,
    subprocess, import of source code, or semantic-equivalence check is performed.
    The output cap applies to all categories combined, without truncating counts.
    """
    max_source_bytes = _limit(max_source_bytes, 4194304)
    max_ast_nodes = _limit(max_ast_nodes, 500000)
    max_symbols = _limit(max_symbols, 50000)
    remaining = _limit(max_output_items, 1000)
    left_tree = _parse(before, max_source_bytes, max_ast_nodes)
    right_tree = _parse(after, max_source_bytes, max_ast_nodes)
    left, left_count = _definitions(left_tree, max_symbols)
    right, right_count = _definitions(right_tree, max_symbols)
    categories = ('ambiguous', 'removed', 'added', 'changed', 'unchanged')
    rows = {key: [] for key in categories}
    for name in sorted(left.keys() | right.keys()):
        old, new = left.get(name, []), right.get(name, [])
        if len(old) > 1 or len(new) > 1:
            kind = 'ambiguous'
        elif not old:
            kind = 'added'
        elif not new:
            kind = 'removed'
        elif old[0]['astSha256'] == new[0]['astSha256']:
            kind = 'unchanged'
        else:
            kind = 'changed'
        rows[kind].append({
            'name': name, 'before': old[:5], 'after': new[:5],
            'beforeCount': len(old), 'afterCount': len(new),
            'beforeTruncated': len(old) > 5, 'afterTruncated': len(new) > 5,
        })
    counts = {key: len(rows[key]) for key in categories}
    output, omitted = {}, {}
    # Prioritize ambiguity and differences over unchanged definitions.
    for key in categories:
        output[key] = rows[key][:remaining]
        remaining -= len(output[key])
        omitted[key] = counts[key] - len(output[key])
    return {
        'schemaVersion': 1,
        'comparisonKind': 'python-ast-structure-only',
        'moduleAstEqual': _fingerprint(left_tree) == _fingerprint(right_tree),
        'definitionCounts': {'before': left_count, 'after': right_count},
        'counts': counts, 'items': output,
        'truncated': any(omitted.values()) or any(
            row['beforeTruncated'] or row['afterTruncated']
            for category in output.values() for row in category),
        'omittedCounts': omitted,
    }


def main():
    """Read one bounded UTF-8 JSON request from stdin; write one JSON response."""
    try:
        raw = sys.stdin.buffer.read(16777217)
        if len(raw) > 16777216:
            raise ComparisonError('REQUEST_TOO_LARGE')
        try:
            request = json.loads(raw.decode('utf-8'))
        except (UnicodeDecodeError, json.JSONDecodeError, RecursionError):
            raise ComparisonError('INVALID_JSON_OR_UTF8') from None
        if (not isinstance(request, dict) or set(request) - {'before', 'after', 'options'}
                or not {'before', 'after'} <= request.keys()):
            raise ComparisonError('INVALID_REQUEST')
        options = request.get('options', {})
        allowed = {'max_source_bytes', 'max_ast_nodes', 'max_symbols', 'max_output_items'}
        if not isinstance(options, dict) or options.keys() - allowed:
            raise ComparisonError('INVALID_OPTIONS')
        result = compare_python_ast(request['before'], request['after'], **options)
    except ComparisonError as error:
        print(json.dumps({'ok': False, 'error': {'code': str(error)}}))
        return 1
    print(json.dumps({'ok': True, **result}, ensure_ascii=True))
    return 0


if __name__ == '__main__':
    sys.exit(main())
