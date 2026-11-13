# Collecte coverage sentinel-sync

- Skill : ci-cd ; mode : amélioration ciblée ; date : 2026-11-13.
- Périmètre : matrice `coverage-unit` de `.github/workflows/ci.yml`, collecte `lcov-*` et merge Sonar existants. Aucun changement de permissions, secrets, actions, exclusions ou déploiement.

## P2 — Collecte manquante

- [x] **#1 Tests unitaires sentinel-sync absents de la matrice coverage.**
  - Preuve : baseline CI 43293107226, pas d'artefact `lcov-unit-sentinel-sync` ; matrice initiale IAM/Telegraph/Hive/Manifesto/readiness seulement. Les tests du worker sont présents dans ses modules `src`.
  - Fix : entrée Sentinel Sync, `--package sentinel-sync`, artefact `lcov-unit-sentinel-sync`, fichier `.info` dédié ; réutilisation du merge `lcov-*`.
  - Vérification : parse YAML et assertions matrice/identité unique/dépendance/pattern PASS ; 50 tests lib + 0 bin PASS Windows instrumenté ; exécution distante après publication à vérifier.

Total : 1 corrigé, 0 finding ouvert. Limite : ce contrôle ciblé n'est pas un audit global de CI ; il ne certifie pas les anciennes décisions hors diff.
