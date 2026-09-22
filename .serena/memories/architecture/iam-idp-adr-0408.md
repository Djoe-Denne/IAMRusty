Jalon : IAM-IdP. Canon : `docs/adr/0408-connecteurs-idp-services-http.md`. Statut : Accepted. Réalité : Implemented.

- v1 = crate contrat + services HTTP `GitHubConnect` / `GitLabConnect` (standalones compose 8085/8446, 8086/8447).
- Registry `[[idp.connectors]]` : slug, URL, HMAC, `redirect_uris[]` (callback et relink-callback).
- Feature `server` du contrat : HTTP S2S partagé. Pas de nest monolith v1.
- Pas OpenFGA / events / DB / bearer user. Hive org-sync hors slice.
- IAM compose interne : HTTP `github-connect-service:8080` / `gitlab-connect-service:8080` (pas le mesh 8446/8447).
- Voir le fichier ADR.