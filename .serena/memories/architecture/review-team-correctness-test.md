# Équipe reviewers — 4 reviewers (2026-09-15)

- `correctness-reviewer` — bugs logiques/comportementaux + contrats observables + doc impactée.
- `test-reviewer` — couverture des risques par tests, mutation mindset, scénario de régression obligatoire.
- `rust-perf-reviewer` — ownership/borrowing/lifetimes, allocations, CPU, mémoire, concurrence, monomorphisation, binary size. Contraintes : unsafe interdit (workspace forbid), MSRV 1.84, zéro bench → MEASURE obligatoire.
- `security-reviewer` (ajouté 2026-09-15) — Security / Red-Team ancré repo : deux passes (défensive + adversariale), trust boundaries ADR 0003/0004/0007, IDOR/authz, secrets & logs, taint data-flow, races TOCTOU, DoS, supply chain, CI, prompt injection. Blocking : CRITICAL/HIGH = BLOCK (override humain seul), MEDIUM = REVIEW REQUIRED.

## Trust boundaries canon (dans le prompt security-reviewer)
Plugin ↔ gateway Lazaret (jamais bearer IAM/HMAC au plugin) ; utilisateur ↔ services (JWT HS256 iss=iamrusty aud=aiforall ; session workload iss/aud=lazaret, autorité distincte) ; Lazaret ↔ Manifesto (grants/consentements close-at-commit, grant_revision) ; service ↔ connecteurs nommés (no-redirect, timeout) ; service ↔ Vault (références opaques secret:{path}#{field}, fail-closed) ; service ↔ DB/Redis (namespace binding_id, bornes KV).

## Routing (orchestrator.md)
- Local pur → correctness seul. Logique métier/état ou contrat → correctness + test. Hot path/ownership/allocations/async/binary size + diff Rust non trivial → + rust-perf. Surface sécu (auth, permissions, Lazaret, secrets, CI, deps) → security-reviewer (+ équipe si logique métier).
- `FULL REVIEW` = correctness + test (+ rust-perf si Rust non trivial) (+ security si surface sécu).
- `SECURITY FULL REVIEW` = mode élargi security-reviewer (2 passes + threat models + attack chains + supply chain + régressions sécu + PoC).
- DEFER_TO mutuels à jour entre les 4 reviewers.

## Validation security (2026-09-15, subagents glm-5p3)
1. `5707f9e^` pré-fix Hive : 3 HIGH retrouvés — IDOR list/get/update members (requester manquant), fuite token invitation (InvitationResponse.token sérialisé), privesc rôles (role_is_privileged absent). Chaîne d'exploitation complète. Verdict BLOCK. Tous true positives.
2. Lazaret actuel : S-1 HIGH réel (parse_secret_reference accepte .. / absolu → traversal Vault via normalisation reqwest, exfil via connecteur admis — recommandation allowlist/réjection '..'), S-2 MEDIUM (connector.bytes() non plafonné + InvokeResponse::validate jamais appelé → DoS monolithe), S-3 MEDIUM (kv.put erreur driver → BadRequest 400 = fuite topologie), S-4/S-5 LOW, S-6 HARDENING. Angles sains confirmés avec preuve : connecteurs nommés, fail-closed DeniedSecretResolver, from_utf8_lossy bearer (infirmé comme vuln), /invoke préfixé (infirmé), bornes KV + namespace.
- Ajustement prompt : aucun requis. Corpus security informel : 5707f9e^ (IDOR/fuite/privesc) + S-1 traversal Vault.