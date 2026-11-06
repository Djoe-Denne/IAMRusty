Jalon : IAM-IdP (hors Apparatus). Canon : `docs/adr/0407-contrat-authn-federee-vendor-neutral.md`. Statut : Accepted. Réalité : Implemented.

- IAM = IdP plateforme (0400) ; Connect = federated authenticators, pas émetteurs JWT.
- Crate `idp-connect-contract` : trait `FederatedOAuthClient` + DTOs ; HTTP JSON `/v1/authorize|token|profile`.
- Feature `server` : middleware Axum HMAC + 3 handlers wrapping `Arc<dyn FederatedOAuthClient>`.
- `OAuthService` reste IAM. Subject = `(provider_id, provider_user_id)`.
- Leftover : enum `Provider` GitHub|GitLab = adapter v1 de route (0407 §8). Pas de Google.
- Voir le fichier ADR.
- Actualisation 2026-11-06 : interface d'authentification partiellement supersédée par ADR0413 Accepted/Partial ; frontière de contrat v2 implémentée et vérifiée, raccord IAM/service/nonce durable encore à faire. Les chemins v1 ci-dessus sont historiques ; claims riches/actions toujours différés. Voir l'annotation canonique dans 0407 et le périmètre de preuve dans 0413.
