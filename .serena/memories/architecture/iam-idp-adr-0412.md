# IAM-IdP — ADR-0412 transaction OAuth navigateur

Statut : Accepted. Réalité : Partial. Canon : docs/adr/0412-oauth-transaction-persistante-liee-navigateur.md ; voir le fichier ADR.

- Cible ratifiée le 2026-10-03 : transaction writer PostgreSQL login/link/relink, cookie noncehash, consommation atomique inter-replicas et PKCE supporté.
- Exception relink ratifiée le 2026-10-04 : GET/HEAD relink-callback exact github/gitlab configurés sans access JWT, autorisation transaction cookie/state ; START Link/Relink JWT plateforme, aucun alias/wildcard/non-browser.
- IamHttpSecurityContext partagé standalone/tests/monolith ; writer unique et purge expirées bornée SKIP LOCKED, watch instance-local supervisé ; pas TTL+60 garanti sous panne/backlog. Schéma OAuth dans la migration IAM initiale unique.
- 2026-10-08 : units + IT IAM vertes sur master, exécutées et confirmées par Djoé Denne ; attestation utilisateur, pas artefact CI archivé ni exécution de cet agent. Suites oauth_browser_transactions/oauth_cleanup_lifecycle/auth_transactions_postgres référencées par le canon.
- E2E finale exact-route/Envoy NON attestée et explicitement restante ; Réalité Partial inchangée jusqu’à cette preuve. SDK22 et review statique ne la remplacent pas ; aucun runtime démarré dans cette réconciliation.
