"""Bounded source-local Rust function localization; no semantic or closure proof.

Offsets always address the original decoded source, including CRLF and Unicode.
Comments and literals are scanned states, never regex-masked source rewrites.
"""

from collections import namedtuple
import re

PREDICATES = (
    "source_binding_verified", "source_hash_stable_before_after",
    "diagnostic_coordinates_verified", "symbol_endpoints_source_verified",
    "diagnostic_fully_enclosed", "relevant_local_candidates_exhausted",
    "nested_or_competing_symbols_excluded",
)
Token = namedtuple("Token", "text start end kind")
FUNCTION_KINDS = {"Function", "Method", "function", "method", "fn", 6, 12}


class Unsupported(ValueError):
    pass


def _lines(source):
    parts = source.split("\n")
    return [line + "\n" for line in parts[:-1]] + ([parts[-1]] if parts[-1] else [])


def _position(source, line, column, unit):
    lines = _lines(source)
    if type(line) is not int or type(column) is not int or line < 1 or column < 0:
        raise Unsupported("Invalid coordinates")
    if line == len(lines) + 1 and column == 0 and (not source or source.endswith("\n")):
        return len(source)
    if line > len(lines):
        raise Unsupported("Coordinate outside source")
    content = lines[line - 1].rstrip("\r\n")
    if unit == "unicode_scalar":
        if column > len(content):
            raise Unsupported("Column outside source")
        count = column
    elif unit in ("utf8_byte", "utf16"):
        encoding = "utf-8" if unit == "utf8_byte" else "utf-16-le"
        width = column * (2 if unit == "utf16" else 1)
        data = content.encode(encoding)
        if width > len(data):
            raise Unsupported("Column outside source")
        try:
            count = len(data[:width].decode(encoding))
        except UnicodeDecodeError:
            raise Unsupported("Column splits a Unicode character") from None
    else:
        raise Unsupported("Unknown column unit")
    return sum(map(len, lines[:line - 1])) + count


def _range(source, value):
    return (_position(source, value["start_line"], value["start_column"], value["column_unit"]),
            _position(source, value["end_line"], value["end_column"], value["column_unit"]))


def _native_range(source, location):
    lines = _lines(source)
    a, b = location.get("start_line"), location.get("end_line")
    if type(a) is not int or type(b) is not int or not 0 <= a <= b < len(lines):
        raise Unsupported("Invalid native inclusive body lines")
    return {"start_line": a + 1, "start_column": 0,
            "end_line": b + 2 if lines[b].endswith("\n") else b + 1,
            "end_column": 0 if lines[b].endswith("\n") else len(lines[b]),
            "column_unit": "unicode_scalar"}


def _identifier_start(char):
    return bool(char) and (char == "_" or char.isalpha())


def _identifier_end(source, index):
    while index < len(source) and (source[index] == "_" or source[index].isalnum()):
        index += 1
    return index


def _suffix(source, index):
    return _identifier_end(source, index + 1) if _identifier_start(source[index:index + 1]) else index


def _escape(source, index):
    """End of one bounded Rust escape (index addresses the backslash)."""
    if index + 1 >= len(source):
        raise Unsupported("Unterminated escape")
    code = source[index + 1]
    if code in "nrt\\0'\"":
        return index + 2
    if code == "x" and re.fullmatch(r"[0-9a-fA-F]{2}", source[index + 2:index + 4]):
        return index + 4
    if code == "u" and source[index + 2:index + 3] == "{":
        end = source.find("}", index + 3)
        digits = source[index + 3:end].replace("_", "")
        if end != -1 and 1 <= len(digits) <= 6 and all(c in "0123456789abcdefABCDEF" for c in digits):
            scalar = int(digits, 16)
            if scalar <= 0x10ffff and not 0xd800 <= scalar <= 0xdfff:
                return end + 1
    raise Unsupported("Unsupported or invalid escape")


