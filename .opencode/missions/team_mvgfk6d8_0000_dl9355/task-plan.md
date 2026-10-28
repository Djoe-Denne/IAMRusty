# Plan de mission — Apparatus : runtime HTTP Knative par binding

- Mission : `team_mvgfk6d8_0000_dl9355`.
- Policy **approuvée** : révision 1, hash `c110f9aae29751b5833cd8a8e14aa76eebf8e86b19005912bc8b7ad87ddfd411` (2026-10-20).
- Classification : `general` (LLM `openai/gpt-6.1-sol`), 0 appel ; toutes les tâches portent des scores manuels → `not_required`.
- Base : HEAD `2760e02bc43f5149070aac6ace29f7c2d11f9aad` = base de l'ordre de mission ; arbre sale = préparation documentaire uniquement ; sous-module `rustycog` → `heads/fix/it-fixtures-lifecycle`. Aucun commit autorisé en préparation.
- Source : plan `project-manager` `ses_ea7858190ffeo5124Sg6Ly1r3w` ; scores validés par la lead (plancher complexité 70 appliqué à B et T).
- Grille approuvée : strong = `openai/gpt-6.1-sol` (low) — Sol partout, Astra absent de cette mission ; le routage est imposé par Ensemble, sans override explicite.

## 1. Tâches (publication topologique après `/ensemble-mission start`)

