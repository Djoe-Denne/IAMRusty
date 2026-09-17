# Review briefings locaux (gitignorés)

**Décidé 2026-09-17.** Les reviewers ne laissent plus leur raisonnement seulement dans le chat.

## Où
`.cursor/review-briefings/` — findings + `INDEX.md` **gitignorés**. Trackés : `README.md`, `TEMPLATE.md`.

Nom : `YYYYMMDDTHHMMZ-<reviewer>-<scope-slug>-<shortsha>.md`
(`correctness` | `tests` | `rust-perf` | `security`).

## Contrat
- **Écrire** (même PASS/BLOCK) : correctness, test, rust-perf, security-reviewer. Si `readonly` : renvoyer `BRIEFING_PATH` + `BRIEFING_MARKDOWN` ; l’orchestrateur persiste.
- **Lire avant d’explorer** : orchestrator, implementer, hard-implementer, emergency-engineer. Mechanical-worker : seulement les chemins du work package.
- **Stale** si SHA / fichiers / branche ont bougé : pistes, pas évangile ; revérifier `fichier:ligne`.
- **Secrets** interdits (tokens, JWT, connection strings, PII).

## Motif
Chat `a6bcd095-8de8-4f13-87d0-3ae55f4e7244` : les implementers ont redécouvert A-1 / T-1 / T-2 / T-5 (invoke, identity, KV purge, enrollment) faute de briefing local.

Voir aussi `mem:architecture/review-team-correctness-test`.