def _lex(source):
    tokens, comments, errors = [], [], []
    i, size = 0, len(source)
    while i < size:
        start = i
        if source[i].isspace():
            i += 1
            continue
        if source.startswith("//", i):
            end = source.find("\n", i)
            i = size if end == -1 else end
            comments.append(Token(source[start:i], start, i, "comment"))
            continue
        if source.startswith("/*", i):
            depth, i = 1, i + 2
            while i < size and depth:
                if source.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif source.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
            if depth:
                errors.append((start, size, "Unterminated nested block comment"))
                break
            comments.append(Token(source[start:i], start, i, "comment"))
            continue
        # r/br/cr raw strings. r#identifier is an identifier, not a string.
        raw = None
        for prefix in ("br", "cr", "r"):
            if source.startswith(prefix, i):
                at = i + len(prefix)
                while at < size and source[at] == "#":
                    at += 1
                if at < size and source[at] == '"':
                    raw = (at, at - i - len(prefix))
                    break
        if raw:
            quote, hashes = raw
            closing = '"' + "#" * hashes
            end = source.find(closing, quote + 1)
            if end == -1:
                errors.append((start, size, "Unterminated raw string"))
                break
            i = _suffix(source, end + len(closing))
            tokens.append(Token(source[start:i], start, i, "literal"))
            continue
        quote = i + 1 if source[i:i + 1] in ("b", "c") and source[i + 1:i + 2] == '"' else i
        if source[quote:quote + 1] == '"':
            i = quote + 1
            try:
                while i < size and source[i] != '"':
                    if source[i] == "\\":
                        if source[i + 1:i + 2] in ("\n", "\r"):
                            i += 2
                            while i < size and source[i].isspace():
                                i += 1
                        else:
                            i = _escape(source, i)
                    else:
                        i += 1
                if i == size:
                    raise Unsupported("Unterminated escaped string")
            except Unsupported as exc:
                errors.append((start, size, str(exc)))
                break
            i = _suffix(source, i + 1)
            tokens.append(Token(source[start:i], start, i, "literal"))
            continue
        quote = i + 1 if source.startswith("b'", i) else i
        if source[quote:quote + 1] == "'":
            char_end = quote + 1
            try:
                if source[char_end:char_end + 1] == "\\":
                    char_end = _escape(source, char_end)
                elif char_end < size and source[char_end] not in "'\r\n\t":
                    char_end += 1
                if source[char_end:char_end + 1] == "'":
                    if quote != start and (source[quote + 1:quote + 3] == "\\u"
                                           or (source[quote + 1] != "\\" and ord(source[quote + 1]) > 127)):
                        raise Unsupported("Invalid byte-character scalar")
                    i = _suffix(source, char_end + 1)
                    tokens.append(Token(source[start:i], start, i, "literal"))
                    continue
                if quote != start:
                    raise Unsupported("Invalid byte character")
                at = quote + 1
                if source.startswith("r#", at):
                    at += 2
                if at < size and _identifier_start(source[at]):
                    i = _identifier_end(source, at + 1)
                    if source[i:i + 1] == "'":
                        raise Unsupported("Invalid multi-scalar character literal")
                    tokens.append(Token(source[start:i], start, i, "lifetime"))
                    continue
                raise Unsupported("Unterminated/unsupported character or lifetime")
            except Unsupported as exc:
                errors.append((start, size, str(exc)))
                break
        if _identifier_start(source[i]):
            i = _identifier_end(source, i + 1)
            if source[start:i] == "r" and source[i:i + 1] == "#" and _identifier_start(source[i + 1:i + 2]):
                i = _identifier_end(source, i + 2)
            tokens.append(Token(source[start:i], start, i, "identifier"))
            continue
        operator = next((op for op in ("::", "->", "=>", "!=", "&&", "||", "..=")
                         if source.startswith(op, i)), source[i])
        i += len(operator)
        tokens.append(Token(operator, start, i, "punctuation"))
    return tokens, comments, errors


def _structure(source):
    tokens, comments, errors = _lex(source)
    pairs, stack = {}, []
    for i, token in enumerate(tokens):
        if token.text in ("(", "[", "{"):
            stack.append(i)
        elif token.text in (")", "]", "}"):
            if not stack or tokens[stack[-1]].text != {")": "(", "]": "[", "}": "{"}[token.text]:
                errors.append((token.start, token.end, "Unbalanced delimiter"))
                continue
            opening = stack.pop()
            pairs[opening] = i
            pairs[i] = opening
    errors.extend((tokens[i].start, len(source), "Unclosed delimiter") for i in stack)
    opaque, attributes = [], []
    for i, token in enumerate(tokens):
        if token.text == "#" and i + 1 < len(tokens):
            at = i + 2 if tokens[i + 1].text == "!" else i + 1
            if at in pairs and tokens[at].text == "[":
                attributes.append((i, pairs[at]))
        if token.text == "!" and i and tokens[i - 1].kind == "identifier":
            at = i + 1
            if tokens[i - 1].text == "macro_rules" and at < len(tokens) and tokens[at].kind == "identifier":
                at += 1
            if at in pairs and tokens[at].text in ("(", "[", "{"):
                opaque.append((tokens[i - 1].start, tokens[pairs[at]].end))
    def hidden(index):
        return any(a <= tokens[index].start < b for a, b in opaque) or any(a <= index <= b for a, b in attributes)
    return tokens, comments, errors, pairs, opaque, attributes, hidden


