"""
Uncomment only commented wikilinks that bridge two existing islands.

An island is computed as a connected component of ACTIVE wikilinks.
Then this tool only restores links of the form:
  component(A) <-> component(B)

Usage example:
  python _scripts/uncomment_links_between_islands.py ^
    --seed-a references/index.md --seed-a entities/index.md ^
    --seed-b projects/rustycog/rustycog.md
"""
from __future__ import annotations

import argparse
import re
from collections import defaultdict, deque
from pathlib import Path

VAULT = Path(__file__).resolve().parents[1]

WIKILINK_RE = re.compile(r"(?<!!)\[\[([^\]]+)\]\]")
COMMENTED_WIKILINK_RE = re.compile(r"<!--\s*(\[\[[^\]]+\]\])\s*-->")
FENCE_RE = re.compile(r"^(```|~~~)([^\n]*)\n.*?^\1\s*$", re.MULTILINE | re.DOTALL)
HTML_COMMENT_RE = re.compile(r"<!--.*?-->", re.DOTALL)


def norm(p: str) -> str:
    return p.replace("\\", "/").strip().lower()


def strip_fenced_blocks(text: str) -> tuple[str, list[str]]:
    placeholders: list[str] = []

    def repl(m: re.Match) -> str:
        placeholders.append(m.group(0))
        return f"\0FENCE{len(placeholders)-1}\0"

    return FENCE_RE.sub(repl, text), placeholders


def restore_fences(text: str, placeholders: list[str]) -> str:
    for i, ph in enumerate(placeholders):
        text = text.replace(f"\0FENCE{i}\0", ph)
    return text


def build_indexes() -> tuple[dict[str, str], dict[str, list[str]]]:
    by_rel: dict[str, str] = {}
    by_basename: dict[str, list[str]] = {}
    for p in VAULT.rglob("*.md"):
        rel = p.relative_to(VAULT).as_posix()
        if "_archives/" in rel:
            continue
        by_rel[norm(rel)] = rel
        by_basename.setdefault(Path(rel).stem.lower(), []).append(rel)
    return by_rel, by_basename


def resolve_link(link_inner: str, by_rel: dict[str, str], by_basename: dict[str, list[str]]) -> str | None:
    inner = link_inner.strip()
    if "|" in inner:
        inner = inner.split("|", 1)[0].strip()
    if not inner:
        return None

    candidates: list[str] = []
    if inner.endswith(".md"):
        k = norm(inner)
        if k in by_rel:
            candidates.append(by_rel[k])
    else:
        for candidate in (inner, f"{inner}.md"):
            k = norm(candidate)
            if k in by_rel:
                candidates.append(by_rel[k])

    if candidates:
        return min(candidates, key=len)

    stem = Path(inner.replace("\\", "/")).stem.lower()
    paths = by_basename.get(stem, [])
    if len(paths) == 1:
        return paths[0]
    suffix_matches = [p for p in paths if norm(p).endswith("/" + stem + ".md")]
    if len(suffix_matches) == 1:
        return suffix_matches[0]
    return None


def resolve_seed(seed: str, by_rel: dict[str, str], by_basename: dict[str, list[str]]) -> str | None:
    # Try as a direct relative path first.
    k = norm(seed if seed.endswith(".md") else f"{seed}.md")
    if k in by_rel:
        return by_rel[k]
    # Then reuse wikilink resolver.
    return resolve_link(seed, by_rel, by_basename)


def extract_active_targets(raw: str, by_rel: dict[str, str], by_basename: dict[str, list[str]]) -> list[str]:
    masked, _ = strip_fenced_blocks(raw)
    masked = HTML_COMMENT_RE.sub("", masked)
    targets: list[str] = []
    for m in WIKILINK_RE.finditer(masked):
        target = resolve_link(m.group(1), by_rel, by_basename)
        if target:
            targets.append(target)
    return targets


