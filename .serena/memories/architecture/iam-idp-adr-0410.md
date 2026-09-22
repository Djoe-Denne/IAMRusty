Jalon : IAM-IdP. Canon : `docs/adr/0410-migration-iam-connecteurs-idp.md`. Statut : Accepted. Réalité : Implemented.

- S1–S6 tenus : contrat, adapter HTTP IAM, GitHubConnect, GitLabConnect, retrait in-process, docs/wiki.
- IAM boot HTTP-only : `setup_http_idp_clients` ; fail-closed si connectors vides ou hmac < 16 octets.
- Pas de `client_secret` vendor dans les TOML IAM. Trait `ProviderOAuth2Client` retiré (port = `FederatedOAuthClient`).
- IT : IAM WireMock connecteur ; connecteur WireMock vendor. `oodhive-monolith` non nesté. Pas de Google.
- Leftover : enum `Provider` GH/GL adapter v1 (0407 §8).
- Voir le fichier ADR.