def _declarations(source, structure):
    tokens, comments, errors, pairs, opaque, attributes, hidden = structure
    declarations = []
    for index, token in enumerate(tokens):
        if hidden(index) or token.text not in ("fn", "impl", "trait", "mod", "struct", "enum", "type", "const", "static", "union", "use", "extern"):
            continue
        is_fn = token.text == "fn"
        if is_fn and (index + 1 >= len(tokens) or tokens[index + 1].kind != "identifier"):
            continue  # function-pointer types do not declare an item
        if token.text == "const" and index + 1 < len(tokens) and tokens[index + 1].text == "fn":
            continue
        if token.text == "extern":
            following = index + 1
            if following < len(tokens) and tokens[following].kind == "literal":
                following += 1
            if following < len(tokens) and tokens[following].text == "fn":
                continue  # ABI qualifier belongs to the function, not a block.
        at, angle = index + (2 if is_fn else 1), 0
        while at < len(tokens):
            word = tokens[at].text
            if word == "<":
                angle += 1
            elif word == ">" and angle:
                angle -= 1
            elif word in ("(", "[") and at in pairs:
                at = pairs[at]
            elif word == "{" and at in pairs:
                if angle:
                    at = pairs[at]
                else:
                    break
            elif word == ";" and not angle:
                break
            at += 1
        if at >= len(tokens):
            continue
        opening = at if tokens[at].text == "{" else None
        closing = pairs.get(at, at)
        if not is_fn and closing + 1 < len(tokens) and tokens[closing + 1].text == ";":
            closing += 1
        # Bodyless fn items remain exclusion extents; they are never selectable.
        start_index = index
        # Only source qualifiers can precede the fn/impl declaration.
        while start_index:
            prev = tokens[start_index - 1]
            if prev.text in ("pub", "async", "unsafe", "const", "default", "extern"):
                start_index -= 1
            elif prev.text == ")" and start_index - 1 in pairs:
                p = pairs[start_index - 1]
                if p and tokens[p - 1].text == "pub":
                    start_index = p - 1
                else:
                    break
            elif prev.kind == "literal" and start_index > 1 and tokens[start_index - 2].text == "extern":
                start_index -= 2
            else:
                break
        code_start = tokens[start_index].start
        relevant_attrs = []
        while start_index and tokens[start_index - 1].text == "]":
            found = next(((a, b) for a, b in attributes if b == start_index - 1), None)
            if found is None:
                break
            if tokens[found[0] + 1].text == "!":
                break  # Inner attributes annotate their scope, not the next item.
            relevant_attrs.append(found)
            start_index = found[0]
        start = tokens[start_index].start
        # Attached outer docs, not //! module documentation or ordinary comments.
        docs, cursor = [], start
        for comment in reversed(comments):
            if comment.end > cursor:
                continue
            if source[comment.end:cursor].strip():
                break
            if comment.text.startswith(("///", "/**")) and not comment.text.startswith(("////", "/***")):
                docs.append(comment)
            cursor = comment.start
        declarations.append({
            "type": token.text, "name": tokens[index + 1].text if is_fn else "",
            "start": start, "token": index, "opening": opening, "closing": closing,
            "code_start": code_start,
            "end": tokens[closing].end, "attrs": relevant_attrs,
            "doc_starts": [doc.start for doc in docs],
        })
    # Return-type `impl Trait` and qualifiers must not masquerade as containers.
    functions = [d for d in declarations if d["type"] == "fn" and d["opening"] is not None]
    return [d for d in declarations if d["type"] == "fn" or not any(
        f["start"] < tokens[d["token"]].start < tokens[f["opening"]].start for f in functions)]


def _path(value):
    if not isinstance(value, str):
        raise Unsupported("Invalid witness path")
    parts = re.split(r"[/\\]+", value)
    if any(part in ("", ".", "..") or ":" in part for part in parts):
        raise Unsupported("Noncanonical witness path")
    return "/".join(parts)