def build_active_graph(by_rel: dict[str, str], by_basename: dict[str, list[str]]) -> dict[str, set[str]]:
    adj: dict[str, set[str]] = defaultdict(set)
    for p in VAULT.rglob("*.md"):
        rel = p.relative_to(VAULT).as_posix()
        if "_archives/" in rel:
            continue
        raw = p.read_text(encoding="utf-8")
        for tgt in extract_active_targets(raw, by_rel, by_basename):
            adj[rel].add(tgt)
            adj[tgt].add(rel)
        # Ensure isolated nodes exist in graph.
        adj.setdefault(rel, set())
    return adj


def component_from_seeds(adj: dict[str, set[str]], seeds: list[str]) -> set[str]:
    comp: set[str] = set()
    q: deque[str] = deque()
    for s in seeds:
        if s in adj and s not in comp:
            comp.add(s)
            q.append(s)
    while q:
        cur = q.popleft()
        for nxt in adj[cur]:
            if nxt not in comp:
                comp.add(nxt)
                q.append(nxt)
    return comp


def uncomment_bridges_in_file(
    path: Path,
    comp_a: set[str],
    comp_b: set[str],
    by_rel: dict[str, str],
    by_basename: dict[str, list[str]],
    dry_run: bool,
) -> int:
    raw = path.read_text(encoding="utf-8")
    rel = path.relative_to(VAULT).as_posix()
    if "_archives/" in rel:
        return 0

    masked, fences = strip_fenced_blocks(raw)
    changed = 0

    def repl(m: re.Match) -> str:
        nonlocal changed
        token = m.group(1)  # [[...]]
        inner = token[2:-2]
        target = resolve_link(inner, by_rel, by_basename)
        if not target:
            return m.group(0)
        bridges = (rel in comp_a and target in comp_b) or (rel in comp_b and target in comp_a)
        if not bridges:
            return m.group(0)
        changed += 1
        return token

    new_masked = COMMENTED_WIKILINK_RE.sub(repl, masked)
    new_raw = restore_fences(new_masked, fences)
    if changed and not dry_run:
        path.write_text(new_raw, encoding="utf-8", newline="\n")
    return changed


def main() -> None:
    ap = argparse.ArgumentParser(description="Uncomment bridge links between two islands.")
    ap.add_argument("--seed-a", action="append", required=True, help="Seed page for island A (repeatable)")
    ap.add_argument("--seed-b", action="append", required=True, help="Seed page for island B (repeatable)")
    ap.add_argument("--dry-run", action="store_true", help="Report only, do not write files")
    args = ap.parse_args()

    by_rel, by_basename = build_indexes()

    seeds_a: list[str] = []
    seeds_b: list[str] = []
    for s in args.seed_a:
        r = resolve_seed(s, by_rel, by_basename)
        if not r:
            raise SystemExit(f"Unresolved --seed-a: {s}")
        seeds_a.append(r)
    for s in args.seed_b:
        r = resolve_seed(s, by_rel, by_basename)
        if not r:
            raise SystemExit(f"Unresolved --seed-b: {s}")
        seeds_b.append(r)

    adj = build_active_graph(by_rel, by_basename)
    comp_a = component_from_seeds(adj, seeds_a)
    comp_b = component_from_seeds(adj, seeds_b)

    touched = 0
    total = 0
    for p in sorted(VAULT.rglob("*.md")):
        rel = p.relative_to(VAULT).as_posix()
        if "_archives/" in rel:
            continue
        n = uncomment_bridges_in_file(p, comp_a, comp_b, by_rel, by_basename, args.dry_run)
        if n:
            touched += 1
            total += n
            print(f"{n:3}  {rel}")

    print(
        f"\nIsland A size: {len(comp_a)} | Island B size: {len(comp_b)} | "
        f"files touched: {touched} | links uncommented: {total}"
        + (" (dry-run)" if args.dry_run else "")
    )


if __name__ == "__main__":
    main()