| Réf | Lot | Owner | Périmètre possédé | Dépend de | Compl./Risque | Groupe |
|---|---|---|---|---|---|---|
| A | Contrat + ratification | `architecte` (reprise `ses_eab4461deffeoHdOxvDRyUOlk4`) | `docs/adr/0012-*`, contrat v1, annexes B1–B6, inventaire deltas API/événements/DTO/schémas/migrations, attribution fichiers partagés C/D + chemins de tests | — (1ʳᵉ au start) | 72/40 | strong |
| Q | Plan QA | `qa-planner` | `.cursor/qa-plans/20261019-apparatus-knative-runtime.md` | A intégré + Accept humain 0012 | 52/25 | standard |
| B | Socle Serving + profil réseau/isolation | lot infra attribué | `ops/deploy/p4/**` + fichiers de pile système (liste close par A) | A (+ oracles Q pour l'acceptation en E) | 70/65 | strong |
| C | Projection par binding, operator | `hard-implementer` (seul écrivain) | `workers/apparatus-operator/**` exclusif | A + Q commité | 78/72 | strong |
| D | Caller/identité/routage Lazaret | `hard-implementer` | Lazaret + contrats consommateurs exactement attribués par A | A + Q commité | 76/78 | strong |
| T | Tests durables | `test-engineer` | chemins de tests attribués par Q + arbitrage lead, jamais l'intérieur des modules prod | Q commité | 70/35 | strong |
| E | Validation/reviews/clôture | lead (pas de worker) | — | B+C+D+T intégrés | 55/50 (indicatif) | non publiée |

## 2. Barrières de publication

```
start ─► A seule publiée (worktree: false, contexte: session architecte existante)
  H1 : A intégré ET Accept humain 0012 + COMMIT lead ADR/contrat ratifiés (GO utilisateur à ce moment)
       ├─► Q (plan QA) ∥ B (rendu statique seulement, zéro Kind/Docker)
  H2 : plan QA COMMITÉ au chemin exact ─► C ∥ D ∥ T (écriture parallèle ; slot Cargo unique sérialisé)
  H3 : B+C+D+T intégrés ─► E : reviews fresh → gates Windows ciblées → IT finale
  H4 : AUTORISATION EXPLICITE campagne E2E (kind-aiforall-local seul)
       ─► rapport, preuves, décision Implemented 0012 (humaine), revue séparée 0009
```

Consommateurs C/D/T hors tableau jusqu'à H2 réel ; E n'est jamais publiée.

## 3. Contrats de contexte (résumé)

- **A** (`worktree: false`) : entrées = ADR-0012/contrat (état prep dans l'arbre sale), ADR 0008 (supersession partielle écrite seulement à l'Accept), 0011 Rejected intouchable, 0009/0010/0600 Proposed, ledger `.cursor/handoffs/20261019-apparatus-knative-preparation.md` ; ancrages code en lecture (`controller.rs`, `desired_state.rs`, Lazaret `invoke.rs`, `identity.rs`). Interdits : toute modification de production, pré-supposer la supersession de 0008 active.
- **Q** : entrées = ADR-0012/contrat **commités** par la lead, 9 scénarios §6, ADR 0200–0202 ; livrable au chemin exact, matrice impact×probabilité distincte des scores de routage, registre obligation→test, critères entrée/sortie ; aucun code, aucun test, aucun runtime.
- **B** (`worktree: true`) : entrées = contrat ratifié (B2/B5) + liste close de fichiers ; manifests/pins Serving + ingress privé, KPA scale-to-zero (`min-scale=0`, `initial-scale=0`, `allow-zero-initial-scale=true`), quotas, NetworkPolicy/probes, sandbox/mTLS ; rendu statique d'abord, `infra-verifier` read-only après changement ; aucun chart publisher privilégié, aucun second proxy/queue/HPA/KEDA, aucun réveil Docker/Kind.
- **C** (`worktree: true`) : exclusif `workers/apparatus-operator/**` ; Service stable par binding, Revisions par génération autorisée, apply idempotent/fencé (préconditions UID), observation ≠ état logique, retrait sûr ; migration digest inventoriée, ancien chemin coupé avant le nouveau, zéro suppression d'enfants Serving ; IT = fixtures testcontainers, jamais Kind ; slot Cargo + registre Docker.
- **D** (`worktree: true`) : Lazaret + contrats attribués par A ; **zéro client/token Kubernetes dans Lazaret/Manifesto** ; autorisation pré-réveil (B1/B4), locator par binding/génération, transport B2, deadlines/erreurs B5 ; périmé/absent = refus, pas d'endpoint par digest, pas de fallback local, pas de retry métier auto sur timeout ambigu ; fixtures wiremock (HTTP sortant).
- **T** (`worktree: true`) : registre obligation→test du plan QA = spécification des tests ; tests intégrés aux modules prod = propriété C/D/B ; bug produit = preuve ROUGE laissée, correction par C/D/B ; preuves de réseau réel, jamais un mock Kubernetes comme preuve de confinement.

## 4. Points de contrôle lead

1. Scores validés (fait) ; publication avec contextes complets et `require_context` déjà actif.
2. H1 : vérifier l'attribution C/D **exhaustive** (y compris chemins de tests) — toute ambiguïté = retour architecte, pas arbitrage code ; puis commit ADR/contrat (GO utilisateur demandé à ce moment).
3. H2 : vérifier le plan QA au chemin exact + couverture des 9 scénarios ; arbitrer la surface de tests de T vs tests intégrés.
4. Reviews proportionnées au diff, contexte fresh, briefings persistés `.cursor/review-briefings/` avant correction : B → `infra-verifier` + sécurité du profil réseau ; C → correctness (fencing/UID/migration/non-destruction) ; D → security (authz pré-réveil, anti-contournement, no-fallback) + correctness ; T → `test-reviewer` ; rust-perf si surface Rust concernée.
5. Gates Windows ciblées (fmt/clippy/test `-p`), jamais « tout cargo » ; un seul cargo à la fois, ordonnancé par la lead.
6. IT : testcontainers uniquement, stop possédé en fin de campagne (succès ou échec), registre `.tmp/docker-resources-<YYYYMMDD>.log`.
7. H4 : E2E sur autorisation explicite, `kind-aiforall-local` seul ; charger `aiforall-test-evidence` + `local-runtime-lifecycle` avant tout run réel ; inventaires avant/après.
8. Handoff d'exécution créé/mis à jour à chaque lot (`.cursor/handoffs/`) ; ledger des tentatives conservé ; transfert `emergency-thinker` après deux tentatives substantielles sans progrès vérifié.

## 5. Décisions ouvertes (humaines/lead)

- Accept humain de 0012 (barrière H1).
- Commit des ADR/contrat ratifiés à H1 — GO utilisateur demandé à ce moment.
- Attribution définitive des fichiers partagés C/D et des chemins de tests — par A, arbitrage lead à H2.
- Autorisation de la campagne E2E (barrière H4).
- `Implemented` de 0012 et revue séparée de 0009 — fin E, décision humaine.
