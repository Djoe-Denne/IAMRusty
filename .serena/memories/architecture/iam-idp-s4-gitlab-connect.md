Jalon IAM-IdP S4 : arbre `GitLabConnect/` livré (Vague 4 S5 retire l’in-process IAM). Aligne `mem:architecture/iam-idp-adr-0408` Réalité Implemented.

- Forme 0408 : hexagone mince, `SERVICE_PREFIX` `/gitlab-connect`, feature `server`, compose 8086/8447, HMAC PATH préfixé, allowlist `==` URIs IAM `/iam/api/auth/gitlab/*`.
- IAM n’a plus `infra/src/auth/gitlab.rs` ni `GitLabOAuth2Client`. Development : `http://gitlab-connect-service:8080/gitlab-connect` + hmac `change-me-gitlab-connect-hmac`.
- IT IAM : connector WireMock `:3000/gitlab-connect`. `docs/services/gitlab-connect.md` existe. Pas de nest monolith.
- Voir ADR 0408/0410 et `mem:architecture/iam-idp-s3-github-connect`.