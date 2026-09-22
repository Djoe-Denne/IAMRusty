Jalon IAM-IdP S3 : arbre `GitHubConnect/` livré (Vague 4 S5 retire l’in-process IAM). Aligne `mem:architecture/iam-idp-adr-0408` Réalité Implemented.

- Forme 0408 : hexagone mince, `SERVICE_PREFIX` `/github-connect`, feature `server`, compose 8085/8446, HMAC PATH préfixé, allowlist `==`, pas FGA/events/DB/nest.
- IAM n’a plus `infra/src/auth/github.rs` ni `idp.mode`. Development : `http://github-connect-service:8080/github-connect` + hmac `change-me-github-connect-hmac`.
- IT IAM : connector WireMock `:3000/github-connect`. Redirect URIs compose IAM :8080 ; tests IAM :8081 (rustycog-testing).
- Voir ADR 0408/0410.