def _non_rewriting_lint_attribute(source, tokens, attribute):
    """Complete contract §8.2 grammar, not a correction/suppression policy."""
    a, b = attribute
    text = source[tokens[a].start:tokens[b].end]
    ws = r"[ \t\r\n\f\v]*"
    ident = r"[A-Za-z_][A-Za-z0-9_]*"
    lint = rf"(?:(?:clippy|rustdoc){ws}::{ws})?{ident}"
    pattern = (rf"\#{ws}(?:!{ws})?\[{ws}(?:allow|warn|deny|forbid){ws}"
               rf"\({ws}{lint}(?:{ws},{ws}{lint})*{ws},?{ws}\){ws}\]")
    return re.fullmatch(pattern, text) is not None


def _impl_identity(parts):
    """Ordered trait/self-type roles, with a bounded erased-generic display."""
    if parts and parts[0] == "impl":
        parts = parts[1:]
    cleaned, depth = [], 0
    for part in parts:
        if part == "<":
            depth += 1
        elif part == ">" and depth:
            depth -= 1
        elif depth:
            continue
        elif part == "where":
            break
        else:
            cleaned.append(part)
    if depth:
        raise Unsupported("Unbalanced impl identity generics")
    def type_path(items):
        if not items or len(items) % 2 == 0:
            raise Unsupported("Unsupported impl type identity")
        for i, part in enumerate(items):
            if i % 2:
                if part != "::":
                    raise Unsupported("Unsupported impl path separator")
            elif not re.fullmatch(r"(?:r#)?[^\W\d]\w*", part):
                raise Unsupported("Unsupported impl path identifier")
        return tuple(items)
    if cleaned.count("for") == 1:
        at = cleaned.index("for")
        return ("trait_impl", type_path(cleaned[:at]), type_path(cleaned[at + 1:]))
    if "for" in cleaned:
        raise Unsupported("Unsupported impl role identity")
    return ("inherent_impl", type_path(cleaned))


def _parent_identity_matches(source, structure, declaration, name_path):
    tokens = structure[0]
    leaf = name_path.split("/")[-1]
    index = declaration["token"]
    if declaration["type"] in ("fn", "mod", "trait"):
        expected = tokens[index + 1].text.removeprefix("r#")
        return leaf in (expected, declaration["type"] + " " + expected)
    if declaration["type"] != "impl":
        raise Unsupported("Unsupported identity-bearing source container")
    source_parts = [token.text for token in tokens[index:declaration["opening"]]]
    label_tokens, comments, errors = _lex(leaf)
    if errors or comments or any(token.kind == "literal" for token in label_tokens):
        raise Unsupported("Unsupported captured impl identity spelling")
    label_parts = [token.text for token in label_tokens]
    if "<" in label_parts:
        # Explicit generic arguments must agree literally; do not erase a
        # captured, contradictory argument list. Erased displays are supported.
        if source_parts != label_parts:
            return False
    return _impl_identity(source_parts) == _impl_identity(label_parts)


def _verify_parent_chain(source, structure, declarations, selected, witness, witnesses, path):
    tokens = structure[0]
    chain = sorted((d for d in declarations if d != selected and d["opening"] is not None
                    and tokens[d["opening"]].start < selected["start"] < d["end"]),
                   key=lambda d: d["end"] - d["start"])
    current = witness
    for parent in chain:
        name = current.get("parent_name_path")
        encoded = current["name_path"].rsplit("/", 1)[0] if "/" in current["name_path"] else None
        if not name or (encoded is not None and encoded != name):
            raise Unsupported("Captured hierarchy does not identify the immediate source parent")
        allowed = (FUNCTION_KINDS if parent["type"] == "fn" else
                   {"Object", "Impl", 19} if parent["type"] == "impl" else
                   {"Module", "Namespace", 2, 3} if parent["type"] == "mod" else
                   {"Trait", "Interface", 11} if parent["type"] == "trait" else set())
        matches = {}
        for candidate in witnesses:
            if (not isinstance(candidate, dict) or not candidate.get("query_id")
                    or candidate.get("name_path") != name or candidate.get("kind") not in allowed
                    or _path(candidate.get("relative_path")) != path):
                continue
            extent = _native_range(source, candidate.get("body_location", {}))
            a, b = _range(source, extent)
            starts = {source.count("\n", 0, at) + 1 for at in
                      parent["doc_starts"] + [parent["start"], parent["code_start"]]}
            if (extent["start_line"] not in starts
                    or candidate["body_location"]["end_line"] + 1 != source.count("\n", 0, parent["end"] - 1) + 1
                    or not a <= parent["code_start"] < parent["end"] <= b
                    or source[a:min(parent["doc_starts"] + [parent["start"]])].strip()
                    or source[parent["end"]:b].strip()
                    or not _parent_identity_matches(source, structure, parent, name)):
                continue
            identity = (name, extent["start_line"], extent["end_line"],
                        candidate.get("parent_name_path"), candidate["kind"])
            matches[identity] = candidate
        if len(matches) != 1:
            raise Unsupported("Positive ordered parent identity/full ancestor chain required")
        current = next(iter(matches.values()))
    if current.get("parent_name_path") is not None or "/" in current["name_path"]:
        raise Unsupported("Captured hierarchy invents a parent beyond the source root")


