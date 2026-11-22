# Review briefings (local)

Review briefings are local and gitignored by default. Read relevant briefings
before redoing the investigation; do not add new findings to Git by default.

Motif : une revue dans le chat (findings A-1, T-1, T-2, …) disparaît au tour suivant. L’implementer ré-explore invoke, identity, KV purge, enrollment, etc. Le briefing est la mémoire de travail de la revue.

## Contrat

| Rôle | Obligation |
|---|---|
| `correctness-reviewer`, `test-reviewer`, `rust-perf-reviewer`, `security-reviewer` | Après **chaque** revue (PASS, commentaires, BLOCK) : remplir le template et viser un fichier ici. Si `readonly` refuse l’écriture, renvoyer `BRIEFING_PATH` + `BRIEFING_MARKDOWN` complets. |
| Primary session | Persist the briefing if the reviewer could not, update `INDEX.md`, and pass the relevant findings and evidence to the author. |
| `implementer` or `test-engineer` | Read matching briefings before exploring. Authors, not reviewers, implement fixes. |

`INDEX.md` et les briefings `*.md` (hors README/TEMPLATE) ne se commitent pas.

## Nom de fichier

```text
YYYYMMDDTHHMMZ-<reviewer>-<scope-slug>-<shortsha>.md
```

- Heure **UTC**. Exemple : `20260917T1900Z-correctness-apparatus-p3-t9-a1b2c3d.md`
- `<reviewer>` : `correctness` | `tests` | `rust-perf` | `security`
- `<scope-slug>` : kebab du work package ou de la branche, ≤40 caractères, ASCII
- `<shortsha>` : `git rev-parse --short HEAD` (arbre sale : SHA HEAD + `-dirty` dans le frontmatter, pas dans le nom)

## INDEX.md

Créer si absent. Une ligne par briefing, les plus récents en haut :

```markdown
| Written (UTC) | Reviewer | Scope | HEAD | Dirty | Verdict | File |
|---|---|---|---|---|---|---|
| 2026-09-17T19:00Z | correctness | apparatus-p3-t9 | a1b2c3d | yes | GO_WITH_COMMENTS | 20260917T1900Z-correctness-apparatus-p3-t9-a1b2c3d.md |
```

Garder ~30 lignes. Ne pas réécrire l’historique des fichiers.

## Stale

Un briefing n’est **pas** parole d’évangile si :

- `head_sha` ≠ HEAD actuel, ou
- un fichier listé a changé depuis `written_at`, ou
- la branche n’est plus la même.

Alors : garder findings / *why* / forme du fix comme **pistes**, **revérifier** les `fichier:ligne`, ne pas recopier un patch obsolète.

## Secrets

Interdit dans un briefing : tokens, JWT, connection strings, secrets Vault, PII. Remplacer par `<redacted>` ou omettre.

## Comment lire (implementer)

1. `INDEX.md` s’il existe.
2. Fichiers dont le scope / SHA / reviewer matchent le work package.
3. Appliquer `Suggested fix shape` et `Tests to run`.
4. Respecter `Do not redo` et `Anti-goals`.
