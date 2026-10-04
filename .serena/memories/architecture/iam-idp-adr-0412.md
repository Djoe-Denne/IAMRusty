# ADR-0412 — transaction OAuth navigateur

Statut : Accepted. Réalité : Partial. Canon : docs/adr/0412-oauth-transaction-persistante-liee-navigateur.md ; voir le fichier ADR.

- Cible ratifiée 2026-10-03 ; sources A/B/root transaction PostgreSQL, cookie noncehash, consume atomique et PKCE écrites. Aucun nouveau résultat intégré root/IT/E2E.
- Exception humaine 2026-10-04 : GET/HEAD relink-callback exact github/gitlab configurés sans access JWT, autorisation transaction cookie/state ; START Link/Relink JWT plateforme, aucun wildcard/alias/non-browser.
- Agrégat IamHttpSecurityContext instance-local partagé standalone/tests/monolith ; pré-DB policy/issuer/limiter, post-DB usecases.oauth.clone(), writer unique cleanup.
- Purge toutes expirées bornée SKIP LOCKED, 60s/100 cap1000 ; watch instance-local, supervision/join/drain des handles, pas TTL+60 garanti sous panne.
- Gates : suites IAM oauth_browser_transactions/oauth_cleanup_lifecycle/auth_transactions_postgres réellement exécutées puis navigateur/Envoy/replay sur kind-aiforall-local-full autorisé après IT ; SDK22/review statique ≠ preuve IAM.
- Mise à jour 2026-10-04 — migrations aplaties : aucune donnée en production ; `oauth_transactions` et ses contraintes/index sont dans l'unique `m20220101_000001_initial_schema.rs`. Migrations incrémentales seulement quand un état persisté devra être préservé. Statut Accepted / réalité Partial inchangés ; aucune preuve IT/E2E ajoutée.
