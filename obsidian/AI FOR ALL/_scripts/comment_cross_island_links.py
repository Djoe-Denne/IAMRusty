"""
Comment cross-group wikilinks using the same color rules as Obsidian Graph View.

Reads `.obsidian/graph.json` → `colorGroups` (order = first match wins, like Obsidian).
Wraps offending [[links]] as <!-- [[...]] --> without deleting them.

Usage:
  python comment_cross_island_links.py              # uncomment then re-apply
  python comment_cross_island_links.py --dry-run    # report only
  python comment_cross_island_links.py --uncomment-only
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

VAULT = Path(__file__).resolve().parents[1]
GRAPH_JSON = VAULT / ".obsidian" / "graph.json"

WIKILINK_RE = re.compile(r"(?<!!)(\[\[)([^\]]+)(\]\])")
COMMENTED_WIKILINK_RE = re.compile(r"<!--\s*(\[\[[^\]]+\]\])\s*-->")


def norm(p: str) -> str:
    return p.replace("\\", "/").lower().strip()


def split_top_level_or(expr: str) -> list[str]:
    """Split on OR outside parentheses."""
    expr = expr.strip()
    depth = 0
    parts: list[str] = []
    buf: list[str] = []
    i = 0
    while i < len(expr):
        c = expr[i]
        if c == "(":
            depth += 1
            buf.append(c)
        elif c == ")":
            depth -= 1
            buf.append(c)
        elif depth == 0 and expr[i : i + 4].upper() == " OR ":
            parts.append("".join(buf).strip())
            buf = []
            i += 4
            continue
        else:
            buf.append(c)
        i += 1
    if buf:
        parts.append("".join(buf).strip())
    return [p for p in parts if p]


def strip_outer_parens(s: str) -> str:
    s = s.strip()
    while s.startswith("(") and s.endswith(")"):
        inner = s[1:-1].strip()
        # only strip if balanced
        d = 0
        ok = True
        for c in inner:
            if c == "(":
                d += 1
            elif c == ")":
                d -= 1
                if d < 0:
                    ok = False
                    break
        if ok and d == 0:
            s = inner
        else:
            break
    return s


def path_prefix_match(rel: str, prefix: str) -> bool:
    """True if rel is exactly prefix or lives under that folder (first segments match)."""
    rel = rel.replace("\\", "/").lower().strip()
    prefix = prefix.lower().strip("/")
    if not prefix:
        return True
    rel_parts = rel.split("/")
    pre_parts = prefix.split("/")
    if len(rel_parts) < len(pre_parts):
        return False
    return rel_parts[: len(pre_parts)] == pre_parts


def path_atom_hit(rel: str, path_arg: str) -> bool:
    """Positive match for path:... token (Obsidian path: prefix from vault root)."""
    pa = path_arg.strip()
    if pa.lower().startswith("path:"):
        pa = pa[5:].strip()
    return path_prefix_match(rel, pa.strip("/"))


def eval_path_token(rel: str, token: str) -> bool:
    """Single path: or -path: token."""
    token = token.strip()
    neg = token.startswith("-")
    raw = token[1:] if neg else token
    if not raw.startswith("path:"):
        return True
    path_arg = raw[5:].strip()
    hit = path_atom_hit(rel, "path:" + path_arg)
    return (not hit) if neg else hit


def eval_and_clause(rel: str, clause: str) -> bool:
    clause = strip_outer_parens(clause.strip())
    tokens = clause.split()
    if not tokens:
        return False
    for t in tokens:
        if not eval_path_token(rel, t):
            return False
    return True


def eval_obsidian_query(rel: str, query: str) -> bool:
    """Subset of Obsidian search: OR, parens, path:, -path:."""
    query = query.strip()
    if not query:
        return False
    parts = split_top_level_or(query)
    if len(parts) == 0:
        return False
    if len(parts) == 1:
        return eval_and_clause(rel, parts[0])
    return any(eval_obsidian_query(rel, p) for p in parts)


def load_color_groups() -> list[dict]:
    data = json.loads(GRAPH_JSON.read_text(encoding="utf-8"))
    return list(data.get("colorGroups") or [])


def graph_group_index(rel: str, color_groups: list[dict]) -> int | None:
    """
    First matching color group index, same order as Obsidian Graph.
    None = default / uncolored in the graph.
    """
    r = rel.replace("\\", "/")
    if "_archives/" in r:
        return None
    for i, cg in enumerate(color_groups):
        q = cg.get("query") or ""
        if eval_obsidian_query(r, q):
            return i
    return None


def build_indexes():
    by_rel: dict[str, str] = {}
    by_basename: dict[str, list[str]] = {}
    for p in VAULT.rglob("*.md"):
        rel = p.relative_to(VAULT).as_posix()
        if "_archives/" in rel:
            continue
        key = norm(rel)
        by_rel[key] = rel
        stem = Path(rel).stem.lower()
        by_basename.setdefault(stem, []).append(rel)
    return by_rel, by_basename


def resolve_link(link_inner: str, by_rel: dict, by_basename: dict) -> str | None:
    inner = link_inner.strip()
    if "|" in inner:
        inner = inner.split("|", 1)[0].strip()
    inner = inner.strip()
    if not inner:
        return None
    candidates: list[str] = []
    for suf in ("", ".md"):
        k = norm(inner + suf) if not inner.endswith(".md") else norm(inner)
        if k in by_rel:
            candidates.append(by_rel[k])
    if inner.endswith(".md"):
        k = norm(inner)
        if k in by_rel:
            candidates.append(by_rel[k])
    if candidates:
        return min(candidates, key=len)
    stem = Path(inner.replace("\\", "/")).stem.lower()
    paths = by_basename.get(stem, [])
    if len(paths) == 1:
        return paths[0]
    matches = [p for p in paths if norm(p).endswith("/" + stem + ".md")]
    if len(matches) == 1:
        return matches[0]
    return None


def strip_fenced_blocks(text: str) -> tuple[str, list[str]]:
    placeholders: list[str] = []
    fence_re = re.compile(r"^(```|~~~)([^\n]*)\n.*?^\1\s*$", re.MULTILINE | re.DOTALL)

    def repl(m: re.Match) -> str:
        placeholders.append(m.group(0))
        return f"\0FENCE{len(placeholders)-1}\0"

    return fence_re.sub(repl, text), placeholders


def restore_fences(text: str, placeholders: list[str]) -> str:
    for i, ph in enumerate(placeholders):
        text = text.replace(f"\0FENCE{i}\0", ph)
    return text


def uncomment_wikilinks(text: str) -> tuple[str, int]:
    count = len(COMMENTED_WIKILINK_RE.findall(text))
    new_text = COMMENTED_WIKILINK_RE.sub(r"\1", text)
    return new_text, count


def already_commented_region(text: str, start: int, end: int) -> bool:
    before = text[:start]
    open_cm = before.rfind("<!--")
    close_cm = before.rfind("-->")
    if open_cm != -1 and (close_cm == -1 or open_cm > close_cm):
        return True
    return False


def process_file(
    path: Path,
    by_rel: dict,
    by_basename: dict,
    color_groups: list[dict],
    dry_run: bool,
    do_uncomment_first: bool,
) -> tuple[int, int]:
    raw = path.read_text(encoding="utf-8")
    rel = path.relative_to(VAULT).as_posix()
    if "_archives/" in rel:
        return 0, 0

    uncommented = 0
    if do_uncomment_first:
        raw, uncommented = uncomment_wikilinks(raw)

    body, fences = strip_fenced_blocks(raw)
    changed = 0
    src_g = graph_group_index(rel, color_groups)

    def repl(m: re.Match) -> str:
        nonlocal changed
        inner = m.group(2)
        if inner.strip().startswith("!"):
            return m.group(0)
        if already_commented_region(body, m.start(), m.end()):
            return m.group(0)
        target = resolve_link(inner, by_rel, by_basename)
        if not target:
            return m.group(0)
        tgt_g = graph_group_index(target, color_groups)
        if src_g == tgt_g:
            return m.group(0)
        changed += 1
        return f"<!-- {m.group(1)}{inner}{m.group(3)} -->"

    new_body = WIKILINK_RE.sub(repl, body)
    new_raw = restore_fences(new_body, fences)
    if new_raw != raw and not dry_run:
        path.write_text(new_raw, encoding="utf-8", newline="\n")
    return changed, uncommented


def main():
    dry = "--dry-run" in sys.argv
    uncomment_only = "--uncomment-only" in sys.argv

    if not GRAPH_JSON.is_file():
        print(f"Missing {GRAPH_JSON}", file=sys.stderr)
        sys.exit(1)

    color_groups = load_color_groups()
    if not color_groups:
        print("No colorGroups in graph.json", file=sys.stderr)
        sys.exit(1)

    by_rel, by_basename = build_indexes()
    total_comment = 0
    total_uncomment = 0
    touched = 0

    for p in sorted(VAULT.rglob("*.md")):
        if "_archives/" in p.as_posix():
            continue
        if uncomment_only:
            raw = p.read_text(encoding="utf-8")
            new_raw, n = uncomment_wikilinks(raw)
            total_uncomment += n
            if n and not dry_run:
                p.write_text(new_raw, encoding="utf-8", newline="\n")
            if n:
                touched += 1
                print(f"unc {n:3}  {p.relative_to(VAULT)}")
        else:
            n_c, n_u = process_file(
                p, by_rel, by_basename, color_groups, dry, do_uncomment_first=True
            )
            total_uncomment += n_u
            total_comment += n_c
            if n_c or n_u:
                touched += 1
                if n_u or n_c:
                    print(f"u{n_u:3} c{n_c:3}  {p.relative_to(VAULT)}")

    if uncomment_only:
        print(f"\nFiles touched: {touched}, uncommented: {total_uncomment}" + (" (dry-run)" if dry else ""))
    else:
        print(
            f"\nFiles touched: {touched}, uncommented: {total_uncomment}, links commented: {total_comment}"
            + (" (dry-run)" if dry else "")
        )


if __name__ == "__main__":
    main()
