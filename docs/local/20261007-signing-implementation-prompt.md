# Prompt d'implémentation — Simplification signature IAM (ADR-0311 / 0312)

Session fraîche. Lis AVANT d'agir : mémoire Serena `handoff/2026-10-06-iam-signing-enterprise-rustsec-pause` ; `docs/local/20261006-signing-simplification-inventory.md` ; `docs/local/20261006-iam-it-runtime-contract.md` ; ADR 0304, 0305, 0306, 0308, 0309, 0310, 0311, 0312 ; `.cursor/review-briefings/INDEX.md` + `20261005-session-standards.md`. HEAD de référence : `f5c0ba7` (parent), SDK consommé `4cdf202`.

## Contraintes permanentes

- Interdits sans arbitrage : supprimer un scénario, skip/ignorer un test, affaiblir une assertion, réintroduire un signataire privé local en production, repli silencieux plateforme, chiffrer/promettre des gains de performance, version bump crate.
- Tout est Proposition tant qu'une ADR n'est pas passée à Accept ; ADR-0311/0312 sont Proposed — demander ratification utilisateur au moment du passage en Accept, pas avant l'implémentation du code qui les applique.
- Un seul slot Cargo à la fois, compilation Linux dans Docker, ledger Docker tenu à jour.

## Phase a — Sauvegarde du WIP et déblocage 401 froid

1. Committer proprement le WIP IAM courant (24 fichiers) en l'état validé, sans mélanger avec la suite.
2. Câbler `UserIdExtractor::from_config_with_seeded_jwks` (SDK `4cdf202` déjà publié) dans `setup_jwt` : seed = snapshot canonique frais via `get_jwks()` avant move dans la registry ; URL identique à `jwks_url` configurée ; zéro HTTP au boot. Cible : `user::test_get_user_concurrent_requests_with_same_token`, `internal_provider_token::test_internal_provider_token_concurrent_requests_same_user`.

## Phase b — Fournisseur Transit complet

3. Épingler `key_version` dans l'adaptateur Transit (aujourd'hui absent, `transit.rs:187–198`) : sign sur version précise, lecture public par version, création/rotation = nouvelles versions fournisseur.
4. Orchestration IAM : enrôlement public + preuve de possession → prépublication → activation atomique version épinglée → ancien public conservé (fenêtre overlap existante) → retire/révocation + fence.
5. Clé plateforme sur Transit ; PEM confiné dev/test par garde de configuration explicite (refus hors non-production).

## Phase c — Retrait du chemin privé local (APRÈS b vert)

6. Supprimer keygen local et écritures PEM/PKCS8 (`rotate.rs:197–237`) et la signature RSA en processus (`pem.rs` chemin prod). Adapter rotate/configure/test/disable aux appels provider. Platform ET org.
7. Rotations/churn : comptage par requête indexée/compteur transactionnel ; interdiction de réinitialiser les compteurs à la révocation.

## Phase d — Slots de capacité (ADR-0312)

8. Constantes nommées + test de preuve (enveloppe + slots ≤ 786432 o, réserve plateforme 65536, ≤ borne SDK 1 MiB) ; admission transactionnelle ; snapshot matérialisé sur mutation/expiration + contrôle final de taille avant activation ; suppression des rescans/reparses complets et de la tarification exacte.

## Phase e — Fixtures

9. Matériel RSA statique pour fixtures ordinaires ; frontière = état pré-rempli + dernières admissions + courses réelles ; assertions inchangées ; les scénarios dont le sujet est la crypto gardent leur matériel réel.

## Phase f — Gates de clôture

10. Build exact matrice IAM (7 packages) `--locked --all-targets` ; lanes clippy exactes (production + tests) ; fmt ; tests ciblés visant les invariants ADR (frontière capacité, rotation overlap, fence révocation, isolation org/plateforme, concurrence admission).
11. Reviews : correctness + test + security (+ rust-perf sur les chemins hot admission/JWKS). Mise à jour des digests Serena et des ADR au fil des commits. Vérifier registre Docker et arrêt des fixtures possédées avant toute réponse finale.

## Rollback / risques

- Chaque phase est un (ou deux) commit(s) isolé(s) ; retour arrière = revert du commit de phase, aucune migration destructive.
- Risque principal : divergence provider (capacités versions) → toute incapacité Transit = STOP + escalation, pas de contournement PEM en prod.
- Ne pas toucher aux fichiers hors périmètre (AGENTS.md, configs locales, preuves CI).