def verify_function_range(source, symbol, diagnostic_range, inventory_column_unit, witnesses):
    """Verify physical localization only, per contract §8.2; never bind hashes."""
    flags = {key: False for key in PREDICATES}
    def result(status, reasons, candidates=(), extent=None):
        return {"status": status, "normalized_range": extent,
                "candidate_name_paths": sorted(set(candidates)),
                "checked_predicates": flags, "reasons": reasons}
    try:
        begin = _position(source, diagnostic_range[0], diagnostic_range[1], inventory_column_unit)
        end = _position(source, diagnostic_range[2], diagnostic_range[3], inventory_column_unit)
        if not begin < end:
            raise Unsupported("Empty or reversed diagnostic")
        flags["diagnostic_coordinates_verified"] = True
        selected_range = _range(source, symbol["range"])
        path = _path(symbol["path"])
        structure = _structure(source)
        tokens, comments, errors, pairs, opaque, attributes, hidden = structure
        declarations = _declarations(source, structure)
        functions = [d for d in declarations if d["type"] == "fn" and d["opening"] is not None]
        qualified_leaf = symbol["qualified_name"].split("::")[-1].split("/")[-1]
        candidates = []
        for witness in witnesses:
            if (not isinstance(witness, dict) or not witness.get("query_id")
                    or witness.get("kind") not in FUNCTION_KINDS
                    or _path(witness.get("relative_path")) != path):
                continue
            name_path = witness.get("name_path")
            if not isinstance(name_path, str) or not name_path:
                continue
            extent = _native_range(source, witness.get("body_location", {}))
            bounds = _range(source, extent)
            if not bounds[0] <= begin < end <= bounds[1]:
                continue
            for declaration in functions:
                if declaration["name"].removeprefix("r#") != name_path.split("/")[-1].removeprefix("r#"):
                    continue
                line_start = source.count("\n", 0, declaration["start"]) + 1
                valid_starts = {line_start, source.count("\n", 0, declaration["code_start"]) + 1} | {
                    source.count("\n", 0, at) + 1 for at in declaration["doc_starts"]}
                closing_line = source.count("\n", 0, declaration["end"] - 1) + 1
                if extent["start_line"] not in valid_starts or witness["body_location"]["end_line"] + 1 != closing_line:
                    continue
                if not bounds[0] <= declaration["code_start"] < declaration["end"] <= bounds[1]:
                    continue
                # Missing columns cannot hide sibling code on boundary lines.
                if source[bounds[0]:min(declaration["doc_starts"] + [declaration["start"]])].strip():
                    continue
                if source[declaration["end"]:bounds[1]].strip():
                    continue
                candidates.append((declaration, witness, extent, bounds))
        selected = []
        identities = set()
        for candidate in candidates:
            w = candidate[1]
            identity = (w["name_path"], w["kind"], candidate[3], w.get("parent_name_path"))
            prefix = "serena::" + path + "::"
            exact_name = (not symbol["qualified_name"].startswith("serena::")
                          or symbol["qualified_name"] == prefix + w["name_path"])
            if (exact_name and w["name_path"].split("/")[-1] == qualified_leaf
                    and candidate[3] == selected_range and identity not in identities):
                selected.append(candidate)
                identities.add(identity)
        if len(selected) != 1:
            return result("ambiguous" if selected else "unsupported",
                          ["Selected witness/endpoints are missing or ambiguous"],
                          [c[1]["name_path"] for c in selected])
        declaration, witness, extent, bounds = selected[0]
        if any(a < declaration["end"] and b > bounds[0] for a, b, _ in errors):
            raise Unsupported("Unterminated or unbalanced lexical state affects selected function")
        if any(a < end and b > begin for a, b in opaque):
            raise Unsupported("Diagnostic overlaps an opaque macro token tree")
        if any(not _non_rewriting_lint_attribute(source, tokens, attribute)
               for attribute in declaration["attrs"]):
            raise Unsupported("Selected declaration has unverified rewriting/cfg attributes")
        if any(marker in source[min(declaration["doc_starts"] + [declaration["start"]]):declaration["start"]].lower()
               for marker in ("@generated", "automatically generated", "do not edit")):
            raise Unsupported("Generated selected declaration")
        containers = [d for d in declarations if d != declaration and d["opening"] is not None
                      and tokens[d["opening"]].start < declaration["start"] < d["end"]]
        if symbol.get("kind") in ("Method", "method", 6) and not any(
                container["type"] in ("impl", "trait") for container in containers):
            raise Unsupported("Method witness lacks a source impl/trait context")
        if any(not _non_rewriting_lint_attribute(source, tokens, attribute)
               for container in containers for attribute in container["attrs"]):
            raise Unsupported("Identity-bearing ancestor has unverified attributes")
        generated = ("@generated", "automatically generated", "do not edit")
        if any(comment.start < tokens[0].start and any(m in comment.text.lower() for m in generated)
               for comment in comments):
            raise Unsupported("Generated file header")
        if any(any(m in source[min(d["doc_starts"] + [d["start"]]):d["start"]].lower() for m in generated)
               for d in containers):
            raise Unsupported("Generated identity-bearing ancestor")
        for a, b in attributes:
            if (a + 1 < len(tokens) and tokens[a + 1].text == "!"
                    and not any(left <= tokens[a].start < right for left, right in opaque)):
                if _non_rewriting_lint_attribute(source, tokens, (a, b)):
                    continue  # Continue checking every other relevant attribute.
                groups = [i for i, token in enumerate(tokens) if token.text == "{" and i in pairs
                           and token.start < tokens[a].start < tokens[pairs[i]].end]
                nearest = max(groups) if groups else None
                relevant = {declaration["opening"]} | {container["opening"] for container in containers}
                if nearest is None or nearest in relevant:
                    raise Unsupported("Selected item/identity ancestor has unverified inner attributes")
                if tokens[nearest].start < end and tokens[pairs[nearest]].end > begin:
                    raise Unsupported("Diagnostic intersects an inner-attributed lexical block")
        _verify_parent_chain(source, structure, declarations, declaration, witness, witnesses, path)
        for child in declarations:
            if child != declaration and declaration["start"] < child["start"] < declaration["end"]:
                if min(child["doc_starts"] + [child["start"]]) < end and child["end"] > begin:
                    return result("ambiguous", ["Diagnostic overlaps a nested source declaration"],
                                  [witness["name_path"]])
        # Closures/async block interiors require a separate symbol method.
        for i, token in enumerate(tokens):
            if (declaration["start"] < token.start < declaration["end"] and not hidden(i)
                    and token.text in ("||", "|", "async")):
                if token.text == "async" and i < declaration["token"]:
                    continue  # async fn qualifier, not an async expression block
                at = i + 1
                while (at < len(tokens) and tokens[at].start < declaration["end"]
                       and tokens[at].text not in (";", "}")):
                    at = pairs[at] + 1 if at in pairs and tokens[at].text in ("{", "(", "[") else at + 1
                stop = min(tokens[at].start, declaration["end"]) if at < len(tokens) else declaration["end"]
                if token.start < end and stop > begin:
                    raise Unsupported("Diagnostic may require expression-closure attribution")
        if symbol.get("kind") not in FUNCTION_KINDS:
            raise Unsupported("Unsupported selected symbol kind")
        if not bounds[0] <= begin < end <= min(bounds[1], declaration["end"]):
            raise Unsupported("Diagnostic not wholly in selected declaration/docs")
        for key in PREDICATES[2:]:
            flags[key] = True
        return result("verified", [], [witness["name_path"]], extent)
    except (Unsupported, KeyError, TypeError, ValueError, IndexError) as exc:
        return result("unsupported", [str(exc)])
